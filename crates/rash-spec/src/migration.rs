use std::fs;
use std::path::Path;

use chrono::Utc;
use semver::Version;
use thiserror::Error;

use crate::types::error::{ErrorEntry, E_MIGRATION_FAILED, E_VERSION_MISMATCH};

// ── Error type ──

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        expected: Version,
        found: Version,
        entry: Box<ErrorEntry>,
    },

    #[error("migration step {from} -> {to} failed: {reason}")]
    StepFailed {
        from: Version,
        to: Version,
        reason: String,
        entry: Box<ErrorEntry>,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid version: {0}")]
    InvalidVersion(#[from] semver::Error),

    #[error("no migration path from {from} to {to}")]
    NoPath { from: Version, to: Version },
}

// ── MigrationStep trait ──

pub trait MigrationStep: Send + Sync {
    fn source_version(&self) -> Version;
    fn target_version(&self) -> Version;
    fn migrate(&self, config_value: &mut serde_json::Value) -> Result<(), MigrationError>;
}

// ── MigrationRunner ──

pub struct MigrationRunner {
    steps: Vec<Box<dyn MigrationStep>>,
}

impl MigrationRunner {
    pub fn new(steps: Vec<Box<dyn MigrationStep>>) -> Self {
        Self { steps }
    }

    /// Create a runner with the built-in migration chain.
    pub fn with_builtins() -> Self {
        Self::new(vec![Box::new(V0_9ToV1_0)])
    }

    /// Run all applicable migration steps from `current_version` forward.
    ///
    /// - Backs up affected files to `.rash/migrations/<timestamp>/`
    /// - On failure: leaves backup, returns error
    /// - On success: updates version field in config
    pub fn run(
        &self,
        project_dir: &Path,
        current_version: &Version,
    ) -> Result<Version, MigrationError> {
        // Collect the applicable steps in order
        let applicable: Vec<&dyn MigrationStep> = self
            .steps
            .iter()
            .filter(|s| &s.source_version() >= current_version)
            .map(|s| s.as_ref())
            .collect();

        if applicable.is_empty() {
            // Already at (or past) the latest version — no-op
            return Ok(current_version.clone());
        }

        // Verify chain continuity
        let mut expected = current_version.clone();
        for step in &applicable {
            if step.source_version() != expected {
                return Err(MigrationError::NoPath {
                    from: current_version.clone(),
                    to: step.target_version(),
                });
            }
            expected = step.target_version();
        }

        let config_path = project_dir.join("rash.config.json");
        let config_text = fs::read_to_string(&config_path)?;
        let mut config_value: serde_json::Value = serde_json::from_str(&config_text)?;

        // Create backup
        let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
        let backup_dir = project_dir.join(".rash/migrations").join(&timestamp);
        fs::create_dir_all(&backup_dir)?;
        fs::write(backup_dir.join("rash.config.json"), &config_text)?;

        // Run each step
        let mut version = current_version.clone();
        for step in &applicable {
            step.migrate(&mut config_value).map_err(|e| {
                // Leave backup in place and propagate error
                match e {
                    MigrationError::StepFailed { .. } => e,
                    other => MigrationError::StepFailed {
                        from: step.source_version(),
                        to: step.target_version(),
                        reason: other.to_string(),
                        entry: Box::new(ErrorEntry::error(
                            E_MIGRATION_FAILED,
                            format!(
                                "Migration {} -> {} failed: {}",
                                step.source_version(),
                                step.target_version(),
                                other
                            ),
                            "rash.config.json",
                            "$.version",
                        )),
                    },
                }
            })?;
            version = step.target_version();
        }

        // Update version field in the config value
        if let Some(obj) = config_value.as_object_mut() {
            obj.insert(
                "version".to_string(),
                serde_json::json!(version.to_string()),
            );
        }

        // Write back the migrated config
        let output = serde_json::to_string_pretty(&config_value)?;
        fs::write(&config_path, output)?;

        Ok(version)
    }
}

// ── Built-in migration steps ──

/// Migrate spec format from v0.9.0 to v1.0.0.
///
/// Changes:
/// - Updates `version` from "0.9.0" to "1.0.0"
/// - Adds `meta.lastMigratedFrom: "0.9.0"`
pub struct V0_9ToV1_0;

impl MigrationStep for V0_9ToV1_0 {
    fn source_version(&self) -> Version {
        Version::new(0, 9, 0)
    }

    fn target_version(&self) -> Version {
        Version::new(1, 0, 0)
    }

    fn migrate(&self, config_value: &mut serde_json::Value) -> Result<(), MigrationError> {
        let obj = config_value
            .as_object_mut()
            .ok_or_else(|| MigrationError::StepFailed {
                from: self.source_version(),
                to: self.target_version(),
                reason: "config is not a JSON object".to_string(),
                entry: Box::new(ErrorEntry::error(
                    E_MIGRATION_FAILED,
                    "Config root must be a JSON object",
                    "rash.config.json",
                    "$",
                )),
            })?;

        // Verify current version
        let current = obj.get("version").and_then(|v| v.as_str()).unwrap_or("");
        if current != "0.9.0" {
            return Err(MigrationError::VersionMismatch {
                expected: self.source_version(),
                found: Version::parse(current).unwrap_or(Version::new(0, 0, 0)),
                entry: Box::new(ErrorEntry::error(
                    E_VERSION_MISMATCH,
                    format!("Expected version 0.9.0, found '{current}'"),
                    "rash.config.json",
                    "$.version",
                )),
            });
        }

        // Update version
        obj.insert("version".to_string(), serde_json::json!("1.0.0"));

        // Ensure meta object exists and add lastMigratedFrom
        let meta = obj.entry("meta").or_insert_with(|| serde_json::json!({}));
        if let Some(meta_obj) = meta.as_object_mut() {
            meta_obj.insert("lastMigratedFrom".to_string(), serde_json::json!("0.9.0"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project(dir: &Path, version: &str) -> serde_json::Value {
        let config = serde_json::json!({
            "version": version,
            "name": "test-project",
            "target": {
                "language": "typescript",
                "framework": "express",
                "runtime": "bun"
            },
            "server": {
                "port": 3000,
                "host": "0.0.0.0"
            }
        });
        fs::write(
            dir.join("rash.config.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
        config
    }

    fn read_config(dir: &Path) -> serde_json::Value {
        let text = fs::read_to_string(dir.join("rash.config.json")).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn test_successful_migration_v0_9_to_v1_0() {
        let tmp = TempDir::new().unwrap();
        create_test_project(tmp.path(), "0.9.0");

        let runner = MigrationRunner::with_builtins();
        let result = runner.run(tmp.path(), &Version::new(0, 9, 0));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Version::new(1, 0, 0));

        let config = read_config(tmp.path());
        assert_eq!(config["version"], "1.0.0");
        assert_eq!(config["meta"]["lastMigratedFrom"], "0.9.0");
    }

    #[test]
    fn test_failed_migration_leaves_backup() {
        let tmp = TempDir::new().unwrap();
        // Create a config with version 0.8.0 which V0_9ToV1_0 does not expect
        create_test_project(tmp.path(), "0.8.0");

        let runner = MigrationRunner::with_builtins();
        // Tell runner current version is 0.9.0 so the step is selected,
        // but the actual file has 0.8.0, causing the step to fail.
        let result = runner.run(tmp.path(), &Version::new(0, 9, 0));
        assert!(result.is_err());

        // Backup directory should exist
        let migrations_dir = tmp.path().join(".rash/migrations");
        assert!(migrations_dir.exists());

        // At least one backup folder with the original config
        let backups: Vec<_> = fs::read_dir(&migrations_dir).unwrap().collect();
        assert_eq!(backups.len(), 1);

        let backup_dir = backups[0].as_ref().unwrap().path();
        let backup_config: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(backup_dir.join("rash.config.json")).unwrap())
                .unwrap();
        assert_eq!(backup_config["version"], "0.8.0");

        // Original file should be unchanged (migration failed before writing)
        let original = read_config(tmp.path());
        assert_eq!(original["version"], "0.8.0");
    }

    #[test]
    fn test_already_at_target_version_is_noop() {
        let tmp = TempDir::new().unwrap();
        create_test_project(tmp.path(), "1.0.0");

        let runner = MigrationRunner::with_builtins();
        let result = runner.run(tmp.path(), &Version::new(1, 0, 0));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Version::new(1, 0, 0));

        // No backup directory should be created
        assert!(!tmp.path().join(".rash/migrations").exists());

        // Config should be unchanged
        let config = read_config(tmp.path());
        assert_eq!(config["version"], "1.0.0");
    }

    #[test]
    fn test_migration_preserves_existing_meta_fields() {
        let tmp = TempDir::new().unwrap();
        let config = serde_json::json!({
            "version": "0.9.0",
            "name": "test-project",
            "target": {
                "language": "typescript",
                "framework": "express",
                "runtime": "bun"
            },
            "server": {
                "port": 3000,
                "host": "0.0.0.0"
            },
            "meta": {
                "createdAt": "2026-01-15T00:00:00Z",
                "rashVersion": "0.1.0"
            }
        });
        fs::write(
            tmp.path().join("rash.config.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();

        let runner = MigrationRunner::with_builtins();
        let result = runner.run(tmp.path(), &Version::new(0, 9, 0));
        assert!(result.is_ok());

        let migrated = read_config(tmp.path());
        assert_eq!(migrated["version"], "1.0.0");
        assert_eq!(migrated["meta"]["lastMigratedFrom"], "0.9.0");
        assert_eq!(migrated["meta"]["createdAt"], "2026-01-15T00:00:00Z");
        assert_eq!(migrated["meta"]["rashVersion"], "0.1.0");
    }

    #[test]
    fn test_migration_step_trait_versions() {
        let step = V0_9ToV1_0;
        assert_eq!(step.source_version(), Version::new(0, 9, 0));
        assert_eq!(step.target_version(), Version::new(1, 0, 0));
    }
}
