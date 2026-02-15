use std::collections::HashSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use rash_spec::types::common::{HttpMethod, Language, Tier};

use crate::expr::TypeIR;
use crate::statement::StatementIR;

/// Project-level IR — the root of the entire intermediate representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIR {
    /// Project configuration (kept as opaque JSON for now)
    pub config: serde_json::Value,
    /// All route definitions
    pub routes: Vec<RouteIR>,
    /// All schema (DTO) definitions
    pub schemas: Vec<SchemaIR>,
    /// All database model definitions
    pub models: Vec<ModelIR>,
    /// All middleware definitions
    pub middleware: Vec<MiddlewareIR>,
    /// All handler definitions
    pub handlers: Vec<HandlerIR>,
}

/// A single route and its HTTP method endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteIR {
    /// URL path (e.g., "/v1/users/:id")
    pub path: String,
    /// HTTP method -> endpoint mapping (ordered)
    pub methods: IndexMap<HttpMethod, EndpointIR>,
    /// Tags for grouping/documentation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// A single HTTP endpoint within a route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointIR {
    /// Unique operation identifier (e.g., "getUser")
    pub operation_id: String,
    /// Short human-readable summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Reference to the handler that processes this endpoint
    pub handler_ref: String,
    /// Ordered middleware chain applied before the handler
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub middleware: Vec<String>,
    /// Request shape
    pub request: RequestIR,
    /// Response shapes keyed by HTTP status code
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub response: IndexMap<u16, ResponseIR>,
}

/// Describes the expected request shape for an endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestIR {
    /// Reference to a schema used for query parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_schema: Option<String>,
    /// Reference to a schema used for the request body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_schema: Option<String>,
    /// Content type of the request body (e.g., "application/json")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// Describes a single response variant for an endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseIR {
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Reference to a schema describing the response body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
}

/// A named schema (DTO) definition group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaIR {
    /// Schema group name (e.g., "User")
    pub name: String,
    /// Individual definitions within this schema (e.g., "CreateUserBody", "UserResponse")
    pub definitions: IndexMap<String, serde_json::Value>,
}

/// A database model definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelIR {
    /// Model name (e.g., "User")
    pub name: String,
    /// Database table name
    pub table_name: String,
    /// Column definitions
    pub columns: IndexMap<String, serde_json::Value>,
    /// Relation definitions
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub relations: IndexMap<String, serde_json::Value>,
    /// Index definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<serde_json::Value>,
}

/// A middleware definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareIR {
    /// Middleware name (e.g., "auth")
    pub name: String,
    /// Middleware type (request, response, error, composed)
    pub middleware_type: String,
    /// Reference to the handler implementing this middleware
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler_ref: Option<String>,
}

/// A handler function definition with its full AST body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerIR {
    /// Handler name (e.g., "getUser")
    pub name: String,
    /// Whether this handler is async
    pub is_async: bool,
    /// Input parameters
    pub params: Vec<ParamIR>,
    /// Return type
    pub return_type: TypeIR,
    /// Handler body as a sequence of IR statements
    pub body: Vec<StatementIR>,
    /// Maximum tier of any node in this handler
    pub max_tier: Tier,
    /// Set of languages this handler is locked to (non-empty only for Bridge tier)
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub bridge_languages: HashSet<Language>,
}

/// A named, typed parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamIR {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub type_ir: TypeIR,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_ir_serialization_roundtrip() {
        let project = ProjectIR {
            config: serde_json::json!({ "name": "test-project" }),
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        };

        let json = serde_json::to_value(&project).unwrap();
        let deserialized: ProjectIR = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.routes.len(), 0);
    }

    #[test]
    fn test_route_ir_with_endpoints() {
        let route = RouteIR {
            path: "/v1/users".to_string(),
            methods: {
                let mut m = IndexMap::new();
                m.insert(
                    HttpMethod::Get,
                    EndpointIR {
                        operation_id: "listUsers".to_string(),
                        summary: Some("List all users".to_string()),
                        handler_ref: "users.listUsers".to_string(),
                        middleware: vec!["auth".to_string()],
                        request: RequestIR {
                            query_schema: Some("ListUsersQuery".to_string()),
                            body_schema: None,
                            content_type: None,
                        },
                        response: {
                            let mut r = IndexMap::new();
                            r.insert(
                                200,
                                ResponseIR {
                                    description: Some("Success".to_string()),
                                    schema_ref: Some("UserListResponse".to_string()),
                                },
                            );
                            r
                        },
                    },
                );
                m
            },
            tags: vec!["users".to_string()],
        };

        let json = serde_json::to_value(&route).unwrap();
        let deserialized: RouteIR = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.path, "/v1/users");
        assert_eq!(deserialized.methods.len(), 1);
        assert_eq!(deserialized.tags, vec!["users"]);
    }

    #[test]
    fn test_handler_ir_with_body() {
        let handler = HandlerIR {
            name: "getUser".to_string(),
            is_async: true,
            params: vec![ParamIR {
                name: "ctx".to_string(),
                type_ir: TypeIR::Ref("RequestContext".to_string()),
            }],
            return_type: TypeIR::Ref("HttpResponse".to_string()),
            body: vec![StatementIR::Return { value: None }],
            max_tier: Tier::Domain,
            bridge_languages: HashSet::new(),
        };

        let json = serde_json::to_value(&handler).unwrap();
        let deserialized: HandlerIR = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.name, "getUser");
        assert!(deserialized.is_async);
        assert_eq!(deserialized.params.len(), 1);
        assert_eq!(deserialized.body.len(), 1);
    }

    #[test]
    fn test_schema_ir() {
        let schema = SchemaIR {
            name: "User".to_string(),
            definitions: {
                let mut d = IndexMap::new();
                d.insert(
                    "CreateUserBody".to_string(),
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "email": { "type": "string" }
                        }
                    }),
                );
                d
            },
        };

        let json = serde_json::to_value(&schema).unwrap();
        let deserialized: SchemaIR = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.name, "User");
        assert_eq!(deserialized.definitions.len(), 1);
    }

    #[test]
    fn test_model_ir() {
        let model = ModelIR {
            name: "User".to_string(),
            table_name: "users".to_string(),
            columns: {
                let mut c = IndexMap::new();
                c.insert("id".to_string(), serde_json::json!({ "type": "uuid", "primaryKey": true }));
                c
            },
            relations: IndexMap::new(),
            indexes: vec![],
        };

        let json = serde_json::to_value(&model).unwrap();
        let deserialized: ModelIR = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.name, "User");
        assert_eq!(deserialized.table_name, "users");
    }

    #[test]
    fn test_middleware_ir() {
        let mw = MiddlewareIR {
            name: "auth".to_string(),
            middleware_type: "request".to_string(),
            handler_ref: Some("auth.verifyToken".to_string()),
        };

        let json = serde_json::to_value(&mw).unwrap();
        let deserialized: MiddlewareIR = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.name, "auth");
        assert_eq!(deserialized.handler_ref.as_deref(), Some("auth.verifyToken"));
    }
}
