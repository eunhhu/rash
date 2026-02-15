use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Reference to another spec element (e.g., schema, handler, middleware, model)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Ref {
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

/// Type reference used in AST nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeRef {
    /// Simple type: "string", "number", etc.
    Simple(String),
    /// Reference to a schema/model
    Reference(Ref),
    /// Complex type with nullable
    Complex {
        #[serde(rename = "ref")]
        reference: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        nullable: Option<bool>,
    },
}

/// AST node portability tier (serialized as integer 0-3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Tier {
    /// Universal: works identically in all languages
    Universal = 0,
    /// Domain: server-domain specific, Rash provides mappings
    Domain = 1,
    /// Utility: general utilities, most languages have equivalents
    Utility = 2,
    /// Bridge: language/ecosystem specific, locks language
    Bridge = 3,
}

impl Serialize for Tier {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for Tier {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u8::deserialize(deserializer)?;
        match value {
            0 => Ok(Tier::Universal),
            1 => Ok(Tier::Domain),
            2 => Ok(Tier::Utility),
            3 => Ok(Tier::Bridge),
            _ => Err(serde::de::Error::custom(format!(
                "invalid tier value: {value}, expected 0-3"
            ))),
        }
    }
}

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

/// Target programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Typescript,
    Rust,
    Python,
    Go,
}

/// Target web frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    // TypeScript
    Express,
    Fastify,
    Hono,
    Elysia,
    #[serde(rename = "nestjs")]
    NestJS,
    // Rust
    Actix,
    Axum,
    Rocket,
    // Python
    #[serde(rename = "fastapi")]
    FastAPI,
    Django,
    Flask,
    // Go
    Gin,
    Echo,
    Fiber,
}

/// Server runtime environments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    Bun,
    Node,
    Deno,
    Cargo,
    Python,
    Go,
}

/// Database types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    #[serde(rename = "postgresql")]
    PostgreSQL,
    #[serde(rename = "mysql")]
    MySQL,
    #[serde(rename = "sqlite")]
    SQLite,
    #[serde(rename = "mongodb")]
    MongoDB,
}

/// ORM choices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Orm {
    Prisma,
    #[serde(rename = "typeorm")]
    TypeORM,
    #[serde(rename = "seaorm")]
    SeaORM,
    #[serde(rename = "sqlalchemy")]
    SQLAlchemy,
    #[serde(rename = "django-orm")]
    DjangoORM,
    #[serde(rename = "gorm")]
    Gorm,
}

/// Metadata attached to spec files
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rash_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_migrated_from: Option<String>,
}

/// Error/warning severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Server protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    Https,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_serialization() {
        let r = Ref {
            reference: "UserResponse".to_string(),
            config: None,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["ref"], "UserResponse");
    }

    #[test]
    fn test_ref_with_config() {
        let r = Ref {
            reference: "auth".to_string(),
            config: Some(serde_json::json!({ "roles": ["admin"] })),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["ref"], "auth");
        assert_eq!(json["config"]["roles"][0], "admin");
    }

    #[test]
    fn test_tier_ordering() {
        assert!(Tier::Universal < Tier::Domain);
        assert!(Tier::Domain < Tier::Utility);
        assert!(Tier::Utility < Tier::Bridge);
    }

    #[test]
    fn test_tier_serialization() {
        assert_eq!(serde_json::to_value(Tier::Universal).unwrap(), 0);
        assert_eq!(serde_json::to_value(Tier::Domain).unwrap(), 1);
        assert_eq!(serde_json::to_value(Tier::Bridge).unwrap(), 3);
    }

    #[test]
    fn test_http_method_serialization() {
        assert_eq!(
            serde_json::to_value(HttpMethod::Get).unwrap(),
            "GET"
        );
        assert_eq!(
            serde_json::to_value(HttpMethod::Post).unwrap(),
            "POST"
        );
    }

    #[test]
    fn test_language_serialization() {
        assert_eq!(
            serde_json::to_value(Language::Typescript).unwrap(),
            "typescript"
        );
        assert_eq!(
            serde_json::to_value(Language::Rust).unwrap(),
            "rust"
        );
    }

    #[test]
    fn test_framework_serialization() {
        assert_eq!(
            serde_json::to_value(Framework::Express).unwrap(),
            "express"
        );
        assert_eq!(
            serde_json::to_value(Framework::FastAPI).unwrap(),
            "fastapi"
        );
        assert_eq!(
            serde_json::to_value(Framework::NestJS).unwrap(),
            "nestjs"
        );
    }

    #[test]
    fn test_severity_serialization() {
        assert_eq!(
            serde_json::to_value(Severity::Error).unwrap(),
            "error"
        );
        assert_eq!(
            serde_json::to_value(Severity::Warning).unwrap(),
            "warning"
        );
    }

    #[test]
    fn test_meta_deserialization() {
        let json = serde_json::json!({
            "createdAt": "2026-01-15T00:00:00Z",
            "rashVersion": "0.1.0"
        });
        let meta: Meta = serde_json::from_value(json).unwrap();
        assert_eq!(meta.created_at.unwrap(), "2026-01-15T00:00:00Z");
        assert_eq!(meta.rash_version.unwrap(), "0.1.0");
    }
}
