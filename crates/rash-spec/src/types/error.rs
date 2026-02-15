use serde::{Deserialize, Serialize};

use super::common::Severity;

// ── Error code constants ──

pub const E_REF_NOT_FOUND: &str = "E_REF_NOT_FOUND";
pub const E_REF_TYPE_MISMATCH: &str = "E_REF_TYPE_MISMATCH";
pub const E_REF_CYCLE: &str = "E_REF_CYCLE";
pub const E_REF_AMBIGUOUS: &str = "E_REF_AMBIGUOUS";
pub const E_DUPLICATE_SYMBOL: &str = "E_DUPLICATE_SYMBOL";
pub const E_MISSING_FIELD: &str = "E_MISSING_FIELD";
pub const E_INVALID_TYPE: &str = "E_INVALID_TYPE";
pub const E_PARSE_ERROR: &str = "E_PARSE_ERROR";
pub const E_INVALID_PATH: &str = "E_INVALID_PATH";
pub const E_VERSION_MISMATCH: &str = "E_VERSION_MISMATCH";
pub const E_MIGRATION_FAILED: &str = "E_MIGRATION_FAILED";
pub const E_SCHEMA_VIOLATION: &str = "E_SCHEMA_VIOLATION";
pub const E_INCOMPATIBLE_TARGET: &str = "E_INCOMPATIBLE_TARGET";

/// A single validation/parsing error entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEntry {
    /// Machine-readable stable error code
    pub code: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable error message
    pub message: String,
    /// Relative file path where the error occurred
    pub file: String,
    /// JSONPath to the offending field
    pub path: String,
    /// Suggestion for fixing the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Aggregated validation report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether the spec is valid (no errors, warnings are ok)
    pub ok: bool,
    /// All collected errors, warnings, and info entries
    pub errors: Vec<ErrorEntry>,
}

impl ValidationReport {
    /// Create a successful (empty) report
    pub fn success() -> Self {
        Self {
            ok: true,
            errors: Vec::new(),
        }
    }

    /// Create a report from a list of error entries
    pub fn from_errors(errors: Vec<ErrorEntry>) -> Self {
        let ok = !errors.iter().any(|e| e.severity == Severity::Error);
        Self { ok, errors }
    }

    /// Add an error entry and update the ok flag
    pub fn push(&mut self, entry: ErrorEntry) {
        if entry.severity == Severity::Error {
            self.ok = false;
        }
        self.errors.push(entry);
    }

    /// Merge another report into this one
    pub fn merge(&mut self, other: ValidationReport) {
        for entry in other.errors {
            self.push(entry);
        }
    }

    /// Count errors of a specific severity
    pub fn count(&self, severity: Severity) -> usize {
        self.errors.iter().filter(|e| e.severity == severity).count()
    }

    /// Check if any errors exist (not warnings/info)
    pub fn has_errors(&self) -> bool {
        !self.ok
    }
}

impl ErrorEntry {
    /// Create a new error entry
    pub fn error(code: &str, message: impl Into<String>, file: &str, path: &str) -> Self {
        Self {
            code: code.to_string(),
            severity: Severity::Error,
            message: message.into(),
            file: file.to_string(),
            path: path.to_string(),
            suggestion: None,
        }
    }

    /// Create a warning entry
    pub fn warning(code: &str, message: impl Into<String>, file: &str, path: &str) -> Self {
        Self {
            code: code.to_string(),
            severity: Severity::Warning,
            message: message.into(),
            file: file.to_string(),
            path: path.to_string(),
            suggestion: None,
        }
    }

    /// Add a suggestion to this entry
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_entry_json_format_matches_docs() {
        let entry = ErrorEntry {
            code: E_REF_NOT_FOUND.to_string(),
            severity: Severity::Error,
            message: "Referenced schema 'UserResponse' was not found".to_string(),
            file: "routes/api/v1/users.route.json".to_string(),
            path: "$.methods.GET.response.200.schema.ref".to_string(),
            suggestion: Some(
                "Create schema 'UserResponse' or fix the ref name".to_string(),
            ),
        };

        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["code"], "E_REF_NOT_FOUND");
        assert_eq!(json["severity"], "error");
        assert_eq!(
            json["message"],
            "Referenced schema 'UserResponse' was not found"
        );
        assert_eq!(json["file"], "routes/api/v1/users.route.json");
        assert_eq!(json["path"], "$.methods.GET.response.200.schema.ref");
        assert_eq!(
            json["suggestion"],
            "Create schema 'UserResponse' or fix the ref name"
        );
    }

    #[test]
    fn test_validation_report_json_format_matches_docs() {
        let report = ValidationReport {
            ok: false,
            errors: vec![ErrorEntry {
                code: E_REF_NOT_FOUND.to_string(),
                severity: Severity::Error,
                message: "Referenced schema 'UserResponse' was not found".to_string(),
                file: "routes/api/v1/users.route.json".to_string(),
                path: "$.methods.GET.response.200.schema.ref".to_string(),
                suggestion: Some(
                    "Create schema 'UserResponse' or fix the ref name".to_string(),
                ),
            }],
        };

        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["ok"], false);
        assert!(json["errors"].is_array());
        assert_eq!(json["errors"][0]["code"], "E_REF_NOT_FOUND");
    }

    #[test]
    fn test_validation_report_success() {
        let report = ValidationReport::success();
        assert!(report.ok);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_validation_report_from_errors() {
        let errors = vec![
            ErrorEntry::warning("W_001", "some warning", "file.json", "$.path"),
        ];
        let report = ValidationReport::from_errors(errors);
        assert!(report.ok); // warnings don't make it fail

        let errors = vec![
            ErrorEntry::error(E_REF_NOT_FOUND, "not found", "file.json", "$.path"),
        ];
        let report = ValidationReport::from_errors(errors);
        assert!(!report.ok);
    }

    #[test]
    fn test_validation_report_merge() {
        let mut report1 = ValidationReport::success();
        let mut report2 = ValidationReport::success();
        report2.push(ErrorEntry::error(
            E_REF_NOT_FOUND,
            "not found",
            "file.json",
            "$.path",
        ));
        report1.merge(report2);
        assert!(!report1.ok);
        assert_eq!(report1.errors.len(), 1);
    }

    #[test]
    fn test_error_entry_builder() {
        let entry = ErrorEntry::error(E_REF_NOT_FOUND, "not found", "file.json", "$.path")
            .with_suggestion("fix it");
        assert_eq!(entry.suggestion.unwrap(), "fix it");
    }

    #[test]
    fn test_entry_without_suggestion_skips_in_json() {
        let entry = ErrorEntry::error(E_REF_NOT_FOUND, "not found", "file.json", "$.path");
        let json = serde_json::to_value(&entry).unwrap();
        assert!(json.get("suggestion").is_none());
    }
}
