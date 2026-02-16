use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// A runtime detected on the host system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedRuntime {
    pub name: String,
    pub version: String,
    pub path: Option<PathBuf>,
}

/// Known runtime names that we can detect.
const KNOWN_RUNTIMES: &[&str] = &["bun", "node", "python", "cargo", "go", "deno"];

pub struct RuntimeDetector;

impl RuntimeDetector {
    /// Detect all installed runtimes on the system.
    pub fn detect_installed() -> Vec<DetectedRuntime> {
        KNOWN_RUNTIMES
            .iter()
            .filter_map(|name| Self::check_runtime(name))
            .collect()
    }

    /// Check if a specific runtime is available.
    pub fn check_runtime(name: &str) -> Option<DetectedRuntime> {
        let (cmd, args) = Self::runtime_command(name)?;
        let version = Self::run_version_command(cmd, args, name)?;
        let path = which::which(cmd).ok().map(PathBuf::from);

        Some(DetectedRuntime {
            name: name.to_string(),
            version,
            path,
        })
    }

    /// Get the command and version arguments for a runtime name.
    fn runtime_command(name: &str) -> Option<(&'static str, &'static [&'static str])> {
        match name {
            "bun" => Some(("bun", &["--version"])),
            "node" => Some(("node", &["--version"])),
            "python" => Some(("python3", &["--version"])),
            "cargo" => Some(("cargo", &["--version"])),
            "go" => Some(("go", &["version"])),
            "deno" => Some(("deno", &["--version"])),
            _ => None,
        }
    }

    /// Execute the version command and parse the output.
    fn run_version_command(cmd: &str, args: &[&str], name: &str) -> Option<String> {
        let output = Command::new(cmd).args(args).output();

        // Fallback: python3 not found → try python
        if output.is_err() && name == "python" {
            return Command::new("python")
                .args(["--version"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        let raw = String::from_utf8_lossy(&o.stdout).to_string();
                        Some(parse_version(&raw, "python"))
                    } else {
                        None
                    }
                });
        }

        let output = output.ok()?;
        if !output.status.success() {
            return None;
        }

        // Some commands (e.g. python --version) write to stderr on older versions.
        let raw = {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                String::from_utf8_lossy(&output.stderr).to_string()
            } else {
                stdout.to_string()
            }
        };

        Some(parse_version(&raw, name))
    }
}

/// Parse a raw version string for the given runtime.
///
/// Handles formats like:
/// - `1.1.38\n` (bun)
/// - `v22.11.0\n` (node)
/// - `Python 3.12.0\n` (python)
/// - `cargo 1.82.0 (8f40fc59f 2024-08-21)\n` (cargo)
/// - `go version go1.23.0 darwin/arm64\n` (go)
/// - `deno 2.0.0\n` (first line of multi-line output)
pub fn parse_version(raw: &str, runtime: &str) -> String {
    let line = raw.lines().next().unwrap_or("").trim();

    match runtime {
        "go" => {
            // "go version go1.23.0 darwin/arm64" → "1.23.0"
            line.split_whitespace()
                .find(|s| s.starts_with("go") && s.len() > 2 && s.as_bytes()[2].is_ascii_digit())
                .map(|s| s.trim_start_matches("go"))
                .unwrap_or(line)
                .to_string()
        }
        "cargo" => {
            // "cargo 1.82.0 (8f40fc59f 2024-08-21)" → "1.82.0"
            line.split_whitespace()
                .nth(1)
                .unwrap_or(line)
                .to_string()
        }
        "python" => {
            // "Python 3.12.0" → "3.12.0"
            line.strip_prefix("Python ")
                .unwrap_or(line)
                .trim_start_matches('v')
                .to_string()
        }
        "deno" => {
            // "deno 2.0.0" → "2.0.0"
            line.strip_prefix("deno ")
                .unwrap_or(line)
                .trim_start_matches('v')
                .to_string()
        }
        _ => {
            // Generic: strip leading 'v' if present
            line.trim_start_matches('v').to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Version parsing tests ──────────────────────────────────────

    #[test]
    fn parse_bun_version() {
        assert_eq!(parse_version("1.1.38\n", "bun"), "1.1.38");
    }

    #[test]
    fn parse_node_version() {
        assert_eq!(parse_version("v22.11.0\n", "node"), "22.11.0");
    }

    #[test]
    fn parse_python_version() {
        assert_eq!(parse_version("Python 3.12.0\n", "python"), "3.12.0");
    }

    #[test]
    fn parse_cargo_version() {
        assert_eq!(
            parse_version("cargo 1.82.0 (8f40fc59f 2024-08-21)\n", "cargo"),
            "1.82.0"
        );
    }

    #[test]
    fn parse_go_version() {
        assert_eq!(
            parse_version("go version go1.23.0 darwin/arm64\n", "go"),
            "1.23.0"
        );
    }

    #[test]
    fn parse_deno_version() {
        // deno --version outputs multiple lines; we only care about the first
        let raw = "deno 2.0.0\nv8 12.9.202.13\ntypescript 5.6.2\n";
        assert_eq!(parse_version(raw, "deno"), "2.0.0");
    }

    #[test]
    fn parse_empty_string() {
        assert_eq!(parse_version("", "node"), "");
    }

    #[test]
    fn parse_unknown_runtime_strips_v() {
        assert_eq!(parse_version("v1.0.0\n", "unknown"), "1.0.0");
    }

    // ── RuntimeDetector tests ──────────────────────────────────────

    #[test]
    fn detect_installed_returns_results() {
        // We're in a Rust project, so cargo should always be available.
        let runtimes = RuntimeDetector::detect_installed();
        let names: Vec<&str> = runtimes.iter().map(|r| r.name.as_str()).collect();
        assert!(
            names.contains(&"cargo"),
            "cargo should be detected; found: {names:?}"
        );
    }

    #[test]
    fn check_runtime_cargo() {
        let rt = RuntimeDetector::check_runtime("cargo");
        assert!(rt.is_some(), "cargo should be available");
        let rt = rt.unwrap();
        assert_eq!(rt.name, "cargo");
        assert!(!rt.version.is_empty());
        assert!(rt.path.is_some());
    }

    #[test]
    fn check_runtime_unknown_returns_none() {
        assert!(RuntimeDetector::check_runtime("nonexistent_runtime_xyz").is_none());
    }

    #[test]
    fn runtime_command_known() {
        assert!(RuntimeDetector::runtime_command("bun").is_some());
        assert!(RuntimeDetector::runtime_command("node").is_some());
        assert!(RuntimeDetector::runtime_command("python").is_some());
        assert!(RuntimeDetector::runtime_command("cargo").is_some());
        assert!(RuntimeDetector::runtime_command("go").is_some());
        assert!(RuntimeDetector::runtime_command("deno").is_some());
    }

    #[test]
    fn runtime_command_unknown() {
        assert!(RuntimeDetector::runtime_command("ruby").is_none());
        assert!(RuntimeDetector::runtime_command("").is_none());
    }

    #[test]
    fn detected_runtime_serialization() {
        let rt = DetectedRuntime {
            name: "node".into(),
            version: "22.11.0".into(),
            path: Some(PathBuf::from("/usr/local/bin/node")),
        };

        let json = serde_json::to_value(&rt).unwrap();
        assert_eq!(json["name"], "node");
        assert_eq!(json["version"], "22.11.0");
        assert_eq!(json["path"], "/usr/local/bin/node");
    }

    #[test]
    fn detected_runtime_roundtrip() {
        let rt = DetectedRuntime {
            name: "bun".into(),
            version: "1.1.38".into(),
            path: None,
        };

        let json_str = serde_json::to_string(&rt).unwrap();
        let deserialized: DetectedRuntime = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.name, "bun");
        assert_eq!(deserialized.version, "1.1.38");
        assert!(deserialized.path.is_none());
    }
}
