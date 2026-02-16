use std::net::TcpListener;
use std::path::Path;
use std::process::Command;

use rash_spec::types::common::Runtime;
use rash_spec::types::config::RashConfig;

use crate::preflight::{CheckStatus, PreflightCheck, PreflightReport};

pub struct PreflightChecker;

impl PreflightChecker {
    /// Run all preflight checks for the given project configuration.
    pub fn run(config: &RashConfig, project_dir: &Path) -> PreflightReport {
        let mut checks = Vec::new();

        checks.push(Self::check_runtime_exists(config.target.runtime));
        checks.push(Self::check_port_available(config.server.port));
        checks.push(Self::check_output_dir(config, project_dir));

        let ok = !checks.iter().any(|c| c.status == CheckStatus::Fail);
        PreflightReport { ok, checks }
    }

    /// Check whether the configured runtime binary is available on the system.
    fn check_runtime_exists(runtime: Runtime) -> PreflightCheck {
        let (bin, install_hint) = runtime_binary_info(runtime);

        match Command::new(bin).arg("--version").output() {
            Ok(output) if output.status.success() => {
                let raw = String::from_utf8_lossy(&output.stdout);
                let version = raw.lines().next().unwrap_or("").trim();
                PreflightCheck {
                    code: "RUNTIME_EXISTS".into(),
                    status: CheckStatus::Pass,
                    message: format!("{bin} {version} found"),
                    suggestion: None,
                }
            }
            _ => PreflightCheck {
                code: "RUNTIME_EXISTS".into(),
                status: CheckStatus::Fail,
                message: format!("{bin} not found in PATH"),
                suggestion: Some(format!("Install {bin}: {install_hint}")),
            },
        }
    }

    /// Check whether the configured port is available for binding.
    fn check_port_available(port: u16) -> PreflightCheck {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(_) => PreflightCheck {
                code: "PORT_AVAILABLE".into(),
                status: CheckStatus::Pass,
                message: format!("Port {port} is available"),
                suggestion: None,
            },
            Err(_) => PreflightCheck {
                code: "PORT_AVAILABLE".into(),
                status: CheckStatus::Warn,
                message: format!("Port {port} is in use"),
                suggestion: Some(format!(
                    "Kill the process using port {port} or use a different port"
                )),
            },
        }
    }

    /// Check whether the codegen output directory is writable.
    fn check_output_dir(config: &RashConfig, project_dir: &Path) -> PreflightCheck {
        let out_dir_raw = config
            .codegen
            .as_ref()
            .map(|c| c.out_dir.as_str())
            .unwrap_or("./dist");

        let out_path = project_dir.join(out_dir_raw);

        if out_path.exists() {
            if is_writable(&out_path) {
                PreflightCheck {
                    code: "OUTPUT_DIR_WRITABLE".into(),
                    status: CheckStatus::Pass,
                    message: format!("Output directory {} is writable", out_path.display()),
                    suggestion: None,
                }
            } else {
                PreflightCheck {
                    code: "OUTPUT_DIR_WRITABLE".into(),
                    status: CheckStatus::Fail,
                    message: format!(
                        "Output directory {} is not writable",
                        out_path.display()
                    ),
                    suggestion: Some(format!("Check permissions on {}", out_path.display())),
                }
            }
        } else {
            // Directory doesn't exist yet — check if parent is writable so we can create it.
            let parent = out_path.parent().unwrap_or(project_dir);
            if parent.exists() && is_writable(parent) {
                PreflightCheck {
                    code: "OUTPUT_DIR_WRITABLE".into(),
                    status: CheckStatus::Pass,
                    message: format!(
                        "Output directory {} will be created (parent is writable)",
                        out_path.display()
                    ),
                    suggestion: None,
                }
            } else {
                PreflightCheck {
                    code: "OUTPUT_DIR_WRITABLE".into(),
                    status: CheckStatus::Fail,
                    message: format!(
                        "Cannot create output directory {}",
                        out_path.display()
                    ),
                    suggestion: Some(format!("Check permissions on {}", parent.display())),
                }
            }
        }
    }
}

/// Map a `Runtime` enum variant to its binary name and an install hint URL/command.
fn runtime_binary_info(runtime: Runtime) -> (&'static str, &'static str) {
    match runtime {
        Runtime::Bun => ("bun", "https://bun.sh/docs/installation"),
        Runtime::Node => ("node", "https://nodejs.org/"),
        Runtime::Deno => ("deno", "https://deno.land/#installation"),
        Runtime::Cargo => ("cargo", "https://rustup.rs/"),
        Runtime::Python => ("python3", "https://www.python.org/downloads/"),
        Runtime::Go => ("go", "https://go.dev/dl/"),
    }
}

/// Check if a path is writable by attempting to write a temporary file.
fn is_writable(path: &Path) -> bool {
    if path.is_dir() {
        let test_file = path.join(".rash_write_test");
        match std::fs::write(&test_file, b"") {
            Ok(()) => {
                let _ = std::fs::remove_file(&test_file);
                true
            }
            Err(_) => false,
        }
    } else {
        path.parent().map_or(false, |p| is_writable(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rash_spec::types::common::{Framework, Language};
    use rash_spec::types::config::{CodegenConfig, ServerConfig, TargetConfig};
    use std::net::TcpListener;

    fn make_config(runtime: Runtime, port: u16, out_dir: Option<&str>) -> RashConfig {
        RashConfig {
            schema: None,
            version: "1.0.0".into(),
            name: "test-project".into(),
            description: None,
            target: TargetConfig {
                language: Language::Typescript,
                framework: Framework::Express,
                runtime,
            },
            server: ServerConfig {
                port,
                host: "127.0.0.1".into(),
                protocol: None,
                base_path: None,
            },
            database: None,
            codegen: out_dir.map(|d| CodegenConfig {
                out_dir: d.to_string(),
                source_map: false,
                strict: false,
            }),
            middleware: None,
            plugins: vec![],
            meta: None,
        }
    }

    // ── Runtime check tests ──────────────────────────────────────

    #[test]
    fn runtime_check_cargo_passes() {
        let check = PreflightChecker::check_runtime_exists(Runtime::Cargo);
        assert_eq!(check.code, "RUNTIME_EXISTS");
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.message.contains("cargo"));
        assert!(check.suggestion.is_none());
    }

    #[test]
    fn runtime_binary_info_maps_all_variants() {
        let cases = [
            (Runtime::Bun, "bun"),
            (Runtime::Node, "node"),
            (Runtime::Deno, "deno"),
            (Runtime::Cargo, "cargo"),
            (Runtime::Python, "python3"),
            (Runtime::Go, "go"),
        ];
        for (runtime, expected_bin) in cases {
            let (bin, hint) = runtime_binary_info(runtime);
            assert_eq!(bin, expected_bin, "binary mismatch for {runtime:?}");
            assert!(!hint.is_empty(), "install hint should not be empty for {runtime:?}");
        }
    }

    // ── Port check tests ─────────────────────────────────────────

    #[test]
    fn port_check_available() {
        // Bind to port 0 to get an OS-assigned free port, then release it.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let free_port = listener.local_addr().unwrap().port();
        drop(listener);

        let check = PreflightChecker::check_port_available(free_port);
        assert_eq!(check.code, "PORT_AVAILABLE");
        assert_eq!(check.status, CheckStatus::Pass);
    }

    #[test]
    fn port_check_occupied() {
        // Hold a port open so the check finds it in use.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = listener.local_addr().unwrap().port();

        let check = PreflightChecker::check_port_available(occupied_port);
        assert_eq!(check.code, "PORT_AVAILABLE");
        assert_eq!(check.status, CheckStatus::Warn);
        assert!(check.suggestion.is_some());

        drop(listener);
    }

    #[test]
    fn port_warn_does_not_cause_report_failure() {
        let check = PreflightCheck {
            code: "PORT_AVAILABLE".into(),
            status: CheckStatus::Warn,
            message: "Port in use".into(),
            suggestion: Some("Use another port".into()),
        };
        let report = PreflightReport {
            ok: ![&check].iter().any(|c| c.status == CheckStatus::Fail),
            checks: vec![check],
        };
        assert!(report.ok, "Warn should not cause ok=false");
    }

    // ── Output dir check tests ───────────────────────────────────

    #[test]
    fn output_dir_writable_existing_dir() {
        let tmp = std::env::temp_dir();
        let config = make_config(Runtime::Cargo, 3000, Some("."));
        let check = PreflightChecker::check_output_dir(&config, &tmp);
        assert_eq!(check.code, "OUTPUT_DIR_WRITABLE");
        assert_eq!(check.status, CheckStatus::Pass);
    }

    #[test]
    fn output_dir_writable_nonexistent_but_parent_writable() {
        let tmp = std::env::temp_dir();
        let config = make_config(Runtime::Cargo, 3000, Some("nonexistent_rash_test_dir"));
        let check = PreflightChecker::check_output_dir(&config, &tmp);
        assert_eq!(check.code, "OUTPUT_DIR_WRITABLE");
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(
            check.message.contains("will be created"),
            "message: {}",
            check.message
        );
    }

    #[test]
    fn output_dir_default_when_codegen_is_none() {
        let tmp = std::env::temp_dir();
        let config = make_config(Runtime::Cargo, 3000, None);
        let check = PreflightChecker::check_output_dir(&config, &tmp);
        assert_eq!(check.code, "OUTPUT_DIR_WRITABLE");
        // ./dist probably doesn't exist under temp, but parent (temp) is writable
        assert_ne!(check.status, CheckStatus::Fail);
    }

    // ── Combined report tests ────────────────────────────────────

    #[test]
    fn report_ok_when_all_pass() {
        let tmp = std::env::temp_dir();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let free_port = listener.local_addr().unwrap().port();
        drop(listener);

        let config = make_config(Runtime::Cargo, free_port, Some("."));
        let report = PreflightChecker::run(&config, &tmp);

        assert!(report.ok, "All checks should pass: {:#?}", report.checks);
        assert_eq!(report.checks.len(), 3);
        for check in &report.checks {
            assert_ne!(check.status, CheckStatus::Fail, "check {} failed", check.code);
        }
    }

    #[test]
    fn report_not_ok_when_any_fail() {
        let tmp = std::env::temp_dir();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let free_port = listener.local_addr().unwrap().port();
        drop(listener);

        let config = make_config(
            Runtime::Cargo,
            free_port,
            Some("/nonexistent_root_path/impossible_dir"),
        );
        let report = PreflightChecker::run(&config, &tmp);

        assert!(!report.ok, "Report should fail: {:#?}", report.checks);
        let failing = report
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count();
        assert!(failing > 0, "At least one check should fail");
    }

    #[test]
    fn report_ok_with_warn() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = listener.local_addr().unwrap().port();

        let tmp = std::env::temp_dir();
        let config = make_config(Runtime::Cargo, occupied_port, Some("."));
        let report = PreflightChecker::run(&config, &tmp);

        assert!(report.ok, "Warn should not cause failure: {:#?}", report.checks);

        let warns = report
            .checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count();
        assert!(warns > 0, "Should have at least one Warn");

        drop(listener);
    }
}
