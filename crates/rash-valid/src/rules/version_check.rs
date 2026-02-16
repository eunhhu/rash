use semver::Version;

use rash_spec::loader::LoadedProject;
use rash_spec::types::error::{ErrorEntry, ValidationReport, E_VERSION_MISMATCH};

const SUPPORTED_VERSION: &str = "1.0.0";

/// Check that the project spec version is supported.
pub fn check(project: &LoadedProject, report: &mut ValidationReport) {
    let version_str = &project.config.version;

    match Version::parse(version_str) {
        Ok(version) => {
            let supported = Version::parse(SUPPORTED_VERSION).unwrap();
            if version.major != supported.major || version.minor != supported.minor {
                report.push(
                    ErrorEntry::error(
                        E_VERSION_MISMATCH,
                        format!(
                            "Unsupported spec version '{}'. Expected compatible with {}",
                            version_str, SUPPORTED_VERSION
                        ),
                        "rash.config.json",
                        "$.version",
                    )
                    .with_suggestion(
                        "Run 'rash migrate' to update to the latest spec version, or update the version field manually",
                    ),
                );
            }
        }
        Err(_) => {
            report.push(
                ErrorEntry::error(
                    E_VERSION_MISMATCH,
                    format!("Invalid semver version: '{}'", version_str),
                    "rash.config.json",
                    "$.version",
                )
                .with_suggestion("Version must be valid semver (e.g., '1.0.0')"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use rash_spec::types::common::{Framework, Language, Runtime};
    use rash_spec::types::config::{RashConfig, ServerConfig, TargetConfig};

    fn project_with_version(version: &str) -> LoadedProject {
        LoadedProject {
            root: PathBuf::from("/tmp/test"),
            config: RashConfig {
                schema: None,
                version: version.to_string(),
                name: "test".to_string(),
                description: None,
                target: TargetConfig {
                    language: Language::Typescript,
                    framework: Framework::Express,
                    runtime: Runtime::Bun,
                },
                server: ServerConfig {
                    port: 3000,
                    host: "0.0.0.0".to_string(),
                    protocol: None,
                    base_path: None,
                },
                database: None,
                codegen: None,
                middleware: None,
                plugins: vec![],
                meta: None,
            },
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        }
    }

    #[test]
    fn test_valid_version() {
        let project = project_with_version("1.0.0");
        let mut report = ValidationReport::success();
        check(&project, &mut report);
        assert!(!report.has_errors());
    }

    #[test]
    fn test_compatible_patch_version() {
        let project = project_with_version("1.0.3");
        let mut report = ValidationReport::success();
        check(&project, &mut report);
        assert!(!report.has_errors());
    }

    #[test]
    fn test_old_version_detected() {
        let project = project_with_version("0.9.0");
        let mut report = ValidationReport::success();
        check(&project, &mut report);
        assert!(report.has_errors());
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].code, E_VERSION_MISMATCH);
    }

    #[test]
    fn test_invalid_semver() {
        let project = project_with_version("not-a-version");
        let mut report = ValidationReport::success();
        check(&project, &mut report);
        assert!(report.has_errors());
        assert_eq!(report.errors[0].code, E_VERSION_MISMATCH);
    }

    #[test]
    fn test_future_major_version() {
        let project = project_with_version("2.0.0");
        let mut report = ValidationReport::success();
        check(&project, &mut report);
        assert!(report.has_errors());
    }
}
