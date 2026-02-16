use std::collections::HashMap;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::time::timeout;

use crate::hmu_types::*;

/// Configuration for the HMU engine.
#[derive(Debug, Clone)]
pub struct HmuConfig {
    /// Timeout waiting for ACK (default: 5 seconds).
    pub ack_timeout: Duration,
    /// Max consecutive failures before requiring restart (default: 3).
    pub max_consecutive_failures: u32,
}

impl Default for HmuConfig {
    fn default() -> Self {
        Self {
            ack_timeout: Duration::from_secs(5),
            max_consecutive_failures: 3,
        }
    }
}

/// Result of an HMU operation.
#[derive(Debug, Clone)]
pub struct HmuResult {
    pub ack: HmuAck,
    pub requires_restart: bool,
}

/// Errors that can occur during HMU operations.
#[derive(Debug, thiserror::Error)]
pub enum HmuError {
    #[error("HMU timeout: no ACK received within {0:?}")]
    Timeout(Duration),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Server requires restart")]
    RequiresRestart,
}

/// The HMU Engine manages hot module updates to a running server process.
///
/// Communication happens via stdin/stdout JSON messages (newline-delimited IPC).
pub struct HmuEngine {
    config: HmuConfig,
    next_id: u64,
    failure_counts: HashMap<String, u32>,
}

impl HmuEngine {
    pub fn new(config: HmuConfig) -> Self {
        Self {
            config,
            next_id: 1,
            failure_counts: HashMap::new(),
        }
    }

    /// Compute a SHA-256 hash of the given content.
    /// Returns the hash in `"sha256:{hex}"` format.
    pub fn compute_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        format!("sha256:{:x}", result)
    }

    /// Create an `HmuUpdate` from a list of `(path, action, content)` tuples.
    ///
    /// Generates sequential IDs like `"hmu_001"`, `"hmu_002"`, etc.
    pub fn create_update(
        &mut self,
        modules: Vec<(String, HmuAction, String)>,
    ) -> HmuUpdate {
        let id = format!("hmu_{:03}", self.next_id);
        self.next_id += 1;

        let hmu_modules = modules
            .into_iter()
            .map(|(path, action, content)| {
                let hash = Self::compute_hash(&content);
                HmuModule {
                    path,
                    action,
                    content,
                    hash,
                }
            })
            .collect();

        HmuUpdate {
            kind: HmuUpdateType::HmuUpdate,
            id,
            timestamp: chrono::Utc::now(),
            modules: hmu_modules,
        }
    }

    /// Send an HMU update through the child process's stdin and wait for an ACK
    /// on stdout.
    ///
    /// Returns `HmuResult` on success, or `HmuError` on timeout / IO / parse
    /// failure.
    pub async fn send_update<W, R>(
        &mut self,
        stdin: &mut W,
        stdout: &mut R,
        update: &HmuUpdate,
    ) -> Result<HmuResult, HmuError>
    where
        W: AsyncWriteExt + Unpin,
        R: AsyncBufReadExt + Unpin,
    {
        // Serialize and write
        let mut json = serde_json::to_string(update)?;
        json.push('\n');
        stdin.write_all(json.as_bytes()).await?;
        stdin.flush().await?;

        // Read ACK with timeout
        let mut line = String::new();
        let read_result = timeout(self.config.ack_timeout, stdout.read_line(&mut line)).await;

        match read_result {
            Ok(Ok(0)) => {
                return Err(HmuError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "child process closed stdout",
                )));
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(HmuError::Io(e)),
            Err(_) => return Err(HmuError::Timeout(self.config.ack_timeout)),
        }

        let ack: HmuAck = serde_json::from_str(line.trim())?;

        // Update failure counts
        for path in &ack.applied {
            self.reset_failure_count(path);
        }
        for path in &ack.failed {
            let count = self.failure_counts.entry(path.clone()).or_insert(0);
            *count += 1;
        }

        let requires_restart =
            ack.requires_restart || self.check_escalation(&ack.failed);

        Ok(HmuResult {
            ack,
            requires_restart,
        })
    }

    /// Reset the failure count for a module path (e.g. after a successful update).
    pub fn reset_failure_count(&mut self, path: &str) {
        self.failure_counts.remove(path);
    }

    /// Check if any of the given failed paths has exceeded `max_consecutive_failures`.
    pub fn check_escalation(&self, failed_paths: &[String]) -> bool {
        failed_paths.iter().any(|path| {
            self.failure_counts
                .get(path)
                .map_or(false, |&count| count >= self.config.max_consecutive_failures)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tokio::io::BufReader;

    // ── compute_hash ──────────────────────────────────────────────

    #[test]
    fn compute_hash_produces_consistent_sha256() {
        let hash1 = HmuEngine::compute_hash("hello world");
        let hash2 = HmuEngine::compute_hash("hello world");
        assert_eq!(hash1, hash2);
        assert!(hash1.starts_with("sha256:"));
    }

    #[test]
    fn compute_hash_differs_for_different_content() {
        let hash1 = HmuEngine::compute_hash("hello");
        let hash2 = HmuEngine::compute_hash("world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn compute_hash_known_value() {
        // SHA-256 of "test" is well-known
        let hash = HmuEngine::compute_hash("test");
        assert_eq!(
            hash,
            "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    // ── create_update ─────────────────────────────────────────────

    #[test]
    fn create_update_generates_sequential_ids() {
        let mut engine = HmuEngine::new(HmuConfig::default());

        let u1 = engine.create_update(vec![(
            "a.ts".into(),
            HmuAction::Replace,
            "content a".into(),
        )]);
        let u2 = engine.create_update(vec![(
            "b.ts".into(),
            HmuAction::Add,
            "content b".into(),
        )]);

        assert_eq!(u1.id, "hmu_001");
        assert_eq!(u2.id, "hmu_002");
    }

    #[test]
    fn create_update_computes_hashes() {
        let mut engine = HmuEngine::new(HmuConfig::default());

        let update = engine.create_update(vec![
            ("a.ts".into(), HmuAction::Replace, "content a".into()),
            ("b.ts".into(), HmuAction::Add, "content b".into()),
        ]);

        assert_eq!(update.modules.len(), 2);
        assert_eq!(
            update.modules[0].hash,
            HmuEngine::compute_hash("content a")
        );
        assert_eq!(
            update.modules[1].hash,
            HmuEngine::compute_hash("content b")
        );
    }

    #[test]
    fn create_update_sets_correct_fields() {
        let mut engine = HmuEngine::new(HmuConfig::default());

        let update = engine.create_update(vec![(
            "src/index.ts".into(),
            HmuAction::Remove,
            "".into(),
        )]);

        assert_eq!(update.kind, HmuUpdateType::HmuUpdate);
        assert_eq!(update.modules[0].path, "src/index.ts");
        assert_eq!(update.modules[0].action, HmuAction::Remove);
    }

    // ── HmuConfig defaults ────────────────────────────────────────

    #[test]
    fn config_defaults() {
        let config = HmuConfig::default();
        assert_eq!(config.ack_timeout, Duration::from_secs(5));
        assert_eq!(config.max_consecutive_failures, 3);
    }

    // ── send_update ───────────────────────────────────────────────

    /// Helper: create a duplex pair simulating child process stdin/stdout.
    /// Returns (engine_stdin_writer, engine_stdout_reader, server_stdin_reader, server_stdout_writer).
    fn make_ipc_pair() -> (
        tokio::io::DuplexStream,
        BufReader<tokio::io::DuplexStream>,
        BufReader<tokio::io::DuplexStream>,
        tokio::io::DuplexStream,
    ) {
        // stdin channel: engine writes → server reads
        let (server_stdin_reader, engine_stdin_writer) = tokio::io::duplex(4096);
        // stdout channel: server writes → engine reads
        let (engine_stdout_reader, server_stdout_writer) = tokio::io::duplex(4096);

        (
            engine_stdin_writer,
            BufReader::new(engine_stdout_reader),
            BufReader::new(server_stdin_reader),
            server_stdout_writer,
        )
    }

    #[tokio::test]
    async fn send_update_success() {
        let mut engine = HmuEngine::new(HmuConfig::default());
        let update = engine.create_update(vec![(
            "a.ts".into(),
            HmuAction::Replace,
            "new code".into(),
        )]);

        let ack_json = serde_json::json!({
            "type": "HMU_ACK",
            "id": "hmu_001",
            "status": "success",
            "applied": ["a.ts"],
            "failed": [],
            "requiresRestart": false
        });
        let ack_line = format!("{}\n", ack_json);

        let (mut stdin_w, mut stdout_r, mut srv_stdin_r, mut srv_stdout_w) = make_ipc_pair();

        // Simulate server: read update from stdin, write ACK to stdout
        tokio::spawn(async move {
            let mut line = String::new();
            srv_stdin_r.read_line(&mut line).await.unwrap();
            srv_stdout_w.write_all(ack_line.as_bytes()).await.unwrap();
            srv_stdout_w.flush().await.unwrap();
        });

        let result = engine
            .send_update(&mut stdin_w, &mut stdout_r, &update)
            .await
            .unwrap();

        assert_eq!(result.ack.status, HmuStatus::Success);
        assert_eq!(result.ack.applied, vec!["a.ts"]);
        assert!(!result.requires_restart);
    }

    #[tokio::test]
    async fn send_update_timeout() {
        let config = HmuConfig {
            ack_timeout: Duration::from_millis(50),
            ..Default::default()
        };
        let mut engine = HmuEngine::new(config);
        let update = engine.create_update(vec![(
            "a.ts".into(),
            HmuAction::Replace,
            "code".into(),
        )]);

        let (mut stdin_w, mut stdout_r, _srv_stdin_r, srv_stdout_w) = make_ipc_pair();

        // Server never responds — keep writer alive to prevent EOF
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            drop(srv_stdout_w);
        });

        let result = engine
            .send_update(&mut stdin_w, &mut stdout_r, &update)
            .await;

        assert!(matches!(result, Err(HmuError::Timeout(_))));
    }

    #[tokio::test]
    async fn send_update_partial_failure_tracks_counts() {
        let mut engine = HmuEngine::new(HmuConfig::default());

        let update = engine.create_update(vec![
            ("a.ts".into(), HmuAction::Replace, "code a".into()),
            ("b.ts".into(), HmuAction::Replace, "code b".into()),
        ]);

        let ack_json = serde_json::json!({
            "type": "HMU_ACK",
            "id": "hmu_001",
            "status": "partial",
            "applied": ["a.ts"],
            "failed": ["b.ts"],
            "requiresRestart": false
        });
        let ack_line = format!("{}\n", ack_json);

        let (mut stdin_w, mut stdout_r, mut srv_stdin_r, mut srv_stdout_w) = make_ipc_pair();

        tokio::spawn(async move {
            let mut line = String::new();
            srv_stdin_r.read_line(&mut line).await.unwrap();
            srv_stdout_w.write_all(ack_line.as_bytes()).await.unwrap();
            srv_stdout_w.flush().await.unwrap();
        });

        let result = engine
            .send_update(&mut stdin_w, &mut stdout_r, &update)
            .await
            .unwrap();

        assert_eq!(result.ack.status, HmuStatus::Partial);
        assert_eq!(*engine.failure_counts.get("b.ts").unwrap(), 1);
        assert!(!engine.failure_counts.contains_key("a.ts"));
        assert!(!result.requires_restart);
    }

    // ── failure escalation ────────────────────────────────────────

    #[test]
    fn check_escalation_below_threshold() {
        let mut engine = HmuEngine::new(HmuConfig::default());
        engine.failure_counts.insert("a.ts".into(), 2);

        assert!(!engine.check_escalation(&["a.ts".into()]));
    }

    #[test]
    fn check_escalation_at_threshold() {
        let mut engine = HmuEngine::new(HmuConfig::default());
        engine.failure_counts.insert("a.ts".into(), 3);

        assert!(engine.check_escalation(&["a.ts".into()]));
    }

    #[test]
    fn check_escalation_above_threshold() {
        let mut engine = HmuEngine::new(HmuConfig::default());
        engine.failure_counts.insert("a.ts".into(), 5);

        assert!(engine.check_escalation(&["a.ts".into()]));
    }

    #[test]
    fn check_escalation_unknown_path() {
        let engine = HmuEngine::new(HmuConfig::default());
        assert!(!engine.check_escalation(&["unknown.ts".into()]));
    }

    // ── reset_failure_count ───────────────────────────────────────

    #[test]
    fn reset_failure_count_removes_entry() {
        let mut engine = HmuEngine::new(HmuConfig::default());
        engine.failure_counts.insert("a.ts".into(), 2);

        engine.reset_failure_count("a.ts");
        assert!(!engine.failure_counts.contains_key("a.ts"));
    }

    #[test]
    fn reset_failure_count_noop_for_unknown() {
        let mut engine = HmuEngine::new(HmuConfig::default());
        engine.reset_failure_count("unknown.ts"); // should not panic
    }

    // ── escalation via send_update ────────────────────────────────

    #[tokio::test]
    async fn send_update_escalates_after_max_failures() {
        let config = HmuConfig {
            max_consecutive_failures: 2,
            ..Default::default()
        };
        let mut engine = HmuEngine::new(config);

        // Simulate first failure — count becomes 1
        engine.failure_counts.insert("x.ts".into(), 1);

        let update = engine.create_update(vec![(
            "x.ts".into(),
            HmuAction::Replace,
            "code".into(),
        )]);

        let ack_json = serde_json::json!({
            "type": "HMU_ACK",
            "id": "hmu_001",
            "status": "failed",
            "applied": [],
            "failed": ["x.ts"],
            "requiresRestart": false
        });
        let ack_line = format!("{}\n", ack_json);

        let (mut stdin_w, mut stdout_r, mut srv_stdin_r, mut srv_stdout_w) = make_ipc_pair();

        tokio::spawn(async move {
            let mut line = String::new();
            srv_stdin_r.read_line(&mut line).await.unwrap();
            srv_stdout_w.write_all(ack_line.as_bytes()).await.unwrap();
            srv_stdout_w.flush().await.unwrap();
        });

        let result = engine
            .send_update(&mut stdin_w, &mut stdout_r, &update)
            .await
            .unwrap();

        // count is now 2 == max_consecutive_failures → escalation
        assert_eq!(*engine.failure_counts.get("x.ts").unwrap(), 2);
        assert!(result.requires_restart);
    }
}
