use indexmap::IndexMap;
use regex::Regex;

use rash_spec::types::common::{HttpMethod, Ref};
use rash_spec::types::handler::HandlerSpec;
use rash_spec::types::middleware::{MiddlewareSpec, MiddlewareType};
use rash_spec::types::route::{EndpointSpec, RouteSpec};

use super::handler_extract;
use super::schema_extract::extract_brace_block;

/// Extract Express routes, handlers, and middleware from source code.
///
/// Returns `(routes, handlers, middleware)`.
pub fn extract_express(
    source: &str,
    warnings: &mut Vec<String>,
) -> (Vec<RouteSpec>, Vec<HandlerSpec>, Vec<MiddlewareSpec>) {
    let mut routes: Vec<RouteSpec> = Vec::new();
    let mut handlers: Vec<HandlerSpec> = Vec::new();
    let mut middleware: Vec<MiddlewareSpec> = Vec::new();

    extract_global_middleware(source, &mut middleware, warnings);
    extract_route_handlers(source, &mut routes, &mut handlers, warnings);

    (routes, handlers, middleware)
}

/// Extract global middleware: `app.use(express.json())`, `app.use(cors())`, etc.
fn extract_global_middleware(
    source: &str,
    middleware: &mut Vec<MiddlewareSpec>,
    _warnings: &mut Vec<String>,
) {
    let mw_re = Regex::new(r"app\.use\(\s*(\w+(?:\.\w+)?)\(\s*\)\s*\)").unwrap();

    for cap in mw_re.captures_iter(source) {
        let mw_call = &cap[1];
        let name = mw_call.replace('.', "_");

        middleware.push(MiddlewareSpec {
            schema: None,
            name: name.clone(),
            description: Some(format!("Global middleware: {mw_call}()")),
            middleware_type: MiddlewareType::Request,
            config: None,
            handler: None,
            provides: None,
            errors: None,
            compose: None,
            short_circuit: None,
            meta: None,
        });
    }
}

/// Extract route handlers: `app.get("/path", handler)`, `app.post(...)`, etc.
fn extract_route_handlers(
    source: &str,
    routes: &mut Vec<RouteSpec>,
    handlers: &mut Vec<HandlerSpec>,
    warnings: &mut Vec<String>,
) {
    // Match patterns like: app.get("/path", ...) or router.get("/path", ...)
    let route_re = Regex::new(
        r#"(?:app|router)\.(get|post|put|patch|delete|head|options)\(\s*["']([^"']+)["']\s*,"#,
    )
    .unwrap();

    for cap in route_re.captures_iter(source) {
        let method_str = &cap[1];
        let path = &cap[2];

        let method = match method_str.to_lowercase().as_str() {
            "get" => HttpMethod::Get,
            "post" => HttpMethod::Post,
            "put" => HttpMethod::Put,
            "patch" => HttpMethod::Patch,
            "delete" => HttpMethod::Delete,
            "head" => HttpMethod::Head,
            "options" => HttpMethod::Options,
            _ => continue,
        };

        // Extract the handler name (path-based + method)
        let handler_name = make_handler_name(path, method_str);

        // Find the handler body — look for arrow function or function body
        let match_end = cap.get(0).unwrap().end();
        let handler_body = extract_handler_body_from_position(source, match_end, warnings);

        // Build the handler AST from the body
        let body_ast = handler_extract::extract_handler_body(&handler_body, warnings);
        let is_async = source[cap.get(0).unwrap().start()..].contains("async");

        handlers.push(HandlerSpec {
            schema: None,
            name: handler_name.clone(),
            description: Some(format!("{} {}", method_str.to_uppercase(), path)),
            is_async,
            params: None,
            return_type: None,
            body: body_ast,
            meta: None,
        });

        // Build or extend route spec
        let endpoint = EndpointSpec {
            operation_id: Some(handler_name.clone()),
            summary: None,
            handler: Ref {
                reference: handler_name,
                config: None,
            },
            middleware: Vec::new(),
            request: None,
            response: None,
        };

        // Check if we already have a route for this path
        if let Some(existing) = routes.iter_mut().find(|r| r.path == *path) {
            existing.methods.insert(method, endpoint);
        } else {
            let mut methods = IndexMap::new();
            methods.insert(method, endpoint);

            let params = extract_path_params(path);

            routes.push(RouteSpec {
                schema: None,
                path: path.to_string(),
                description: None,
                params: if params.is_empty() {
                    None
                } else {
                    Some(params)
                },
                methods,
                tags: Vec::new(),
                meta: None,
            });
        }
    }
}

/// Extract handler body text from source starting after the route pattern match.
///
/// Looks for arrow function `=> { ... }` or `=> expr` or named function body.
fn extract_handler_body_from_position(
    source: &str,
    start: usize,
    _warnings: &mut Vec<String>,
) -> String {
    let rest = &source[start..];

    // Find arrow function: `async (req, res) => { ... }`
    let arrow_re = Regex::new(r"(?:async\s+)?\([^)]*\)\s*=>\s*\{").unwrap();
    if let Some(m) = arrow_re.find(rest) {
        let brace_start = start + m.end() - 1;
        if let Some(body) = extract_brace_block(source, brace_start) {
            return body;
        }
    }

    // Find function keyword: `async function(req, res) { ... }`
    let fn_re = Regex::new(r"(?:async\s+)?function\s*\([^)]*\)\s*\{").unwrap();
    if let Some(m) = fn_re.find(rest) {
        let brace_start = start + m.end() - 1;
        if let Some(body) = extract_brace_block(source, brace_start) {
            return body;
        }
    }

    // If no body found, take up to the next closing paren at depth 0
    String::new()
}

/// Generate a handler name from a route path and method.
///
/// Example: `/users/:id` + `get` → `getUsersById`
fn make_handler_name(path: &str, method: &str) -> String {
    let parts: Vec<&str> = path
        .split('/')
        .filter(|p| !p.is_empty())
        .collect();

    let mut name = method.to_lowercase();
    for part in &parts {
        if let Some(param) = part.strip_prefix(':') {
            name.push_str("By");
            name.push_str(&capitalize(param));
        } else {
            name.push_str(&capitalize(part));
        }
    }

    if name == method.to_lowercase() {
        // Root path "/"
        name.push_str("Root");
    }

    name
}

/// Extract path parameters from Express-style paths like `/users/:id`.
fn extract_path_params(
    path: &str,
) -> IndexMap<String, rash_spec::types::route::ParamSpec> {
    let mut params = IndexMap::new();
    let param_re = Regex::new(r":(\w+)").unwrap();

    for cap in param_re.captures_iter(path) {
        let name = &cap[1];
        params.insert(
            name.to_string(),
            rash_spec::types::route::ParamSpec {
                param_type: "string".to_string(),
                format: None,
                description: None,
            },
        );
    }

    params
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_basic_routes() {
        let source = r#"
import express from "express";

const app = express();
app.use(express.json());

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

app.listen(3000);
"#;
        let mut warnings = Vec::new();
        let (routes, handlers, middleware) = extract_express(source, &mut warnings);

        // Should find global middleware
        assert!(!middleware.is_empty());
        assert_eq!(middleware[0].name, "express_json");

        // Should find 2 route specs (one for /users, one for /users/:id)
        assert_eq!(routes.len(), 2);

        // /users route should have GET and POST
        let users_route = routes.iter().find(|r| r.path == "/users").unwrap();
        assert!(users_route.methods.contains_key(&HttpMethod::Get));
        assert!(users_route.methods.contains_key(&HttpMethod::Post));

        // /users/:id route should have GET and path params
        let user_by_id_route = routes.iter().find(|r| r.path == "/users/:id").unwrap();
        assert!(user_by_id_route.methods.contains_key(&HttpMethod::Get));
        assert!(user_by_id_route.params.is_some());
        let params = user_by_id_route.params.as_ref().unwrap();
        assert!(params.contains_key("id"));

        // Should have 3 handlers
        assert_eq!(handlers.len(), 3);
    }

    #[test]
    fn test_make_handler_name() {
        assert_eq!(make_handler_name("/users", "get"), "getUsers");
        assert_eq!(make_handler_name("/users/:id", "get"), "getUsersById");
        assert_eq!(make_handler_name("/users", "post"), "postUsers");
        assert_eq!(
            make_handler_name("/users/:id/posts", "get"),
            "getUsersByIdPosts"
        );
        assert_eq!(make_handler_name("/", "get"), "getRoot");
    }

    #[test]
    fn test_extract_path_params() {
        let params = extract_path_params("/users/:id/posts/:postId");
        assert_eq!(params.len(), 2);
        assert!(params.contains_key("id"));
        assert!(params.contains_key("postId"));
    }

    #[test]
    fn test_global_middleware_extraction() {
        let source = r#"
app.use(express.json());
app.use(cors());
"#;
        let mut mw = Vec::new();
        let mut w = Vec::new();
        extract_global_middleware(source, &mut mw, &mut w);
        assert_eq!(mw.len(), 2);
        assert_eq!(mw[0].name, "express_json");
        assert_eq!(mw[1].name, "cors");
    }

    #[test]
    fn test_router_routes() {
        let source = r#"
import express from "express";
const router = express.Router();

router.get("/items", async (req, res) => {
    res.json([]);
});
"#;
        let mut w = Vec::new();
        let (routes, handlers, _) = extract_express(source, &mut w);
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/items");
        assert_eq!(handlers.len(), 1);
    }

    #[test]
    fn test_delete_route() {
        let source = r#"
import express from "express";
const app = express();

app.delete("/users/:id", async (req, res) => {
    const userId = req.params.id;
    await prisma.user.delete({ where: { id: userId } });
    res.status(204).json({});
});
"#;
        let mut w = Vec::new();
        let (routes, handlers, _) = extract_express(source, &mut w);
        assert_eq!(routes.len(), 1);
        assert!(routes[0].methods.contains_key(&HttpMethod::Delete));
        assert_eq!(handlers.len(), 1);
    }
}
