use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::{
    DatabaseType, Framework, Language, Meta, Orm, Protocol, Ref, Runtime,
};

/// Top-level project configuration (rash.config.json)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RashConfig {
    /// JSON Schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Spec format version (e.g., "1.0.0")
    pub version: String,

    /// Project name
    pub name: String,

    /// Project description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Code generation target
    pub target: TargetConfig,

    /// Server configuration
    pub server: ServerConfig,

    /// Database configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,

    /// Code generation settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codegen: Option<CodegenConfig>,

    /// Global middleware
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middleware: Option<MiddlewareConfig>,

    /// Plugins
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<serde_json::Value>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

/// Target language/framework/runtime configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TargetConfig {
    pub language: Language,
    pub framework: Framework,
    pub runtime: Runtime,
}

/// Server configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default)]
    pub protocol: Option<Protocol>,

    /// Base path prefix for all routes (e.g., "/api")
    #[serde(default, rename = "basePath", skip_serializing_if = "Option::is_none")]
    pub base_path: Option<String>,
}

fn default_port() -> u16 {
    3000
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

/// Database configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DatabaseConfig {
    #[serde(rename = "type")]
    pub db_type: DatabaseType,
    pub orm: Orm,
}

/// Code generation settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodegenConfig {
    #[serde(default = "default_out_dir")]
    pub out_dir: String,

    #[serde(default)]
    pub source_map: bool,

    #[serde(default)]
    pub strict: bool,
}

fn default_out_dir() -> String {
    "./dist".to_string()
}

/// Global middleware configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MiddlewareConfig {
    #[serde(default)]
    pub global: Vec<Ref>,
}

/// Environment variable definitions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    #[serde(flatten)]
    pub environments: IndexMap<String, IndexMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialization() {
        let json = serde_json::json!({
            "$schema": "https://rash.dev/schemas/config.json",
            "version": "1.0.0",
            "name": "my-server",
            "description": "My awesome server",
            "target": {
                "language": "typescript",
                "framework": "express",
                "runtime": "bun"
            },
            "server": {
                "port": 3000,
                "host": "0.0.0.0",
                "protocol": "http",
                "basePath": "/api"
            },
            "database": {
                "type": "postgresql",
                "orm": "prisma"
            },
            "codegen": {
                "outDir": "./dist",
                "sourceMap": true,
                "strict": true
            },
            "middleware": {
                "global": [
                    { "ref": "cors" },
                    { "ref": "rate-limit", "config": { "windowMs": 60000, "max": 100 } }
                ]
            },
            "meta": {
                "createdAt": "2026-01-15T00:00:00Z",
                "rashVersion": "0.1.0"
            }
        });

        let config: RashConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.name, "my-server");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.target.language, Language::Typescript);
        assert_eq!(config.target.framework, Framework::Express);
        assert_eq!(config.target.runtime, Runtime::Bun);
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.base_path.as_deref(), Some("/api"));
        assert_eq!(
            config.database.as_ref().unwrap().db_type,
            DatabaseType::PostgreSQL
        );
        assert_eq!(config.middleware.as_ref().unwrap().global.len(), 2);
    }

    #[test]
    fn test_config_roundtrip() {
        let json = serde_json::json!({
            "version": "1.0.0",
            "name": "test",
            "target": {
                "language": "rust",
                "framework": "actix",
                "runtime": "cargo"
            },
            "server": {
                "port": 8080,
                "host": "127.0.0.1"
            }
        });

        let config: RashConfig = serde_json::from_value(json).unwrap();
        let serialized = serde_json::to_value(&config).unwrap();
        let config2: RashConfig = serde_json::from_value(serialized).unwrap();
        assert_eq!(config, config2);
    }
}
