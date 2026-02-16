use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub source: LogSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogSource {
    Stdout,
    Stderr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_serialization() {
        assert_eq!(serde_json::to_value(LogLevel::Info).unwrap(), "info");
        assert_eq!(serde_json::to_value(LogLevel::Warn).unwrap(), "warn");
        assert_eq!(serde_json::to_value(LogLevel::Error).unwrap(), "error");
        assert_eq!(serde_json::to_value(LogLevel::Debug).unwrap(), "debug");
    }

    #[test]
    fn log_source_serialization() {
        assert_eq!(serde_json::to_value(LogSource::Stdout).unwrap(), "stdout");
        assert_eq!(serde_json::to_value(LogSource::Stderr).unwrap(), "stderr");
    }

    #[test]
    fn log_entry_roundtrip() {
        let entry = LogEntry {
            timestamp: chrono::TimeZone::with_ymd_and_hms(&Utc, 2026, 1, 15, 12, 0, 0).unwrap(),
            level: LogLevel::Info,
            message: "Server started on port 3000".into(),
            source: LogSource::Stdout,
        };

        let json_str = serde_json::to_string(&entry).unwrap();
        let deserialized: LogEntry = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.level, LogLevel::Info);
        assert_eq!(deserialized.source, LogSource::Stdout);
        assert_eq!(deserialized.message, "Server started on port 3000");
    }
}
