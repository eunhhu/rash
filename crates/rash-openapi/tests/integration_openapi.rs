use indexmap::IndexMap;

use rash_spec::types::common::{HttpMethod, Ref};
use rash_spec::types::config::RashConfig;
use rash_spec::types::middleware::{MiddlewareSpec, MiddlewareType};
use rash_spec::types::route::{EndpointSpec, RouteSpec};
use rash_spec::types::schema::SchemaSpec;

use rash_openapi::{export_openapi, import_openapi, ImportResult};
use rash_openapi::reverse_parse;
use rash_openapi::reverse_parse::detect::DetectedFramework;

// ---------------------------------------------------------------------------
// Export → Import roundtrip
// ---------------------------------------------------------------------------

fn sample_config() -> RashConfig {
    serde_json::from_value(serde_json::json!({
        "name": "roundtrip-test",
        "version": "1.0.0",
        "server": {
            "port": 3000,
            "basePath": "/api"
        },
        "target": {
            "language": "typescript",
            "framework": "express",
            "runtime": "bun"
        }
    }))
    .unwrap()
}

fn sample_schemas() -> Vec<SchemaSpec> {
    vec![SchemaSpec {
        schema: None,
        name: "User".to_string(),
        description: Some("User schema".to_string()),
        definitions: {
            let mut defs = IndexMap::new();
            defs.insert(
                "UserResponse".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "email": { "type": "string", "format": "email" },
                        "name": { "type": "string" }
                    }
                }),
            );
            defs
        },
        meta: None,
    }]
}

fn sample_routes() -> Vec<RouteSpec> {
    let mut methods = IndexMap::new();
    methods.insert(
        HttpMethod::Get,
        EndpointSpec {
            operation_id: Some("listUsers".to_string()),
            summary: Some("List users".to_string()),
            handler: Ref {
                reference: "users.listUsers".to_string(),
                config: None,
            },
            middleware: vec![],
            request: None,
            response: None,
        },
    );

    vec![RouteSpec {
        schema: None,
        path: "/users".to_string(),
        description: None,
        params: None,
        methods,
        tags: vec!["users".to_string()],
        meta: None,
    }]
}

fn sample_middleware() -> Vec<MiddlewareSpec> {
    vec![MiddlewareSpec {
        schema: None,
        name: "auth".to_string(),
        description: Some("JWT auth".to_string()),
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
            reference: "auth.verify".to_string(),
            config: None,
        }),
        provides: None,
        errors: None,
        compose: None,
        short_circuit: None,
        meta: None,
    }]
}

#[test]
fn test_export_roundtrip() {
    let config = sample_config();
    let schemas = sample_schemas();
    let routes = sample_routes();
    let middleware = sample_middleware();

    // Export to OpenAPI
    let openapi_doc = export_openapi(&config, &routes, &schemas, &middleware).unwrap();
    let json = serde_json::to_value(&openapi_doc).unwrap();

    // Verify OpenAPI structure
    assert_eq!(json["openapi"], "3.1.0");
    assert_eq!(json["info"]["title"], "roundtrip-test");
    assert!(json["paths"]["/users"].is_object());

    // Re-import from exported JSON
    let json_str = serde_json::to_string_pretty(&json).unwrap();
    let import_result: ImportResult = import_openapi(&json_str).unwrap();

    // Verify routes survived the roundtrip
    assert!(!import_result.routes.is_empty());
    let imported_route = import_result.routes.iter().find(|r| r.path.contains("users"));
    assert!(imported_route.is_some(), "Should find a users route after roundtrip");
}

#[test]
fn test_import_petstore_sample() {
    let petstore = serde_json::json!({
        "openapi": "3.1.0",
        "info": { "title": "Petstore", "version": "1.0.0" },
        "paths": {
            "/pets": {
                "get": {
                    "operationId": "listPets",
                    "summary": "List all pets",
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
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CreatePetBody" }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created"
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "PetList": {
                    "type": "array",
                    "items": { "$ref": "#/components/schemas/Pet" }
                },
                "Pet": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" },
                        "name": { "type": "string" }
                    }
                },
                "CreatePetBody": {
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": { "type": "string" }
                    }
                }
            }
        }
    });

    let petstore_str = serde_json::to_string_pretty(&petstore).unwrap();
    let result = import_openapi(&petstore_str).unwrap();

    // Should produce routes
    assert!(!result.routes.is_empty());
    let pets_route = result.routes.iter().find(|r| r.path.contains("pets")).unwrap();
    assert!(pets_route.methods.contains_key(&HttpMethod::Get));
    assert!(pets_route.methods.contains_key(&HttpMethod::Post));

    // Should produce schemas
    assert!(!result.schemas.is_empty());

    // Should produce handlers
    assert!(!result.handlers.is_empty());
    let list_handler = result.handlers.iter().find(|h| h.name.contains("listPets"));
    assert!(list_handler.is_some());
}

// ---------------------------------------------------------------------------
// Express reverse parsing
// ---------------------------------------------------------------------------

#[test]
fn test_reverse_parse_express_crud() {
    let source = r#"
import express from "express";
import { z } from "zod";

const app = express();
app.use(express.json());
app.use(cors());

const CreateUserSchema = z.object({
    email: z.string().email(),
    password: z.string().min(8).max(128),
    name: z.string().min(1).max(100),
});

interface User {
    id: string;
    email: string;
    name: string;
}

app.get("/users", async (req, res) => {
    const users = await prisma.user.findMany();
    res.json(users);
});

app.post("/users", async (req, res) => {
    const body = req.body;
    const user = await prisma.user.create({ data: body });
    res.status(201).json(user);
});

app.get("/users/:id", async (req, res) => {
    const userId = req.params.id;
    const user = await prisma.user.findUnique({ where: { id: userId } });
    res.json(user);
});

app.put("/users/:id", async (req, res) => {
    const userId = req.params.id;
    const body = req.body;
    const user = await prisma.user.update({ where: { id: userId }, data: body });
    res.json(user);
});

app.delete("/users/:id", async (req, res) => {
    const userId = req.params.id;
    await prisma.user.delete({ where: { id: userId } });
    res.status(204).json({});
});

app.listen(3000);
"#;

    let result = reverse_parse::reverse_parse(source, "app.ts").unwrap();

    // Framework detection
    assert_eq!(result.framework, DetectedFramework::Express);

    // Middleware
    assert!(result.middleware.len() >= 2, "Should detect express.json() and cors()");

    // Schemas: Zod schema + TS interface
    assert!(result.schemas.len() >= 2, "Should extract CreateUser and User schemas");

    // Routes
    assert!(result.routes.len() >= 2, "Should have /users and /users/:id routes");

    let users_route = result.routes.iter().find(|r| r.path == "/users").unwrap();
    assert!(users_route.methods.contains_key(&HttpMethod::Get));
    assert!(users_route.methods.contains_key(&HttpMethod::Post));

    let user_by_id_route = result.routes.iter().find(|r| r.path == "/users/:id").unwrap();
    assert!(user_by_id_route.methods.contains_key(&HttpMethod::Get));
    assert!(user_by_id_route.methods.contains_key(&HttpMethod::Put));
    assert!(user_by_id_route.methods.contains_key(&HttpMethod::Delete));

    // Path params
    let params = user_by_id_route.params.as_ref().unwrap();
    assert!(params.contains_key("id"));

    // Handlers — one per route-method
    assert!(result.handlers.len() >= 5, "Should have 5 handlers (GET/POST /users + GET/PUT/DELETE /users/:id)");
}

#[test]
fn test_reverse_parse_detects_unknown_framework() {
    let source = r#"
const http = require("http");
const server = http.createServer((req, res) => {
    res.end("hello");
});
"#;
    let result = reverse_parse::reverse_parse(source, "server.ts");
    assert!(result.is_err());
}

#[test]
fn test_reverse_parse_zod_schema_details() {
    let source = r#"
import express from "express";
const app = express();

const ProductSchema = z.object({
    name: z.string().min(1).max(255),
    price: z.number().min(0),
    active: z.boolean(),
    sku: z.string().uuid(),
    email: z.string().email().optional(),
});

app.get("/products", async (req, res) => {
    res.json([]);
});

app.listen(3000);
"#;

    let result = reverse_parse::reverse_parse(source, "app.ts").unwrap();

    let schema = result.schemas.iter().find(|s| s.name == "Product").unwrap();
    let def = &schema.definitions["Product"];
    let props = def["properties"].as_object().unwrap();

    assert_eq!(props["name"]["type"], "string");
    assert_eq!(props["name"]["minLength"], 1);
    assert_eq!(props["name"]["maxLength"], 255);
    assert_eq!(props["price"]["type"], "number");
    assert_eq!(props["price"]["minimum"], 0);
    assert_eq!(props["active"]["type"], "boolean");
    assert_eq!(props["sku"]["format"], "uuid");
    assert_eq!(props["email"]["format"], "email");
    assert_eq!(props["email"]["nullable"], true);
}
