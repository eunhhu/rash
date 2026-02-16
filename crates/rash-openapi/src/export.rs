use std::collections::BTreeSet;

use indexmap::IndexMap;
use rash_spec::types::common::HttpMethod;
use rash_spec::types::config::RashConfig;
use rash_spec::types::middleware::MiddlewareSpec;
use rash_spec::types::route::RouteSpec;
use rash_spec::types::schema::SchemaSpec;

use crate::error::OpenApiError;
use crate::types::*;

/// Export Rash spec to an OpenAPI 3.1 document.
pub fn export_openapi(
    config: &RashConfig,
    routes: &[RouteSpec],
    schemas: &[SchemaSpec],
    middleware: &[MiddlewareSpec],
) -> Result<OpenApiDocument, OpenApiError> {
    let info = build_info(config);
    let servers = build_servers(config);
    let (paths, collected_tags) = build_paths(routes, middleware)?;
    let components = build_components(schemas, middleware);
    let tags = build_tags(&collected_tags);

    // Global security: if any auth middleware exists, apply globally
    let security = if components.security_schemes.is_empty() {
        vec![]
    } else {
        components
            .security_schemes
            .keys()
            .map(|name| {
                let mut m = IndexMap::new();
                m.insert(name.clone(), vec![]);
                m
            })
            .collect()
    };

    let components = if components.schemas.is_empty() && components.security_schemes.is_empty() {
        None
    } else {
        Some(components)
    };

    Ok(OpenApiDocument {
        openapi: "3.1.0".to_string(),
        info,
        servers,
        paths,
        components,
        tags,
        security,
    })
}

fn build_info(config: &RashConfig) -> InfoObject {
    InfoObject {
        title: config.name.clone(),
        version: config.version.clone(),
        description: config.description.clone(),
    }
}

fn build_servers(config: &RashConfig) -> Vec<ServerObject> {
    let protocol = config
        .server
        .protocol
        .map(|p| match p {
            rash_spec::types::common::Protocol::Http => "http",
            rash_spec::types::common::Protocol::Https => "https",
        })
        .unwrap_or("http");

    let base_path = config.server.base_path.as_deref().unwrap_or("");
    let url = format!(
        "{}://{}:{}{}",
        protocol, config.server.host, config.server.port, base_path
    );

    vec![ServerObject {
        url,
        description: None,
    }]
}

/// Convert `:param` style path to `{param}` style for OpenAPI.
fn convert_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if let Some(param) = segment.strip_prefix(':') {
                format!("{{{param}}}")
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn build_paths(
    routes: &[RouteSpec],
    middleware: &[MiddlewareSpec],
) -> Result<(IndexMap<String, PathItemObject>, BTreeSet<String>), OpenApiError> {
    let mut paths: IndexMap<String, PathItemObject> = IndexMap::new();
    let mut all_tags = BTreeSet::new();

    for route in routes {
        let openapi_path = convert_path(&route.path);

        // Path-level parameters from route.params
        let path_params = build_path_params(route);

        let path_item = paths.entry(openapi_path).or_default();
        path_item.parameters = path_params;

        for (method, endpoint) in &route.methods {
            let mut op_tags = route.tags.clone();
            for tag in &op_tags {
                all_tags.insert(tag.clone());
            }

            // Query parameters
            let mut parameters = Vec::new();
            if let Some(ref req) = endpoint.request {
                if let Some(ref query_ref) = req.query {
                    parameters.push(ParameterObject {
                        name: query_ref.reference.clone(),
                        location: "query".to_string(),
                        required: Some(false),
                        description: None,
                        schema: Some(serde_json::json!({
                            "$ref": format!("#/components/schemas/{}", query_ref.reference)
                        })),
                    });
                }
            }

            // Request body
            let request_body = endpoint
                .request
                .as_ref()
                .and_then(|req| req.body.as_ref())
                .map(|body| {
                    let content_type = body
                        .content_type
                        .as_deref()
                        .unwrap_or("application/json");
                    let mut content = IndexMap::new();
                    content.insert(
                        content_type.to_string(),
                        MediaTypeObject {
                            schema: Some(serde_json::json!({
                                "$ref": format!("#/components/schemas/{}", body.reference)
                            })),
                        },
                    );
                    RequestBodyObject {
                        required: Some(true),
                        content,
                    }
                });

            // Responses
            let mut responses = IndexMap::new();
            if let Some(ref resp_map) = endpoint.response {
                for (status, resp) in resp_map {
                    let description = resp
                        .description
                        .clone()
                        .unwrap_or_else(|| default_status_description(status));
                    let content = resp.schema.as_ref().map(|schema_ref| {
                        let mut m = IndexMap::new();
                        m.insert(
                            "application/json".to_string(),
                            MediaTypeObject {
                                schema: Some(serde_json::json!({
                                    "$ref": format!("#/components/schemas/{}", schema_ref.reference)
                                })),
                            },
                        );
                        m
                    });
                    responses.insert(status.clone(), ResponseObject {
                        description,
                        content,
                    });
                }
            }
            if responses.is_empty() {
                responses.insert(
                    "200".to_string(),
                    ResponseObject {
                        description: "Successful response".to_string(),
                        content: None,
                    },
                );
            }

            // Security from endpoint middleware — use middleware name as scheme key
            let security: Vec<IndexMap<String, Vec<String>>> = endpoint
                .middleware
                .iter()
                .filter(|mw| middleware.iter().any(|m| m.name == mw.reference && is_security_middleware(m)))
                .map(|mw| {
                    let mut m = IndexMap::new();
                    m.insert(mw.reference.clone(), vec![]);
                    m
                })
                .collect();

            if op_tags.is_empty() {
                // Derive tag from path first segment
                if let Some(tag) = derive_tag_from_path(&route.path) {
                    op_tags.push(tag.clone());
                    all_tags.insert(tag);
                }
            }

            let operation = OperationObject {
                operation_id: endpoint.operation_id.clone(),
                summary: endpoint.summary.clone(),
                description: None,
                tags: op_tags,
                parameters,
                request_body,
                responses,
                security,
            };

            match method {
                HttpMethod::Get => path_item.get = Some(operation),
                HttpMethod::Post => path_item.post = Some(operation),
                HttpMethod::Put => path_item.put = Some(operation),
                HttpMethod::Patch => path_item.patch = Some(operation),
                HttpMethod::Delete => path_item.delete = Some(operation),
                HttpMethod::Head => path_item.head = Some(operation),
                HttpMethod::Options => path_item.options = Some(operation),
            }
        }
    }

    Ok((paths, all_tags))
}

fn build_path_params(route: &RouteSpec) -> Vec<ParameterObject> {
    let Some(ref params) = route.params else {
        return vec![];
    };

    params
        .iter()
        .map(|(name, spec)| {
            let schema = serde_json::json!({
                "type": spec.param_type,
            });
            ParameterObject {
                name: name.clone(),
                location: "path".to_string(),
                required: Some(true),
                description: spec.description.clone(),
                schema: Some(schema),
            }
        })
        .collect()
}

/// Normalize internal `{ "ref": "Name" }` back to standard `{ "$ref": "#/components/schemas/Name" }`.
fn normalize_refs(val: &serde_json::Value) -> serde_json::Value {
    match val {
        serde_json::Value::Object(map) => {
            // Internal ref → standard $ref
            if map.len() == 1 {
                if let Some(ref_val) = map.get("ref") {
                    if let Some(ref_str) = ref_val.as_str() {
                        return serde_json::json!({
                            "$ref": format!("#/components/schemas/{ref_str}")
                        });
                    }
                }
            }
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k.clone(), normalize_refs(v));
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_refs).collect())
        }
        other => other.clone(),
    }
}

fn build_components(schemas: &[SchemaSpec], middleware: &[MiddlewareSpec]) -> ComponentsObject {
    let mut comp_schemas = IndexMap::new();

    for schema in schemas {
        for (def_name, def_value) in &schema.definitions {
            comp_schemas.insert(def_name.clone(), normalize_refs(def_value));
        }
    }

    let mut security_schemes = IndexMap::new();
    for mw in middleware {
        if let Some(scheme) = derive_security_scheme(mw) {
            security_schemes.insert(mw.name.clone(), scheme);
        }
    }

    ComponentsObject {
        schemas: comp_schemas,
        security_schemes,
    }
}

/// Check if a middleware spec represents a security middleware.
/// Looks for config properties that indicate bearer/apiKey scheme.
fn is_security_middleware(mw: &MiddlewareSpec) -> bool {
    if let Some(ref config) = mw.config {
        // Check for bearerFormat (bearer auth) or "in"+"name" (apiKey auth)
        let has_bearer = config.pointer("/properties/bearerFormat").is_some();
        let has_api_key = config.pointer("/properties/in").is_some()
            && config.pointer("/properties/name").is_some();
        return has_bearer || has_api_key;
    }
    // Fallback: if the middleware has errors with "UNAUTHORIZED" key
    if let Some(ref errors) = mw.errors {
        return errors.contains_key("UNAUTHORIZED");
    }
    false
}

/// Derive an OpenAPI SecuritySchemeObject from a middleware spec.
fn derive_security_scheme(mw: &MiddlewareSpec) -> Option<SecuritySchemeObject> {
    if let Some(ref config) = mw.config {
        // Bearer auth
        if let Some(bearer_format_obj) = config.pointer("/properties/bearerFormat") {
            let bearer_format = bearer_format_obj
                .get("default")
                .and_then(|v| v.as_str())
                .unwrap_or("JWT")
                .to_string();
            return Some(SecuritySchemeObject {
                scheme_type: "http".to_string(),
                scheme: Some("bearer".to_string()),
                bearer_format: Some(bearer_format),
                description: mw.description.clone(),
            });
        }

        // API Key auth
        if config.pointer("/properties/in").is_some() {
            return Some(SecuritySchemeObject {
                scheme_type: "apiKey".to_string(),
                scheme: None,
                bearer_format: None,
                description: mw.description.clone(),
            });
        }
    }

    // Fallback: middleware with UNAUTHORIZED errors and a handler → treat as bearer auth
    if let Some(ref errors) = mw.errors {
        if errors.contains_key("UNAUTHORIZED") && mw.handler.is_some() {
            return Some(SecuritySchemeObject {
                scheme_type: "http".to_string(),
                scheme: Some("bearer".to_string()),
                bearer_format: Some("JWT".to_string()),
                description: mw.description.clone(),
            });
        }
    }

    None
}

fn build_tags(tags: &BTreeSet<String>) -> Vec<TagObject> {
    tags.iter()
        .map(|name| TagObject {
            name: name.clone(),
            description: None,
        })
        .collect()
}

fn derive_tag_from_path(path: &str) -> Option<String> {
    path.split('/')
        .find(|s| !s.is_empty() && !s.starts_with(':') && !s.starts_with("v"))
        .map(|s| s.to_string())
}

fn default_status_description(status: &str) -> String {
    match status {
        "200" => "OK".to_string(),
        "201" => "Created".to_string(),
        "204" => "No Content".to_string(),
        "400" => "Bad Request".to_string(),
        "401" => "Unauthorized".to_string(),
        "403" => "Forbidden".to_string(),
        "404" => "Not Found".to_string(),
        "409" => "Conflict".to_string(),
        "422" => "Unprocessable Entity".to_string(),
        "500" => "Internal Server Error".to_string(),
        _ => format!("Response {status}"),
    }
}

#[cfg(test)]
mod tests {
    use rash_spec::types::common::{Framework, Language, Protocol, Ref, Runtime};
    use rash_spec::types::config::{RashConfig, ServerConfig, TargetConfig};
    use rash_spec::types::middleware::{MiddlewareSpec, MiddlewareType};
    use rash_spec::types::route::{
        EndpointSpec, ParamSpec, RequestBodySpec, RequestSpec, ResponseSpec, RouteSpec,
    };
    use rash_spec::types::schema::SchemaSpec;

    use super::*;

    fn make_config() -> RashConfig {
        RashConfig {
            schema: None,
            version: "1.0.0".to_string(),
            name: "test-api".to_string(),
            description: Some("Test API".to_string()),
            target: TargetConfig {
                language: Language::Typescript,
                framework: Framework::Express,
                runtime: Runtime::Bun,
            },
            server: ServerConfig {
                port: 3000,
                host: "0.0.0.0".to_string(),
                protocol: Some(Protocol::Http),
                base_path: Some("/api".to_string()),
            },
            database: None,
            codegen: None,
            middleware: None,
            plugins: vec![],
            meta: None,
        }
    }

    fn make_route() -> RouteSpec {
        let mut methods = IndexMap::new();
        methods.insert(
            HttpMethod::Get,
            EndpointSpec {
                operation_id: Some("listUsers".to_string()),
                summary: Some("List all users".to_string()),
                handler: Ref {
                    reference: "users.listUsers".to_string(),
                    config: None,
                },
                middleware: vec![Ref {
                    reference: "auth".to_string(),
                    config: None,
                }],
                request: Some(RequestSpec {
                    query: Some(Ref {
                        reference: "ListUsersQuery".to_string(),
                        config: None,
                    }),
                    body: None,
                    headers: None,
                }),
                response: Some({
                    let mut m = IndexMap::new();
                    m.insert(
                        "200".to_string(),
                        ResponseSpec {
                            description: Some("Success".to_string()),
                            schema: Some(Ref {
                                reference: "UserListResponse".to_string(),
                                config: None,
                            }),
                        },
                    );
                    m.insert(
                        "401".to_string(),
                        ResponseSpec {
                            description: None,
                            schema: Some(Ref {
                                reference: "ErrorResponse".to_string(),
                                config: None,
                            }),
                        },
                    );
                    m
                }),
            },
        );
        methods.insert(
            HttpMethod::Post,
            EndpointSpec {
                operation_id: Some("createUser".to_string()),
                summary: None,
                handler: Ref {
                    reference: "users.createUser".to_string(),
                    config: None,
                },
                middleware: vec![],
                request: Some(RequestSpec {
                    query: None,
                    body: Some(RequestBodySpec {
                        reference: "CreateUserBody".to_string(),
                        content_type: Some("application/json".to_string()),
                    }),
                    headers: None,
                }),
                response: Some({
                    let mut m = IndexMap::new();
                    m.insert(
                        "201".to_string(),
                        ResponseSpec {
                            description: Some("Created".to_string()),
                            schema: Some(Ref {
                                reference: "UserResponse".to_string(),
                                config: None,
                            }),
                        },
                    );
                    m
                }),
            },
        );

        RouteSpec {
            schema: None,
            path: "/v1/users".to_string(),
            description: Some("User management".to_string()),
            params: None,
            methods,
            tags: vec!["users".to_string()],
            meta: None,
        }
    }

    fn make_schema() -> SchemaSpec {
        let mut definitions = IndexMap::new();
        definitions.insert(
            "CreateUserBody".to_string(),
            serde_json::json!({
                "type": "object",
                "required": ["email", "password"],
                "properties": {
                    "email": { "type": "string", "format": "email" },
                    "password": { "type": "string", "minLength": 8 }
                }
            }),
        );
        definitions.insert(
            "UserResponse".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "email": { "type": "string" }
                }
            }),
        );

        SchemaSpec {
            schema: None,
            name: "User".to_string(),
            description: None,
            definitions,
            meta: None,
        }
    }

    fn make_auth_middleware() -> MiddlewareSpec {
        MiddlewareSpec {
            schema: None,
            name: "auth".to_string(),
            description: Some("JWT authentication".to_string()),
            middleware_type: MiddlewareType::Request,
            config: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "bearerFormat": {
                        "type": "string",
                        "default": "JWT"
                    }
                }
            })),
            handler: Some(Ref {
                reference: "auth.verifyToken".to_string(),
                config: None,
            }),
            provides: None,
            errors: Some({
                let mut m = IndexMap::new();
                m.insert(
                    "UNAUTHORIZED".to_string(),
                    rash_spec::types::middleware::MiddlewareError {
                        status: 401,
                        message: "Invalid or missing token".to_string(),
                    },
                );
                m
            }),
            compose: None,
            short_circuit: None,
            meta: None,
        }
    }

    #[test]
    fn test_convert_path() {
        assert_eq!(convert_path("/users/:id"), "/users/{id}");
        assert_eq!(
            convert_path("/users/:userId/posts/:postId"),
            "/users/{userId}/posts/{postId}"
        );
        assert_eq!(convert_path("/users"), "/users");
    }

    #[test]
    fn test_basic_export() {
        let config = make_config();
        let routes = vec![make_route()];
        let schemas = vec![make_schema()];
        let middleware = vec![make_auth_middleware()];

        let doc = export_openapi(&config, &routes, &schemas, &middleware).unwrap();

        assert_eq!(doc.openapi, "3.1.0");
        assert_eq!(doc.info.title, "test-api");
        assert_eq!(doc.info.version, "1.0.0");
        assert_eq!(doc.info.description.as_deref(), Some("Test API"));
    }

    #[test]
    fn test_servers() {
        let config = make_config();
        let doc = export_openapi(&config, &[], &[], &[]).unwrap();

        assert_eq!(doc.servers.len(), 1);
        assert_eq!(doc.servers[0].url, "http://0.0.0.0:3000/api");
    }

    #[test]
    fn test_paths_conversion() {
        let config = make_config();
        let routes = vec![make_route()];
        let doc = export_openapi(&config, &routes, &[], &[]).unwrap();

        assert!(doc.paths.contains_key("/v1/users"));
        let path_item = &doc.paths["/v1/users"];

        // GET operation
        let get_op = path_item.get.as_ref().unwrap();
        assert_eq!(get_op.operation_id.as_deref(), Some("listUsers"));
        assert_eq!(get_op.summary.as_deref(), Some("List all users"));
        assert_eq!(get_op.tags, vec!["users"]);
        assert!(get_op.responses.contains_key("200"));
        assert!(get_op.responses.contains_key("401"));

        // POST operation
        let post_op = path_item.post.as_ref().unwrap();
        assert_eq!(post_op.operation_id.as_deref(), Some("createUser"));
        assert!(post_op.request_body.is_some());
        let body = post_op.request_body.as_ref().unwrap();
        assert!(body.content.contains_key("application/json"));
    }

    #[test]
    fn test_path_params() {
        let config = make_config();
        let mut methods = IndexMap::new();
        methods.insert(
            HttpMethod::Get,
            EndpointSpec {
                operation_id: Some("getUser".to_string()),
                summary: None,
                handler: Ref {
                    reference: "users.getUser".to_string(),
                    config: None,
                },
                middleware: vec![],
                request: None,
                response: None,
            },
        );

        let mut params = IndexMap::new();
        params.insert(
            "id".to_string(),
            ParamSpec {
                param_type: "string".to_string(),
                format: Some("uuid".to_string()),
                description: Some("User ID".to_string()),
            },
        );

        let route = RouteSpec {
            schema: None,
            path: "/users/:id".to_string(),
            description: None,
            params: Some(params),
            methods,
            tags: vec![],
            meta: None,
        };

        let doc = export_openapi(&config, &[route], &[], &[]).unwrap();

        assert!(doc.paths.contains_key("/users/{id}"));
        let path_item = &doc.paths["/users/{id}"];
        assert_eq!(path_item.parameters.len(), 1);
        assert_eq!(path_item.parameters[0].name, "id");
        assert_eq!(path_item.parameters[0].location, "path");
        assert_eq!(path_item.parameters[0].required, Some(true));
        assert_eq!(
            path_item.parameters[0].description.as_deref(),
            Some("User ID")
        );
    }

    #[test]
    fn test_schemas_to_components() {
        let config = make_config();
        let schemas = vec![make_schema()];
        let doc = export_openapi(&config, &[], &schemas, &[]).unwrap();

        let components = doc.components.unwrap();
        assert!(components.schemas.contains_key("CreateUserBody"));
        assert!(components.schemas.contains_key("UserResponse"));
        assert_eq!(components.schemas.len(), 2);
    }

    #[test]
    fn test_auth_middleware_to_security_scheme() {
        let config = make_config();
        let middleware = vec![make_auth_middleware()];
        let doc = export_openapi(&config, &[], &[], &middleware).unwrap();

        let components = doc.components.unwrap();
        assert!(components.security_schemes.contains_key("auth"));
        let scheme = &components.security_schemes["auth"];
        assert_eq!(scheme.scheme_type, "http");
        assert_eq!(scheme.scheme.as_deref(), Some("bearer"));
        assert_eq!(scheme.bearer_format.as_deref(), Some("JWT"));
    }

    #[test]
    fn test_endpoint_security() {
        let config = make_config();
        let routes = vec![make_route()];
        let middleware = vec![make_auth_middleware()];
        let doc = export_openapi(&config, &routes, &[], &middleware).unwrap();

        let get_op = doc.paths["/v1/users"].get.as_ref().unwrap();
        assert_eq!(get_op.security.len(), 1);
        assert!(get_op.security[0].contains_key("auth"));

        // POST has no auth middleware
        let post_op = doc.paths["/v1/users"].post.as_ref().unwrap();
        assert!(post_op.security.is_empty());
    }

    #[test]
    fn test_tags_collected() {
        let config = make_config();
        let routes = vec![make_route()];
        let doc = export_openapi(&config, &routes, &[], &[]).unwrap();

        assert_eq!(doc.tags.len(), 1);
        assert_eq!(doc.tags[0].name, "users");
    }

    #[test]
    fn test_full_export_serializes_to_json() {
        let config = make_config();
        let routes = vec![make_route()];
        let schemas = vec![make_schema()];
        let middleware = vec![make_auth_middleware()];

        let doc = export_openapi(&config, &routes, &schemas, &middleware).unwrap();
        let json = serde_json::to_string_pretty(&doc).unwrap();

        assert!(json.contains("\"openapi\": \"3.1.0\""));
        assert!(json.contains("\"title\": \"test-api\""));
        assert!(json.contains("/v1/users"));
        assert!(json.contains("\"auth\""));
        assert!(json.contains("CreateUserBody"));

        // Verify it round-trips
        let parsed: OpenApiDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.openapi, "3.1.0");
        assert_eq!(parsed.info.title, "test-api");
    }

    #[test]
    fn test_empty_project() {
        let config = make_config();
        let doc = export_openapi(&config, &[], &[], &[]).unwrap();

        assert_eq!(doc.openapi, "3.1.0");
        assert!(doc.paths.is_empty());
        assert!(doc.components.is_none());
        assert!(doc.tags.is_empty());
        assert!(doc.security.is_empty());
    }

    #[test]
    fn test_default_status_descriptions() {
        assert_eq!(default_status_description("200"), "OK");
        assert_eq!(default_status_description("201"), "Created");
        assert_eq!(default_status_description("404"), "Not Found");
        assert_eq!(default_status_description("999"), "Response 999");
    }
}
