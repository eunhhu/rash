use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// HMU_UPDATE: Rash -> Server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmuUpdate {
    #[serde(rename = "type")]
    pub kind: HmuUpdateType,
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub modules: Vec<HmuModule>,
}

/// Fixed type tag for HMU_UPDATE messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HmuUpdateType {
    #[serde(rename = "HMU_UPDATE")]
    HmuUpdate,
}

/// A single module in an HMU update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HmuModule {
    pub path: String,
    pub action: HmuAction,
    pub content: String,
    pub hash: String,
}

/// Action to perform on a module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HmuAction {
    Replace,
    Add,
    Remove,
}

/// HMU_ACK: Server -> Rash
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HmuAck {
    #[serde(rename = "type")]
    pub kind: HmuAckType,
    pub id: String,
    pub status: HmuStatus,
    pub applied: Vec<String>,
    pub failed: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolled_back: Option<bool>,
    pub requires_restart: bool,
}

/// Fixed type tag for HMU_ACK messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HmuAckType {
    #[serde(rename = "HMU_ACK")]
    HmuAck,
}

/// Status of an HMU operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HmuStatus {
    Success,
    Partial,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;

    #[test]
    fn hmu_update_serializes_to_expected_json() {
        let update = HmuUpdate {
            kind: HmuUpdateType::HmuUpdate,
            id: "hmu_001".into(),
            timestamp: Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap(),
            modules: vec![HmuModule {
                path: "src/handlers/users.ts".into(),
                action: HmuAction::Replace,
                content: "export async function getUser(ctx) { ... }".into(),
                hash: "sha256:abc123...".into(),
            }],
        };

        let json = serde_json::to_value(&update).unwrap();

        assert_eq!(json["type"], "HMU_UPDATE");
        assert_eq!(json["id"], "hmu_001");
        assert_eq!(json["timestamp"], "2026-01-15T12:00:00Z");
        assert_eq!(json["modules"][0]["path"], "src/handlers/users.ts");
        assert_eq!(json["modules"][0]["action"], "replace");
        assert_eq!(json["modules"][0]["hash"], "sha256:abc123...");
    }

    #[test]
    fn hmu_update_roundtrip() {
        let update = HmuUpdate {
            kind: HmuUpdateType::HmuUpdate,
            id: "hmu_002".into(),
            timestamp: Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap(),
            modules: vec![
                HmuModule {
                    path: "src/handlers/users.ts".into(),
                    action: HmuAction::Replace,
                    content: "new content".into(),
                    hash: "sha256:def456".into(),
                },
                HmuModule {
                    path: "src/handlers/posts.ts".into(),
                    action: HmuAction::Add,
                    content: "post content".into(),
                    hash: "sha256:ghi789".into(),
                },
            ],
        };

        let json_str = serde_json::to_string(&update).unwrap();
        let deserialized: HmuUpdate = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.id, "hmu_002");
        assert_eq!(deserialized.modules.len(), 2);
        assert_eq!(deserialized.modules[0].action, HmuAction::Replace);
        assert_eq!(deserialized.modules[1].action, HmuAction::Add);
    }

    #[test]
    fn hmu_ack_success_serializes_correctly() {
        let ack = HmuAck {
            kind: HmuAckType::HmuAck,
            id: "hmu_001".into(),
            status: HmuStatus::Success,
            applied: vec!["src/handlers/users.ts".into()],
            failed: vec![],
            rolled_back: None,
            requires_restart: false,
        };

        let json = serde_json::to_value(&ack).unwrap();

        assert_eq!(json["type"], "HMU_ACK");
        assert_eq!(json["id"], "hmu_001");
        assert_eq!(json["status"], "success");
        assert_eq!(json["applied"], serde_json::json!(["src/handlers/users.ts"]));
        assert_eq!(json["failed"], serde_json::json!([]));
        assert!(json.get("rolledBack").is_none());
        assert_eq!(json["requiresRestart"], false);
    }

    #[test]
    fn hmu_ack_failed_with_rollback() {
        let ack = HmuAck {
            kind: HmuAckType::HmuAck,
            id: "hmu_001".into(),
            status: HmuStatus::Failed,
            applied: vec![],
            failed: vec!["src/handlers/users.ts".into()],
            rolled_back: Some(true),
            requires_restart: false,
        };

        let json = serde_json::to_value(&ack).unwrap();

        assert_eq!(json["status"], "failed");
        assert_eq!(json["rolledBack"], true);
        assert_eq!(json["requiresRestart"], false);
    }

    #[test]
    fn hmu_ack_roundtrip() {
        let ack = HmuAck {
            kind: HmuAckType::HmuAck,
            id: "hmu_003".into(),
            status: HmuStatus::Partial,
            applied: vec!["a.ts".into()],
            failed: vec!["b.ts".into()],
            rolled_back: Some(false),
            requires_restart: true,
        };

        let json_str = serde_json::to_string(&ack).unwrap();
        let deserialized: HmuAck = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.status, HmuStatus::Partial);
        assert_eq!(deserialized.applied, vec!["a.ts"]);
        assert_eq!(deserialized.failed, vec!["b.ts"]);
        assert_eq!(deserialized.rolled_back, Some(false));
        assert!(deserialized.requires_restart);
    }

    #[test]
    fn hmu_action_variants() {
        assert_eq!(
            serde_json::to_value(HmuAction::Replace).unwrap(),
            "replace"
        );
        assert_eq!(serde_json::to_value(HmuAction::Add).unwrap(), "add");
        assert_eq!(serde_json::to_value(HmuAction::Remove).unwrap(), "remove");
    }

    #[test]
    fn hmu_status_variants() {
        assert_eq!(
            serde_json::to_value(HmuStatus::Success).unwrap(),
            "success"
        );
        assert_eq!(
            serde_json::to_value(HmuStatus::Partial).unwrap(),
            "partial"
        );
        assert_eq!(
            serde_json::to_value(HmuStatus::Failed).unwrap(),
            "failed"
        );
    }
}
