use indexmap::IndexMap;
use serde_json::Value;

use rash_spec::types::common::{HttpMethod, Language, Ref, TypeRef};
use rash_spec::types::config::{RashConfig, ServerConfig, TargetConfig};
use rash_spec::types::handler::{HandlerMeta, HandlerParam, HandlerSpec};
use rash_spec::types::middleware::{MiddlewareSpec, MiddlewareType};
use rash_spec::types::route::{
    EndpointSpec, ParamSpec, RequestBodySpec, RequestSpec, ResponseSpec, RouteSpec,
};
use rash_spec::types::schema::SchemaSpec;

use crate::error::OpenApiError;

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub config: RashConfig,
    pub routes: Vec<RouteSpec>,
    pub schemas: Vec<SchemaSpec>,
    pub middleware: Vec<MiddlewareSpec>,
    pub handlers: Vec<HandlerSpec>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Import an OpenAPI 3.x document (JSON or YAML) and convert it to Rash spec types.
pub fn import_openapi(openapi_json: &str) -> Result<ImportResult, OpenApiError> {
    // Try JSON first, fall back to YAML
    let doc: Value = match serde_json::from_str(openapi_json) {
        Ok(v) => v,
        Err(json_err) => serde_yaml::from_str::<Value>(openapi_json).map_err(|yaml_err| {
            OpenApiError::ParseError(format!(
                "failed to parse as JSON ({json_err}) or YAML ({yaml_err})"
            ))
        })?,
    };

    // Validate version
    let version = doc
        .get("openapi")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OpenApiError::MissingField("openapi".into()))?;

    if !version.starts_with("3.") {
        return Err(OpenApiError::UnsupportedVersion(version.into()));
    }

    let mut warnings: Vec<String> = Vec::new();

    let config = build_config(&doc, &mut warnings);
    let schemas = build_schemas(&doc, &mut warnings);
    let middleware = build_middleware(&doc, &mut warnings);
    let (routes, handlers, extra_schemas) = build_routes_and_handlers(&doc, &middleware, &mut warnings);

    // Merge synthetic schemas into the main list
    let mut schemas = schemas;
    schemas.extend(extra_schemas);

    Ok(ImportResult {
        config,
        routes,
        schemas,
        middleware,
        handlers,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Config builder
// ---------------------------------------------------------------------------

fn build_config(doc: &Value, warnings: &mut Vec<String>) -> RashConfig {
    let info = doc.get("info").cloned().unwrap_or(Value::Object(Default::default()));

    let name = info
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("imported-api")
        .to_string();

    let api_version = info
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0.0")
        .to_string();

    let description = info.get("description").and_then(|v| v.as_str()).map(String::from);

    let (host, port, base_path) = parse_server_url(doc, warnings);

    RashConfig {
        schema: None,
        version: api_version,
        name,
        description,
        target: TargetConfig {
            language: Language::Typescript,
            framework: rash_spec::types::common::Framework::Express,
            runtime: rash_spec::types::common::Runtime::Bun,
        },
        server: ServerConfig {
            port,
            host,
            protocol: None,
            base_path,
        },
        database: None,
        codegen: None,
        middleware: None,
        plugins: Vec::new(),
        meta: None,
    }
}

fn parse_server_url(doc: &Value, warnings: &mut Vec<String>) -> (String, u16, Option<String>) {
    let default = ("0.0.0.0".to_string(), 3000u16, None);

    let url_str = match doc
        .get("servers")
        .and_then(|s| s.as_array())
        .and_then(|a| a.first())
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
    {
        Some(u) => u,
        None => return default,
    };

    // Try to parse as a URL. Handles `http://localhost:8080/api/v1` style.
    // Also handles relative paths like `/api/v1`.
    if url_str.starts_with('/') {
        return (
            default.0,
            default.1,
            Some(url_str.trim_end_matches('/').to_string()),
        );
    }

    // Strip scheme
    let without_scheme = url_str
        .strip_prefix("https://")
        .or_else(|| url_str.strip_prefix("http://"))
        .unwrap_or(url_str);

    // Split host+port from path
    let (host_port, path) = match without_scheme.find('/') {
        Some(idx) => (&without_scheme[..idx], Some(&without_scheme[idx..])),
        None => (without_scheme, None),
    };

    let (host, port) = match host_port.rfind(':') {
        Some(idx) => {
            let h = &host_port[..idx];
            let p = host_port[idx + 1..].parse::<u16>().unwrap_or_else(|_| {
                warnings.push(format!("could not parse port from server URL: {url_str}"));
                3000
            });
            (h.to_string(), p)
        }
        None => (host_port.to_string(), 3000),
    };

    let base_path = path
        .filter(|p| *p != "/")
        .map(|p| p.trim_end_matches('/').to_string());

    (host, port, base_path)
}

// ---------------------------------------------------------------------------
// Schema builder
// ---------------------------------------------------------------------------

fn build_schemas(doc: &Value, warnings: &mut Vec<String>) -> Vec<SchemaSpec> {
    let schemas_map = match doc
        .pointer("/components/schemas")
        .and_then(|v| v.as_object())
    {
        Some(m) => m,
        None => return Vec::new(),
    };

    // Group schemas by common prefix (e.g. User, Post).
    // A schema named "CreateUserBody" → prefix "User".
    // If no prefix match, each schema becomes its own SchemaSpec.

    let mut groups: IndexMap<String, IndexMap<String, Value>> = IndexMap::new();

    for (name, schema_val) in schemas_map {
        let resolved = resolve_schema_refs(schema_val.clone());
        let group_name = infer_group_name(name);
        groups
            .entry(group_name)
            .or_default()
            .insert(name.clone(), resolved);
    }

    let mut result = Vec::new();
    for (group_name, definitions) in groups {
        if definitions.is_empty() {
            continue;
        }
        let desc = if definitions.len() == 1 {
            None
        } else {
            Some(format!("{group_name}-related schemas"))
        };
        result.push(SchemaSpec {
            schema: None,
            name: group_name,
            description: desc,
            definitions,
            meta: None,
        });
    }

    if result.is_empty() {
        warnings.push("no schemas found in components/schemas".into());
    }

    result
}

/// Infer a group name from a schema name.
/// "CreateUserBody" → "User", "UserResponse" → "User", "Pet" → "Pet"
fn infer_group_name(name: &str) -> String {
    let prefixes_to_strip = [
        "Create", "Update", "Delete", "List", "Get", "Patch", "New",
    ];
    let suffixes_to_strip = [
        "Request", "Response", "Body", "Input", "Output", "Dto", "DTO",
        "Params", "Query", "List", "Item", "Detail", "Summary",
    ];

    let mut core = name.to_string();

    for prefix in &prefixes_to_strip {
        if let Some(rest) = core.strip_prefix(prefix) {
            if !rest.is_empty() && rest.chars().next().is_some_and(|c| c.is_uppercase()) {
                core = rest.to_string();
                break;
            }
        }
    }

    for suffix in &suffixes_to_strip {
        if let Some(rest) = core.strip_suffix(suffix) {
            if !rest.is_empty() {
                core = rest.to_string();
                break;
            }
        }
    }

    core
}

/// Resolve `$ref` pointers in a schema value to inline `{ "ref": "Name" }`.
fn resolve_schema_refs(val: Value) -> Value {
    match val {
        Value::Object(map) => {
            // Direct $ref
            if let Some(ref_val) = map.get("$ref") {
                if let Some(ref_str) = ref_val.as_str() {
                    let resolved_name = resolve_ref_string(ref_str);
                    return serde_json::json!({ "ref": resolved_name });
                }
            }

            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(k, resolve_schema_refs(v));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(resolve_schema_refs).collect()),
        other => other,
    }
}

/// Extract the type name from a `$ref` string.
/// `#/components/schemas/UserResponse` → `UserResponse`
fn resolve_ref_string(ref_str: &str) -> String {
    ref_str
        .rsplit('/')
        .next()
        .unwrap_or(ref_str)
        .to_string()
}

// ---------------------------------------------------------------------------
// Middleware builder (from securitySchemes)
// ---------------------------------------------------------------------------

fn build_middleware(doc: &Value, warnings: &mut Vec<String>) -> Vec<MiddlewareSpec> {
    let schemes = match doc
        .pointer("/components/securitySchemes")
        .and_then(|v| v.as_object())
    {
        Some(m) => m,
        None => return Vec::new(),
    };

    let mut result = Vec::new();

    for (name, scheme) in schemes {
        let scheme_type = scheme.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let scheme_scheme = scheme.get("scheme").and_then(|s| s.as_str()).unwrap_or("");

        match (scheme_type, scheme_scheme) {
            ("http", "bearer") => {
                result.push(MiddlewareSpec {
                    schema: None,
                    name: name.clone(),
                    description: scheme
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(String::from),
                    middleware_type: MiddlewareType::Request,
                    config: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "bearerFormat": {
                                "type": "string",
                                "default": scheme.get("bearerFormat")
                                    .and_then(|b| b.as_str())
                                    .unwrap_or("JWT")
                            }
                        }
                    })),
                    handler: Some(Ref {
                        reference: format!("{name}.verifyToken"),
                        config: None,
                    }),
                    provides: Some({
                        let mut m = IndexMap::new();
                        m.insert(
                            "user".to_string(),
                            serde_json::json!({
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string" },
                                    "role": { "type": "string" }
                                }
                            }),
                        );
                        m
                    }),
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
                });
            }
            ("apiKey", _) => {
                let in_location = scheme.get("in").and_then(|i| i.as_str()).unwrap_or("header");
                let key_name = scheme
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("X-API-Key");
                result.push(MiddlewareSpec {
                    schema: None,
                    name: name.clone(),
                    description: scheme
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(String::from),
                    middleware_type: MiddlewareType::Request,
                    config: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "in": { "type": "string", "default": in_location },
                            "name": { "type": "string", "default": key_name }
                        }
                    })),
                    handler: Some(Ref {
                        reference: format!("{name}.verifyApiKey"),
                        config: None,
                    }),
                    provides: None,
                    errors: Some({
                        let mut m = IndexMap::new();
                        m.insert(
                            "UNAUTHORIZED".to_string(),
                            rash_spec::types::middleware::MiddlewareError {
                                status: 401,
                                message: "Invalid API key".to_string(),
                            },
                        );
                        m
                    }),
                    compose: None,
                    short_circuit: None,
                    meta: None,
                });
            }
            _ => {
                warnings.push(format!(
                    "unsupported security scheme type '{scheme_type}' for '{name}', skipping"
                ));
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Route + Handler builder
// ---------------------------------------------------------------------------

fn build_routes_and_handlers(
    doc: &Value,
    middleware_list: &[MiddlewareSpec],
    warnings: &mut Vec<String>,
) -> (Vec<RouteSpec>, Vec<HandlerSpec>, Vec<SchemaSpec>) {
    let paths = match doc.get("paths").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return (Vec::new(), Vec::new(), Vec::new()),
    };

    let mut routes = Vec::new();
    let mut handlers = Vec::new();
    let mut synthetic_schemas: IndexMap<String, IndexMap<String, Value>> = IndexMap::new();

    for (path, path_item) in paths {
        let path_obj = match path_item.as_object() {
            Some(o) => o,
            None => continue,
        };

        let rash_path = convert_openapi_path(path);
        let mut methods: IndexMap<HttpMethod, EndpointSpec> = IndexMap::new();
        let mut route_params: IndexMap<String, ParamSpec> = IndexMap::new();
        let mut route_tags: Vec<String> = Vec::new();

        let http_methods = [
            ("get", HttpMethod::Get),
            ("post", HttpMethod::Post),
            ("put", HttpMethod::Put),
            ("patch", HttpMethod::Patch),
            ("delete", HttpMethod::Delete),
            ("head", HttpMethod::Head),
            ("options", HttpMethod::Options),
        ];

        for (method_str, method_enum) in &http_methods {
            let operation = match path_obj.get(*method_str).and_then(|v| v.as_object()) {
                Some(o) => o,
                None => continue,
            };

            let operation_id = operation
                .get("operationId")
                .and_then(|v| v.as_str())
                .map(String::from);

            let summary = operation
                .get("summary")
                .and_then(|v| v.as_str())
                .map(String::from);

            let description = operation
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Tags
            if let Some(tags) = operation.get("tags").and_then(|t| t.as_array()) {
                for tag in tags {
                    if let Some(t) = tag.as_str() {
                        if !route_tags.contains(&t.to_string()) {
                            route_tags.push(t.to_string());
                        }
                    }
                }
            }

            // Parameters
            let params = operation
                .get("parameters")
                .and_then(|p| p.as_array())
                .cloned()
                .unwrap_or_default();

            let mut query_properties: IndexMap<String, Value> = IndexMap::new();
            let mut query_required: Vec<String> = Vec::new();

            for param in &params {
                let param_name = param.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let param_in = param.get("in").and_then(|i| i.as_str()).unwrap_or("");
                let required = param.get("required").and_then(|r| r.as_bool()).unwrap_or(false);

                match param_in {
                    "path" => {
                        let schema = param.get("schema").cloned().unwrap_or(serde_json::json!({}));
                        let param_type = schema
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("string")
                            .to_string();
                        let format = schema
                            .get("format")
                            .and_then(|f| f.as_str())
                            .map(String::from);
                        let desc = param
                            .get("description")
                            .and_then(|d| d.as_str())
                            .map(String::from);

                        route_params.insert(
                            param_name.to_string(),
                            ParamSpec {
                                param_type,
                                format,
                                description: desc,
                            },
                        );
                    }
                    "query" => {
                        let schema = param.get("schema").cloned().unwrap_or(serde_json::json!({ "type": "string" }));
                        query_properties.insert(param_name.to_string(), schema);
                        if required {
                            query_required.push(param_name.to_string());
                        }
                    }
                    _ => {
                        warnings.push(format!(
                            "parameter in '{param_in}' not fully supported for {path} {method_str}"
                        ));
                    }
                }
            }

            // Request body
            let body_spec = operation
                .get("requestBody")
                .and_then(|rb| {
                    rb.get("content")
                        .and_then(|c| c.get("application/json"))
                        .and_then(|aj| aj.get("schema"))
                })
                .map(|schema| {
                    if let Some(ref_str) = schema.get("$ref").and_then(|r| r.as_str()) {
                        RequestBodySpec {
                            reference: resolve_ref_string(ref_str),
                            content_type: Some("application/json".to_string()),
                        }
                    } else {
                        // Inline schema – create synthetic name and materialize
                        let synthetic_name = format!(
                            "{}Body",
                            operation_id.as_deref().unwrap_or("Unknown")
                        );
                        warnings.push(format!(
                            "inline request body schema at {path} {method_str} mapped to '{synthetic_name}'"
                        ));
                        synthetic_schemas
                            .entry("Synthetic".to_string())
                            .or_default()
                            .insert(synthetic_name.clone(), schema.clone());
                        RequestBodySpec {
                            reference: synthetic_name,
                            content_type: Some("application/json".to_string()),
                        }
                    }
                });

            // Query ref — also materialize synthetic schema
            let query_ref = if !query_properties.is_empty() {
                let query_schema_name = format!(
                    "{}Query",
                    operation_id.as_deref().unwrap_or("Unknown")
                );
                let mut props = serde_json::Map::new();
                for (k, v) in &query_properties {
                    props.insert(k.clone(), v.clone());
                }
                let schema_val = serde_json::json!({
                    "type": "object",
                    "properties": Value::Object(props),
                    "required": query_required,
                });
                synthetic_schemas
                    .entry("Synthetic".to_string())
                    .or_default()
                    .insert(query_schema_name.clone(), schema_val);
                Some(Ref {
                    reference: query_schema_name,
                    config: None,
                })
            } else {
                None
            };

            let request = if body_spec.is_some() || query_ref.is_some() {
                Some(RequestSpec {
                    query: query_ref,
                    body: body_spec,
                    headers: None,
                })
            } else {
                None
            };

            // Responses
            let response_map = build_responses(operation, warnings);

            // Endpoint middleware from security
            let endpoint_middleware = build_endpoint_middleware(operation, middleware_list);

            // Handler name
            let handler_name = operation_id
                .clone()
                .unwrap_or_else(|| format!("{}{}", method_str, sanitize_path_for_name(&rash_path)));

            // Create handler ref
            let handler_ref = Ref {
                reference: handler_name.clone(),
                config: None,
            };

            methods.insert(
                *method_enum,
                EndpointSpec {
                    operation_id: operation_id.clone(),
                    summary: summary.clone(),
                    handler: handler_ref,
                    middleware: endpoint_middleware,
                    request,
                    response: if response_map.is_empty() {
                        None
                    } else {
                        Some(response_map)
                    },
                },
            );

            // Generate stub handler
            let handler = build_stub_handler(&handler_name, description.or(summary));
            handlers.push(handler);
        }

        if methods.is_empty() {
            continue;
        }

        routes.push(RouteSpec {
            schema: None,
            path: rash_path,
            description: None,
            params: if route_params.is_empty() {
                None
            } else {
                Some(route_params)
            },
            methods,
            tags: route_tags,
            meta: None,
        });
    }

    // Convert synthetic schemas to SchemaSpec
    let extra_schemas: Vec<SchemaSpec> = synthetic_schemas
        .into_iter()
        .map(|(group, defs)| SchemaSpec {
            schema: None,
            name: group,
            description: Some("Synthetic schemas from import".to_string()),
            definitions: defs,
            meta: None,
        })
        .collect();

    (routes, handlers, extra_schemas)
}

/// Convert OpenAPI path `{param}` to Rash path `:param`
fn convert_openapi_path(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut in_brace = false;

    for ch in path.chars() {
        match ch {
            '{' => {
                in_brace = true;
                result.push(':');
            }
            '}' => {
                in_brace = false;
            }
            _ => {
                result.push(ch);
            }
        }
    }

    if in_brace {
        // Malformed – just return as-is
        return path.to_string();
    }

    result
}

fn sanitize_path_for_name(path: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty() && !s.starts_with(':'))
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect()
}

fn build_responses(
    operation: &serde_json::Map<String, Value>,
    warnings: &mut Vec<String>,
) -> IndexMap<String, ResponseSpec> {
    let responses = match operation.get("responses").and_then(|r| r.as_object()) {
        Some(r) => r,
        None => return IndexMap::new(),
    };

    let mut result = IndexMap::new();

    for (status, resp) in responses {
        let description = resp
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);

        let schema_ref = resp
            .get("content")
            .and_then(|c| c.get("application/json"))
            .and_then(|aj| aj.get("schema"))
            .and_then(|s| {
                if let Some(ref_str) = s.get("$ref").and_then(|r| r.as_str()) {
                    Some(Ref {
                        reference: resolve_ref_string(ref_str),
                        config: None,
                    })
                } else {
                    warnings.push(format!(
                        "inline response schema for status {status}, skipping ref"
                    ));
                    None
                }
            });

        result.insert(
            status.clone(),
            ResponseSpec {
                description,
                schema: schema_ref,
            },
        );
    }

    result
}

fn build_endpoint_middleware(
    operation: &serde_json::Map<String, Value>,
    middleware_list: &[MiddlewareSpec],
) -> Vec<Ref> {
    let security = match operation.get("security").and_then(|s| s.as_array()) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut refs = Vec::new();

    for sec_item in security {
        if let Some(obj) = sec_item.as_object() {
            for key in obj.keys() {
                // Check if we have a matching middleware
                if middleware_list.iter().any(|m| &m.name == key) {
                    refs.push(Ref {
                        reference: key.clone(),
                        config: None,
                    });
                }
            }
        }
    }

    refs
}

fn build_stub_handler(name: &str, description: Option<String>) -> HandlerSpec {
    let body_json: Vec<serde_json::Value> = vec![serde_json::json!({
        "type": "ReturnStatement",
        "tier": 0,
        "value": {
            "type": "HttpRespond",
            "tier": 1,
            "status": 200,
            "body": {
                "type": "ObjectExpr",
                "tier": 0,
                "properties": {
                    "message": {
                        "type": "Literal",
                        "tier": 0,
                        "value": "TODO: implement"
                    }
                }
            }
        }
    })];

    let body: Vec<rash_spec::types::ast::AstNode> = body_json
        .into_iter()
        .map(|v| serde_json::from_value(v).expect("stub handler AST must be valid"))
        .collect();

    let mut params = IndexMap::new();
    params.insert(
        "ctx".to_string(),
        HandlerParam {
            param_type: "RequestContext".to_string(),
            description: None,
        },
    );

    HandlerSpec {
        schema: None,
        name: name.to_string(),
        description,
        is_async: true,
        params: Some(params),
        return_type: Some(TypeRef::Reference(Ref {
            reference: "HttpResponse".to_string(),
            config: None,
        })),
        body,
        meta: Some(HandlerMeta {
            max_tier: Some(1),
            languages: vec![
                Language::Typescript,
                Language::Rust,
                Language::Python,
                Language::Go,
            ],
            bridges: Vec::new(),
            meta: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn petstore_json() -> String {
        serde_json::json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Petstore",
                "version": "1.0.0",
                "description": "A sample pet store API"
            },
            "servers": [
                { "url": "http://localhost:8080/api/v1" }
            ],
            "paths": {
                "/pets": {
                    "get": {
                        "operationId": "listPets",
                        "summary": "List all pets",
                        "tags": ["pets"],
                        "parameters": [
                            {
                                "name": "limit",
                                "in": "query",
                                "required": false,
                                "schema": { "type": "integer", "format": "int32" }
                            }
                        ],
                        "security": [{ "bearerAuth": [] }],
                        "responses": {
                            "200": {
                                "description": "A list of pets",
                                "content": {
                                    "application/json": {
                                        "schema": { "$ref": "#/components/schemas/PetList" }
                                    }
                                }
                            }
                        }
                    },
                    "post": {
                        "operationId": "createPet",
                        "summary": "Create a pet",
                        "tags": ["pets"],
                        "security": [{ "bearerAuth": [] }],
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CreatePetBody" }
                                }
                            }
                        },
                        "responses": {
                            "201": {
                                "description": "Pet created",
                                "content": {
                                    "application/json": {
                                        "schema": { "$ref": "#/components/schemas/PetResponse" }
                                    }
                                }
                            }
                        }
                    }
                },
                "/pets/{petId}": {
                    "get": {
                        "operationId": "getPet",
                        "summary": "Get a pet by ID",
                        "tags": ["pets"],
                        "parameters": [
                            {
                                "name": "petId",
                                "in": "path",
                                "required": true,
                                "schema": { "type": "string", "format": "uuid" },
                                "description": "Pet identifier"
                            }
                        ],
                        "responses": {
                            "200": {
                                "description": "Pet details",
                                "content": {
                                    "application/json": {
                                        "schema": { "$ref": "#/components/schemas/PetResponse" }
                                    }
                                }
                            },
                            "404": {
                                "description": "Pet not found"
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "format": "uuid" },
                            "name": { "type": "string" },
                            "tag": { "type": "string" }
                        }
                    },
                    "PetResponse": {
                        "type": "object",
                        "properties": {
                            "data": { "$ref": "#/components/schemas/Pet" }
                        }
                    },
                    "PetList": {
                        "type": "object",
                        "properties": {
                            "items": {
                                "type": "array",
                                "items": { "$ref": "#/components/schemas/Pet" }
                            }
                        }
                    },
                    "CreatePetBody": {
                        "type": "object",
                        "required": ["name"],
                        "properties": {
                            "name": { "type": "string" },
                            "tag": { "type": "string" }
                        }
                    },
                    "ErrorResponse": {
                        "type": "object",
                        "properties": {
                            "message": { "type": "string" },
                            "code": { "type": "string" }
                        }
                    }
                },
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer",
                        "bearerFormat": "JWT",
                        "description": "JWT authentication"
                    }
                }
            }
        })
        .to_string()
    }

    #[test]
    fn test_import_petstore_config() {
        let result = import_openapi(&petstore_json()).unwrap();

        assert_eq!(result.config.name, "Petstore");
        assert_eq!(result.config.version, "1.0.0");
        assert_eq!(result.config.description.as_deref(), Some("A sample pet store API"));
        assert_eq!(result.config.server.host, "localhost");
        assert_eq!(result.config.server.port, 8080);
        assert_eq!(
            result.config.server.base_path.as_deref(),
            Some("/api/v1")
        );
    }

    #[test]
    fn test_import_petstore_routes() {
        let result = import_openapi(&petstore_json()).unwrap();

        assert_eq!(result.routes.len(), 2);

        // /pets route
        let pets_route = result.routes.iter().find(|r| r.path == "/pets").unwrap();
        assert_eq!(pets_route.methods.len(), 2);
        assert!(pets_route.methods.contains_key(&HttpMethod::Get));
        assert!(pets_route.methods.contains_key(&HttpMethod::Post));
        assert!(pets_route.tags.contains(&"pets".to_string()));

        let get_pets = &pets_route.methods[&HttpMethod::Get];
        assert_eq!(get_pets.operation_id.as_deref(), Some("listPets"));
        assert!(get_pets.request.is_some());
        assert!(get_pets.request.as_ref().unwrap().query.is_some());

        // /pets/:petId route
        let pet_route = result
            .routes
            .iter()
            .find(|r| r.path == "/pets/:petId")
            .unwrap();
        assert!(pet_route.params.is_some());
        let params = pet_route.params.as_ref().unwrap();
        assert_eq!(params["petId"].param_type, "string");
        assert_eq!(params["petId"].format.as_deref(), Some("uuid"));
    }

    #[test]
    fn test_import_petstore_schemas() {
        let result = import_openapi(&petstore_json()).unwrap();

        // Schemas should be grouped: "Pet" group (Pet, PetResponse, PetList, CreatePetBody), "Error" group
        assert!(!result.schemas.is_empty());

        // Verify Pet group exists
        let pet_schema = result.schemas.iter().find(|s| s.name == "Pet").unwrap();
        assert!(pet_schema.definitions.contains_key("Pet"));
        assert!(pet_schema.definitions.contains_key("PetResponse"));
        assert!(pet_schema.definitions.contains_key("PetList"));
        assert!(pet_schema.definitions.contains_key("CreatePetBody"));
    }

    #[test]
    fn test_import_petstore_middleware() {
        let result = import_openapi(&petstore_json()).unwrap();

        assert_eq!(result.middleware.len(), 1);
        let bearer = &result.middleware[0];
        assert_eq!(bearer.name, "bearerAuth");
        assert_eq!(bearer.middleware_type, MiddlewareType::Request);
        assert!(bearer.handler.is_some());
        assert!(bearer.errors.is_some());
    }

    #[test]
    fn test_import_petstore_handlers() {
        let result = import_openapi(&petstore_json()).unwrap();

        assert_eq!(result.handlers.len(), 3);

        let handler_names: Vec<&str> = result.handlers.iter().map(|h| h.name.as_str()).collect();
        assert!(handler_names.contains(&"listPets"));
        assert!(handler_names.contains(&"createPet"));
        assert!(handler_names.contains(&"getPet"));

        for handler in &result.handlers {
            assert!(handler.is_async);
            assert!(handler.params.is_some());
            assert!(!handler.body.is_empty());
            assert_eq!(handler.meta.as_ref().unwrap().max_tier, Some(1));
        }
    }

    #[test]
    fn test_import_petstore_security_middleware_refs() {
        let result = import_openapi(&petstore_json()).unwrap();

        let pets_route = result.routes.iter().find(|r| r.path == "/pets").unwrap();
        let get_endpoint = &pets_route.methods[&HttpMethod::Get];
        assert_eq!(get_endpoint.middleware.len(), 1);
        assert_eq!(get_endpoint.middleware[0].reference, "bearerAuth");
    }

    #[test]
    fn test_ref_resolution() {
        assert_eq!(
            resolve_ref_string("#/components/schemas/UserResponse"),
            "UserResponse"
        );
        assert_eq!(
            resolve_ref_string("#/components/schemas/Nested/Deep"),
            "Deep"
        );
    }

    #[test]
    fn test_schema_ref_replacement() {
        let input = serde_json::json!({
            "type": "object",
            "properties": {
                "user": { "$ref": "#/components/schemas/User" },
                "items": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Item" }
                }
            },
            "allOf": [
                { "$ref": "#/components/schemas/Base" },
                { "type": "object", "properties": { "extra": { "type": "string" } } }
            ]
        });

        let resolved = resolve_schema_refs(input);

        assert_eq!(resolved["properties"]["user"]["ref"], "User");
        assert_eq!(resolved["properties"]["items"]["items"]["ref"], "Item");
        assert_eq!(resolved["allOf"][0]["ref"], "Base");
        // Non-ref stays intact
        assert_eq!(
            resolved["allOf"][1]["type"],
            "object"
        );
    }

    #[test]
    fn test_path_conversion() {
        assert_eq!(convert_openapi_path("/pets/{petId}"), "/pets/:petId");
        assert_eq!(
            convert_openapi_path("/users/{userId}/posts/{postId}"),
            "/users/:userId/posts/:postId"
        );
        assert_eq!(convert_openapi_path("/simple"), "/simple");
    }

    #[test]
    fn test_group_name_inference() {
        assert_eq!(infer_group_name("CreateUserBody"), "User");
        assert_eq!(infer_group_name("UserResponse"), "User");
        assert_eq!(infer_group_name("ListUsersQuery"), "Users");
        assert_eq!(infer_group_name("Pet"), "Pet");
        assert_eq!(infer_group_name("ErrorResponse"), "Error");
    }

    #[test]
    fn test_stub_handler_structure() {
        let handler = build_stub_handler("testOp", Some("Test operation".into()));
        assert_eq!(handler.name, "testOp");
        assert!(handler.is_async);
        assert_eq!(handler.body.len(), 1);
        assert!(handler.params.is_some());
        assert!(handler.return_type.is_some());
    }

    #[test]
    fn test_unsupported_version() {
        let doc = serde_json::json!({ "openapi": "2.0" }).to_string();
        let err = import_openapi(&doc).unwrap_err();
        assert!(matches!(err, OpenApiError::UnsupportedVersion(_)));
    }

    #[test]
    fn test_missing_openapi_field() {
        let doc = serde_json::json!({ "info": { "title": "test" } }).to_string();
        let err = import_openapi(&doc).unwrap_err();
        assert!(matches!(err, OpenApiError::MissingField(_)));
    }

    #[test]
    fn test_warnings_for_unsupported_schemes() {
        let doc = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "test", "version": "1.0.0" },
            "paths": {},
            "components": {
                "securitySchemes": {
                    "oauth2": {
                        "type": "oauth2",
                        "flows": {
                            "implicit": {
                                "authorizationUrl": "https://example.com",
                                "scopes": {}
                            }
                        }
                    }
                }
            }
        })
        .to_string();

        let result = import_openapi(&doc).unwrap();
        assert!(result.middleware.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("oauth2")));
    }

    #[test]
    fn test_api_key_middleware() {
        let doc = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "test", "version": "1.0.0" },
            "paths": {},
            "components": {
                "securitySchemes": {
                    "apiKeyAuth": {
                        "type": "apiKey",
                        "in": "header",
                        "name": "X-API-Key"
                    }
                }
            }
        })
        .to_string();

        let result = import_openapi(&doc).unwrap();
        assert_eq!(result.middleware.len(), 1);
        assert_eq!(result.middleware[0].name, "apiKeyAuth");
    }

    #[test]
    fn test_server_url_parsing() {
        let mut warnings = Vec::new();

        // Full URL
        let doc = serde_json::json!({
            "servers": [{ "url": "https://api.example.com:9090/v2" }]
        });
        let (host, port, base) = parse_server_url(&doc, &mut warnings);
        assert_eq!(host, "api.example.com");
        assert_eq!(port, 9090);
        assert_eq!(base.as_deref(), Some("/v2"));

        // Relative path
        let doc = serde_json::json!({
            "servers": [{ "url": "/api/v1" }]
        });
        let (host, port, base) = parse_server_url(&doc, &mut warnings);
        assert_eq!(host, "0.0.0.0");
        assert_eq!(port, 3000);
        assert_eq!(base.as_deref(), Some("/api/v1"));

        // No servers
        let doc = serde_json::json!({});
        let (host, port, base) = parse_server_url(&doc, &mut warnings);
        assert_eq!(host, "0.0.0.0");
        assert_eq!(port, 3000);
        assert!(base.is_none());
    }
}
