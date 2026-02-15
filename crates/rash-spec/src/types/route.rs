use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::{HttpMethod, Meta, Ref};

/// Route specification (*.route.json)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RouteSpec {
    /// JSON Schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Route path (relative to basePath)
    pub path: String,

    /// Route description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Path parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<IndexMap<String, ParamSpec>>,

    /// HTTP method → endpoint mapping
    pub methods: IndexMap<HttpMethod, EndpointSpec>,

    /// Tags for grouping
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

/// Path parameter specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ParamSpec {
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Single HTTP endpoint specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EndpointSpec {
    /// Unique operation identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,

    /// Short summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Handler reference
    pub handler: Ref,

    /// Middleware chain for this endpoint
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub middleware: Vec<Ref>,

    /// Request specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestSpec>,

    /// Response specifications keyed by status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<IndexMap<String, ResponseSpec>>,
}

/// Request specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestSpec {
    /// Query parameters schema reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<Ref>,

    /// Request body specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<RequestBodySpec>,

    /// Header definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, serde_json::Value>>,
}

/// Request body specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestBodySpec {
    #[serde(rename = "ref")]
    pub reference: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Response specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResponseSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Ref>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_deserialization_from_docs() {
        let json = serde_json::json!({
            "path": "/v1/users",
            "description": "사용자 관리 API",
            "methods": {
                "GET": {
                    "operationId": "listUsers",
                    "summary": "사용자 목록 조회",
                    "handler": { "ref": "users.listUsers" },
                    "middleware": [
                        { "ref": "auth", "config": { "roles": ["admin", "user"] } }
                    ],
                    "request": {
                        "query": { "ref": "ListUsersQuery" }
                    },
                    "response": {
                        "200": {
                            "description": "성공",
                            "schema": { "ref": "UserListResponse" }
                        },
                        "401": {
                            "description": "인증 실패",
                            "schema": { "ref": "ErrorResponse" }
                        }
                    }
                },
                "POST": {
                    "operationId": "createUser",
                    "handler": { "ref": "users.createUser" },
                    "request": {
                        "body": {
                            "ref": "CreateUserBody",
                            "contentType": "application/json"
                        }
                    },
                    "response": {
                        "201": {
                            "description": "생성 완료",
                            "schema": { "ref": "UserResponse" }
                        }
                    }
                }
            },
            "tags": ["users"],
            "meta": {
                "createdAt": "2026-01-15T00:00:00Z"
            }
        });

        let route: RouteSpec = serde_json::from_value(json).unwrap();
        assert_eq!(route.path, "/v1/users");
        assert_eq!(route.methods.len(), 2);
        assert!(route.methods.contains_key(&HttpMethod::Get));
        assert!(route.methods.contains_key(&HttpMethod::Post));

        let get = &route.methods[&HttpMethod::Get];
        assert_eq!(get.operation_id.as_deref(), Some("listUsers"));
        assert_eq!(get.handler.reference, "users.listUsers");
        assert_eq!(get.middleware.len(), 1);
    }

    #[test]
    fn test_route_with_params() {
        let json = serde_json::json!({
            "path": "/v1/users/:id",
            "params": {
                "id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "사용자 UUID"
                }
            },
            "methods": {
                "GET": {
                    "operationId": "getUser",
                    "handler": { "ref": "users.getUser" },
                    "response": {
                        "200": { "schema": { "ref": "UserResponse" } },
                        "404": { "schema": { "ref": "ErrorResponse" } }
                    }
                }
            }
        });

        let route: RouteSpec = serde_json::from_value(json).unwrap();
        assert!(route.params.is_some());
        let params = route.params.unwrap();
        assert_eq!(params["id"].param_type, "string");
        assert_eq!(params["id"].format.as_deref(), Some("uuid"));
    }
}
