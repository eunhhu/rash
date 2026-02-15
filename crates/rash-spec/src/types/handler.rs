use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ast::AstNode;
use super::common::{Language, Meta, TypeRef};

/// Handler specification (*.handler.json)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HandlerSpec {
    /// JSON Schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Handler name (e.g., "getUser")
    pub name: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this handler is async
    #[serde(default, rename = "async")]
    pub is_async: bool,

    /// Parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<IndexMap<String, HandlerParam>>,

    /// Return type
    #[serde(rename = "returnType", skip_serializing_if = "Option::is_none")]
    pub return_type: Option<TypeRef>,

    /// Handler body (array of AST statements)
    pub body: Vec<AstNode>,

    /// Handler metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HandlerMeta>,
}

/// Handler parameter definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HandlerParam {
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Handler-specific metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandlerMeta {
    /// Maximum tier of any AST node in this handler
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tier: Option<u8>,

    /// Languages this handler can be generated to
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<Language>,

    /// Bridge packages used
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bridges: Vec<String>,

    /// General metadata
    #[serde(flatten)]
    pub meta: Option<Meta>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_deserialization_from_docs() {
        let json = serde_json::json!({
            "name": "getUser",
            "description": "ID로 사용자 조회",
            "async": true,
            "params": {
                "ctx": {
                    "type": "RequestContext",
                    "description": "요청 컨텍스트"
                }
            },
            "returnType": { "ref": "HttpResponse" },
            "body": [
                {
                    "type": "LetStatement",
                    "tier": 0,
                    "name": "userId",
                    "value": {
                        "type": "CtxGet",
                        "tier": 1,
                        "path": "params.id"
                    }
                },
                {
                    "type": "ReturnStatement",
                    "tier": 0,
                    "value": {
                        "type": "HttpRespond",
                        "tier": 1,
                        "status": 200,
                        "body": { "type": "Identifier", "tier": 0, "name": "user" }
                    }
                }
            ],
            "meta": {
                "maxTier": 1,
                "languages": ["typescript", "rust", "python", "go"],
                "bridges": []
            }
        });

        let handler: HandlerSpec = serde_json::from_value(json).unwrap();
        assert_eq!(handler.name, "getUser");
        assert!(handler.is_async);
        assert_eq!(handler.body.len(), 2);
        assert_eq!(handler.meta.as_ref().unwrap().max_tier, Some(1));
    }
}
