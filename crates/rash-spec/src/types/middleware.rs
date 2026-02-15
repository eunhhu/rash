use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::common::{Meta, Ref};

/// Middleware specification (*.middleware.json)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiddlewareSpec {
    /// JSON Schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Middleware name
    pub name: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Middleware type
    #[serde(rename = "type")]
    pub middleware_type: MiddlewareType,

    /// Configuration schema for this middleware
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,

    /// Handler reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<Ref>,

    /// Values this middleware provides to downstream handlers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provides: Option<IndexMap<String, serde_json::Value>>,

    /// Error definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<IndexMap<String, MiddlewareError>>,

    /// Composed middleware chain (for type "composed")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compose: Option<Vec<Ref>>,

    /// Short-circuit behavior for composed middleware
    #[serde(rename = "shortCircuit", skip_serializing_if = "Option::is_none")]
    pub short_circuit: Option<bool>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

/// Middleware type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MiddlewareType {
    Request,
    Response,
    Error,
    Composed,
}

/// Middleware error definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiddlewareError {
    pub status: u16,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_deserialization_from_docs() {
        let json = serde_json::json!({
            "name": "auth",
            "description": "JWT 인증 미들웨어",
            "type": "request",
            "config": {
                "type": "object",
                "properties": {
                    "roles": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "optional": {
                        "type": "boolean",
                        "default": false
                    }
                }
            },
            "handler": { "ref": "auth.verifyToken" },
            "provides": {
                "user": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "email": { "type": "string" },
                        "role": { "type": "string" }
                    }
                }
            },
            "errors": {
                "UNAUTHORIZED": {
                    "status": 401,
                    "message": "유효하지 않은 토큰"
                },
                "FORBIDDEN": {
                    "status": 403,
                    "message": "권한 없음"
                }
            }
        });

        let mw: MiddlewareSpec = serde_json::from_value(json).unwrap();
        assert_eq!(mw.name, "auth");
        assert_eq!(mw.middleware_type, MiddlewareType::Request);
        assert_eq!(mw.handler.as_ref().unwrap().reference, "auth.verifyToken");
        assert!(mw.provides.is_some());
        assert_eq!(mw.errors.as_ref().unwrap().len(), 2);
        assert_eq!(mw.errors.as_ref().unwrap()["UNAUTHORIZED"].status, 401);
    }
}
