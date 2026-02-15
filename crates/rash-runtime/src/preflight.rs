use serde::{Deserialize, Serialize};

/// Result of all preflight checks before server start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    pub ok: bool,
    pub checks: Vec<PreflightCheck>,
}

/// A single preflight check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightCheck {
    pub code: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Status of a preflight check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn preflight_report_all_pass() {
        let report = PreflightReport {
            ok: true,
            checks: vec![
                PreflightCheck {
                    code: "RUNTIME_EXISTS".into(),
                    status: CheckStatus::Pass,
                    message: "bun v1.2.0 found".into(),
                    suggestion: None,
                },
                PreflightCheck {
                    code: "PORT_AVAILABLE".into(),
                    status: CheckStatus::Pass,
                    message: "Port 3000 is available".into(),
                    suggestion: None,
                },
            ],
        };

        let json = serde_json::to_value(&report).unwrap();

        assert_eq!(json["ok"], true);
        assert_eq!(json["checks"].as_array().unwrap().len(), 2);
        assert_eq!(json["checks"][0]["status"], "pass");
        assert_eq!(json["checks"][0]["code"], "RUNTIME_EXISTS");
        assert!(json["checks"][0].get("suggestion").is_none());
    }

    #[test]
    fn preflight_report_with_failure() {
        let report = PreflightReport {
            ok: false,
            checks: vec![
                PreflightCheck {
                    code: "E_RUNTIME_NOT_FOUND".into(),
                    status: CheckStatus::Fail,
                    message: "bun not found in PATH".into(),
                    suggestion: Some("Install bun: curl -fsSL https://bun.sh/install | bash".into()),
                },
                PreflightCheck {
                    code: "E_PORT_IN_USE".into(),
                    status: CheckStatus::Warn,
                    message: "Port 3000 is in use".into(),
                    suggestion: Some("Kill the process or use a different port".into()),
                },
            ],
        };

        let json = serde_json::to_value(&report).unwrap();

        assert_eq!(json["ok"], false);
        assert_eq!(json["checks"][0]["status"], "fail");
        assert_eq!(json["checks"][0]["suggestion"], "Install bun: curl -fsSL https://bun.sh/install | bash");
        assert_eq!(json["checks"][1]["status"], "warn");
    }

    #[test]
    fn preflight_report_roundtrip() {
        let report = PreflightReport {
            ok: false,
            checks: vec![PreflightCheck {
                code: "E_PORT_IN_USE".into(),
                status: CheckStatus::Fail,
                message: "Port 3000 is in use by another process".into(),
                suggestion: Some("Use a different port".into()),
            }],
        };

        let json_str = serde_json::to_string(&report).unwrap();
        let deserialized: PreflightReport = serde_json::from_str(&json_str).unwrap();

        assert!(!deserialized.ok);
        assert_eq!(deserialized.checks.len(), 1);
        assert_eq!(deserialized.checks[0].status, CheckStatus::Fail);
        assert_eq!(
            deserialized.checks[0].suggestion.as_deref(),
            Some("Use a different port")
        );
    }

    #[test]
    fn check_status_variants() {
        assert_eq!(serde_json::to_value(CheckStatus::Pass).unwrap(), "pass");
        assert_eq!(serde_json::to_value(CheckStatus::Warn).unwrap(), "warn");
        assert_eq!(serde_json::to_value(CheckStatus::Fail).unwrap(), "fail");
    }

    #[test]
    fn preflight_check_without_suggestion_omits_field() {
        let check = PreflightCheck {
            code: "SPEC_VALID".into(),
            status: CheckStatus::Pass,
            message: "Spec validation passed".into(),
            suggestion: None,
        };

        let json_str = serde_json::to_string(&check).unwrap();
        assert!(!json_str.contains("suggestion"));
    }
}
