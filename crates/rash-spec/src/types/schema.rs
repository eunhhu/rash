use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::Meta;

/// Schema specification (*.schema.json) — JSON Schema based DTOs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SchemaSpec {
    /// JSON Schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Schema name (e.g., "User")
    pub name: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Schema definitions — kept as serde_json::Value for flexibility
    /// Each key is a definition name (e.g., "CreateUserBody", "UserResponse")
    pub definitions: IndexMap<String, serde_json::Value>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_deserialization_from_docs() {
        let json = serde_json::json!({
            "$schema": "https://rash.dev/schemas/schema.json",
            "name": "User",
            "description": "사용자 정보",
            "definitions": {
                "CreateUserBody": {
                    "type": "object",
                    "required": ["email", "password", "name"],
                    "properties": {
                        "email": {
                            "type": "string",
                            "format": "email",
                            "maxLength": 255
                        },
                        "password": {
                            "type": "string",
                            "minLength": 8,
                            "maxLength": 128
                        },
                        "name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 100
                        },
                        "role": {
                            "type": "string",
                            "enum": ["admin", "user", "moderator"],
                            "default": "user"
                        }
                    }
                },
                "UserResponse": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "email": { "type": "string", "format": "email" },
                        "name": { "type": "string" }
                    }
                }
            }
        });

        let schema: SchemaSpec = serde_json::from_value(json).unwrap();
        assert_eq!(schema.name, "User");
        assert_eq!(schema.definitions.len(), 2);
        assert!(schema.definitions.contains_key("CreateUserBody"));
        assert!(schema.definitions.contains_key("UserResponse"));
    }

    #[test]
    fn test_schema_roundtrip() {
        let json = serde_json::json!({
            "name": "Auth",
            "definitions": {
                "LoginRequest": {
                    "type": "object",
                    "required": ["email", "password"],
                    "properties": {
                        "email": { "type": "string" },
                        "password": { "type": "string" }
                    }
                }
            }
        });

        let schema: SchemaSpec = serde_json::from_value(json.clone()).unwrap();
        let serialized = serde_json::to_value(&schema).unwrap();
        let schema2: SchemaSpec = serde_json::from_value(serialized).unwrap();
        assert_eq!(schema, schema2);
    }
}
