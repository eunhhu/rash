use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, watch};

use rash_spec::types::common::{Framework, Language, Runtime};

use crate::log_types::{LogEntry, LogLevel, LogSource};

/// Error types for process management operations.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("process already running (pid {0})")]
    AlreadyRunning(u32),

    #[error("no process running")]
    NotRunning,

    #[error("failed to start process: {0}")]
    StartFailed(String),

    #[error("port detection timed out after {0} seconds")]
    PortTimeout(u64),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ProcessError>;

/// Status of a managed server process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Errored,
}

/// Configuration for starting a server process.
pub struct ServerConfig {
    pub language: Language,
    pub framework: Framework,
    pub runtime: Runtime,
    pub port: u16,
    pub host: String,
    pub output_dir: PathBuf,
    pub env_vars: HashMap<String, String>,
}

struct RunningProcess {
    child: tokio::process::Child,
    pid: u32,
    port: u16,
    started_at: DateTime<Utc>,
}

/// Manages the lifecycle of a child server process.
///
/// Provides start/stop/restart with stdout/stderr log streaming
/// and automatic port detection from process output.
pub struct ProcessManager {
    process: Option<RunningProcess>,
    status: ServerStatus,
    log_tx: mpsc::UnboundedSender<LogEntry>,
    status_tx: watch::Sender<ServerStatus>,
}

const PORT_DETECT_TIMEOUT_SECS: u64 = 10;
const GRACEFUL_SHUTDOWN_SECS: u64 = 3;

impl ProcessManager {
    /// Create a new ProcessManager and return the log/status receivers.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<LogEntry>, watch::Receiver<ServerStatus>) {
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = watch::channel(ServerStatus::Stopped);

        let mgr = Self {
            process: None,
            status: ServerStatus::Stopped,
            log_tx,
            status_tx,
        };

        (mgr, log_rx, status_rx)
    }

    /// Start a server process with the given configuration.
    /// Returns the detected port on success.
    pub async fn start(&mut self, config: &ServerConfig) -> Result<u16> {
        if let Some(ref proc) = self.process {
            return Err(ProcessError::AlreadyRunning(proc.pid));
        }

        self.set_status(ServerStatus::Starting);

        let (cmd, args, cwd) = Self::resolve_command(config);

        let mut command = tokio::process::Command::new(&cmd);
        command
            .args(&args)
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .envs(&config.env_vars)
            .env("PORT", config.port.to_string())
            .env("HOST", &config.host);

        let mut child = command.spawn().map_err(|e| {
            self.set_status(ServerStatus::Errored);
            ProcessError::StartFailed(format!("{cmd}: {e}"))
        })?;

        let pid = child.id().ok_or_else(|| {
            self.set_status(ServerStatus::Errored);
            ProcessError::StartFailed("failed to get child pid".into())
        })?;

        // Take stdout/stderr handles for streaming
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let expected_port = config.port;

        // Channel to detect the port from stdout
        let (port_tx, port_rx) = tokio::sync::oneshot::channel::<u16>();

        // Spawn stdout reader
        if let Some(stdout) = stdout {
            let log_tx = self.log_tx.clone();
            let port_tx = Some(port_tx);
            tokio::spawn(stream_output(
                stdout,
                LogSource::Stdout,
                LogLevel::Info,
                log_tx,
                port_tx,
                expected_port,
            ));
        }

        // Spawn stderr reader
        if let Some(stderr) = stderr {
            let log_tx = self.log_tx.clone();
            tokio::spawn(stream_output(
                stderr,
                LogSource::Stderr,
                LogLevel::Error,
                log_tx,
                None,
                expected_port,
            ));
        }

        // Wait for port detection or timeout
        let detected_port = tokio::time::timeout(
            std::time::Duration::from_secs(PORT_DETECT_TIMEOUT_SECS),
            port_rx,
        )
        .await;

        let port = match detected_port {
            Ok(Ok(p)) => p,
            _ => {
                // Timeout or channel closed - assume the configured port
                config.port
            }
        };

        let now = Utc::now();
        self.process = Some(RunningProcess {
            child,
            pid,
            port,
            started_at: now,
        });
        self.set_status(ServerStatus::Running);

        Ok(port)
    }

    /// Stop the running process gracefully.
    /// Sends kill signal, waits up to 3 seconds, then force kills.
    pub async fn stop(&mut self) -> Result<()> {
        let mut proc = self.process.take().ok_or(ProcessError::NotRunning)?;

        self.set_status(ServerStatus::Stopping);

        // Try graceful shutdown: kill and wait with timeout
        let shutdown = tokio::time::timeout(
            std::time::Duration::from_secs(GRACEFUL_SHUTDOWN_SECS),
            proc.child.kill(),
        )
        .await;

        match shutdown {
            Ok(Ok(())) => {
                // Wait for the child to fully exit
                let _ = proc.child.wait().await;
            }
            _ => {
                // Force kill if timeout or error
                let _ = proc.child.kill().await;
                let _ = proc.child.wait().await;
            }
        }

        self.set_status(ServerStatus::Stopped);
        Ok(())
    }

    /// Restart the server process: stop (if running) then start.
    pub async fn restart(&mut self, config: &ServerConfig) -> Result<u16> {
        if self.process.is_some() {
            self.stop().await?;
        }
        self.start(config).await
    }

    /// Get current server status.
    pub fn status(&self) -> ServerStatus {
        self.status
    }

    /// Get the PID of the running process, if any.
    pub fn pid(&self) -> Option<u32> {
        self.process.as_ref().map(|p| p.pid)
    }

    /// Get the port of the running process, if any.
    pub fn port(&self) -> Option<u16> {
        self.process.as_ref().map(|p| p.port)
    }

    /// Get the start time of the running process, if any.
    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.process.as_ref().map(|p| p.started_at)
    }

    /// Determine the command, arguments, and working directory for a given config.
    pub fn resolve_command(config: &ServerConfig) -> (String, Vec<String>, PathBuf) {
        let dir = config.output_dir.clone();

        match (config.language, config.runtime) {
            (Language::Typescript, Runtime::Bun) => {
                ("bun".into(), vec!["run".into(), "src/index.ts".into()], dir)
            }
            (Language::Typescript, Runtime::Node) => (
                "node".into(),
                vec![
                    "--loader".into(),
                    "ts-node/esm".into(),
                    "src/index.ts".into(),
                ],
                dir,
            ),
            (Language::Typescript, Runtime::Deno) => (
                "deno".into(),
                vec!["run".into(), "--allow-net".into(), "src/index.ts".into()],
                dir,
            ),
            (Language::Rust, Runtime::Cargo) => {
                ("cargo".into(), vec!["run".into()], dir)
            }
            (Language::Python, Runtime::Python) => (
                "python".into(),
                vec![
                    "-m".into(),
                    "uvicorn".into(),
                    "main:app".into(),
                    "--host".into(),
                    config.host.clone(),
                    "--port".into(),
                    config.port.to_string(),
                ],
                dir,
            ),
            (Language::Go, Runtime::Go) => {
                ("go".into(), vec!["run".into(), ".".into()], dir)
            }
            // Fallback for unexpected combinations
            _ => ("echo".into(), vec!["unsupported runtime".into()], dir),
        }
    }

    fn set_status(&mut self, status: ServerStatus) {
        self.status = status;
        let _ = self.status_tx.send(status);
    }
}

/// Stream lines from an async reader, sending LogEntry messages and optionally detecting port.
async fn stream_output<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    source: LogSource,
    level: LogLevel,
    log_tx: mpsc::UnboundedSender<LogEntry>,
    mut port_tx: Option<tokio::sync::oneshot::Sender<u16>>,
    expected_port: u16,
) {
    let buf = BufReader::new(reader);
    let mut lines = buf.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        // Try to detect port from line
        if port_tx.is_some() {
            if let Some(port) = detect_port(&line, expected_port) {
                if let Some(tx) = port_tx.take() {
                    let _ = tx.send(port);
                }
            }
        }

        let entry = LogEntry {
            timestamp: Utc::now(),
            level,
            message: line,
            source,
        };

        if log_tx.send(entry).is_err() {
            break; // Receiver dropped
        }
    }
}

/// Try to detect a port number from a log line.
///
/// Looks for common patterns like:
/// - "listening on port 3000"
/// - "Server running on port 3000"
/// - ":3000"
/// - "0.0.0.0:3000"
/// - "localhost:3000"
fn detect_port(line: &str, expected_port: u16) -> Option<u16> {
    let lower = line.to_lowercase();

    // Pattern: "port XXXX" or "port: XXXX"
    if let Some(idx) = lower.find("port") {
        let after = &line[idx + 4..];
        if let Some(port) = extract_port_number(after) {
            return Some(port);
        }
    }

    // Pattern: ":XXXX" (host:port)
    for part in line.split_whitespace() {
        if let Some(colon_idx) = part.rfind(':') {
            let port_str = &part[colon_idx + 1..].trim_end_matches(|c: char| !c.is_ascii_digit());
            if let Ok(port) = port_str.parse::<u16>() {
                if port >= 1024 {
                    return Some(port);
                }
            }
        }
    }

    // If line contains the expected port as a standalone number
    let port_str = expected_port.to_string();
    if lower.contains("listen") || lower.contains("start") || lower.contains("running") {
        if line.contains(&port_str) {
            return Some(expected_port);
        }
    }

    None
}

/// Extract a port number from text following "port".
fn extract_port_number(text: &str) -> Option<u16> {
    let trimmed = text.trim_start_matches(|c: char| !c.is_ascii_digit());
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u16>().ok().filter(|&p| p >= 1024)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // --- resolve_command tests ---

    fn make_config(language: Language, runtime: Runtime) -> ServerConfig {
        ServerConfig {
            language,
            framework: Framework::Express,
            runtime,
            port: 3000,
            host: "0.0.0.0".into(),
            output_dir: PathBuf::from("/tmp/rash-out"),
            env_vars: HashMap::new(),
        }
    }

    #[test]
    fn resolve_command_typescript_bun() {
        let config = make_config(Language::Typescript, Runtime::Bun);
        let (cmd, args, dir) = ProcessManager::resolve_command(&config);
        assert_eq!(cmd, "bun");
        assert_eq!(args, vec!["run", "src/index.ts"]);
        assert_eq!(dir, PathBuf::from("/tmp/rash-out"));
    }

    #[test]
    fn resolve_command_typescript_node() {
        let config = make_config(Language::Typescript, Runtime::Node);
        let (cmd, args, dir) = ProcessManager::resolve_command(&config);
        assert_eq!(cmd, "node");
        assert_eq!(args, vec!["--loader", "ts-node/esm", "src/index.ts"]);
        assert_eq!(dir, PathBuf::from("/tmp/rash-out"));
    }

    #[test]
    fn resolve_command_typescript_deno() {
        let config = make_config(Language::Typescript, Runtime::Deno);
        let (cmd, args, dir) = ProcessManager::resolve_command(&config);
        assert_eq!(cmd, "deno");
        assert_eq!(args, vec!["run", "--allow-net", "src/index.ts"]);
        assert_eq!(dir, PathBuf::from("/tmp/rash-out"));
    }

    #[test]
    fn resolve_command_rust_cargo() {
        let config = make_config(Language::Rust, Runtime::Cargo);
        let (cmd, args, _) = ProcessManager::resolve_command(&config);
        assert_eq!(cmd, "cargo");
        assert_eq!(args, vec!["run"]);
    }

    #[test]
    fn resolve_command_python() {
        let config = make_config(Language::Python, Runtime::Python);
        let (cmd, args, _) = ProcessManager::resolve_command(&config);
        assert_eq!(cmd, "python");
        assert_eq!(
            args,
            vec!["-m", "uvicorn", "main:app", "--host", "0.0.0.0", "--port", "3000"]
        );
    }

    #[test]
    fn resolve_command_go() {
        let config = make_config(Language::Go, Runtime::Go);
        let (cmd, args, _) = ProcessManager::resolve_command(&config);
        assert_eq!(cmd, "go");
        assert_eq!(args, vec!["run", "."]);
    }

    #[test]
    fn resolve_command_unsupported_combination() {
        let config = make_config(Language::Rust, Runtime::Bun);
        let (cmd, _, _) = ProcessManager::resolve_command(&config);
        assert_eq!(cmd, "echo");
    }

    // --- ServerStatus serialization tests ---

    #[test]
    fn server_status_serialization() {
        assert_eq!(serde_json::to_value(ServerStatus::Stopped).unwrap(), "stopped");
        assert_eq!(serde_json::to_value(ServerStatus::Starting).unwrap(), "starting");
        assert_eq!(serde_json::to_value(ServerStatus::Running).unwrap(), "running");
        assert_eq!(serde_json::to_value(ServerStatus::Stopping).unwrap(), "stopping");
        assert_eq!(serde_json::to_value(ServerStatus::Errored).unwrap(), "errored");
    }

    #[test]
    fn server_status_roundtrip() {
        let statuses = vec![
            ServerStatus::Stopped,
            ServerStatus::Starting,
            ServerStatus::Running,
            ServerStatus::Stopping,
            ServerStatus::Errored,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: ServerStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    // --- Port detection tests ---

    #[test]
    fn detect_port_from_listening_message() {
        assert_eq!(detect_port("Listening on port 3000", 3000), Some(3000));
        assert_eq!(detect_port("Server running on port 8080", 8080), Some(8080));
    }

    #[test]
    fn detect_port_from_host_port() {
        assert_eq!(detect_port("Server started at http://0.0.0.0:3000", 3000), Some(3000));
        assert_eq!(detect_port("listening on http://localhost:8080", 8080), Some(8080));
    }

    #[test]
    fn detect_port_no_match() {
        assert_eq!(detect_port("Loading configuration...", 3000), None);
        assert_eq!(detect_port("Connected to database", 3000), None);
    }

    #[test]
    fn detect_port_ignores_low_ports() {
        // Port numbers below 1024 are filtered out
        assert_eq!(detect_port("port 80", 80), None);
    }

    // --- ProcessManager unit tests ---

    #[test]
    fn process_manager_initial_state() {
        let (mgr, _log_rx, _status_rx) = ProcessManager::new();
        assert_eq!(mgr.status(), ServerStatus::Stopped);
        assert_eq!(mgr.pid(), None);
        assert_eq!(mgr.port(), None);
        assert_eq!(mgr.started_at(), None);
    }

    #[test]
    fn status_channel_receives_initial_value() {
        let (_mgr, _log_rx, status_rx) = ProcessManager::new();
        assert_eq!(*status_rx.borrow(), ServerStatus::Stopped);
    }
}
