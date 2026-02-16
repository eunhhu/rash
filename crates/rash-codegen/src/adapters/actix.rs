use rash_spec::types::common::{Framework, Language};

use crate::context::EmitContext;
use crate::traits::{CtxAccessPattern, FrameworkAdapter, LanguageEmitter};
use rash_ir::expr::ExprIR;
use rash_ir::types::{HandlerIR, MiddlewareIR, ProjectIR, RouteIR};

use super::convert_colon_params_to_braces;

/// Actix-web framework adapter for Rust.
pub struct ActixAdapter;

impl FrameworkAdapter for ActixAdapter {
    fn framework(&self) -> Framework {
        Framework::Actix
    }

    fn compatible_language(&self) -> Language {
        Language::Rust
    }

    fn emit_route_registration(
        &self,
        route: &RouteIR,
        _emitter: &dyn LanguageEmitter,
        _ctx: &mut EmitContext,
    ) -> String {
        let mut lines = Vec::new();
        for (method, endpoint) in &route.methods {
            let method_lower = format!("{:?}", method).to_lowercase();
            lines.push(format!(
                ".route(\"{}\", web::{}().to({}))",
                route.path, method_lower, endpoint.handler_ref
            ));
        }
        lines.join("\n")
    }

    fn emit_middleware_apply(&self, mw_ref: &str, _ctx: &mut EmitContext) -> String {
        format!(".wrap({})", mw_ref)
    }

    fn emit_handler(
        &self,
        handler: &HandlerIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String {
        let async_kw = if handler.is_async { "async " } else { "" };
        let mut lines = Vec::new();

        // Actix uses extractors as function params
        let params = if handler.params.is_empty() {
            String::new()
        } else {
            handler
                .params
                .iter()
                .map(|p| format!("{}: {}", p.name, emitter.emit_type(&p.type_ir)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        lines.push(format!(
            "pub {}fn {}({}) -> impl Responder {{",
            async_kw, handler.name, params
        ));

        ctx.push_indent();
        for stmt in &handler.body {
            lines.push(emitter.emit_statement(stmt, ctx));
        }
        ctx.pop_indent();

        lines.push("}".to_string());
        lines.join("\n")
    }

    fn emit_entrypoint(&self, project: &ProjectIR, _ctx: &mut EmitContext) -> String {
        let port = project
            .config
            .get("server")
            .and_then(|s| s.get("port"))
            .and_then(|p| p.as_u64())
            .unwrap_or(8080);

        format!(
            r#"use actix_web::{{web, App, HttpServer}};

mod handlers;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {{
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "{port}".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    println!("Server running on port {{}}", port);

    HttpServer::new(|| {{
        App::new()
            .configure(routes::configure)
    }})
    .bind(("0.0.0.0", port))?
    .run()
    .await
}}
"#,
            port = port
        )
    }

    fn emit_project_config(&self, project: &ProjectIR) -> Vec<(String, String)> {
        let mut files = Vec::new();

        let name = project
            .config
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("rash-app");

        let cargo_toml = format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4"
actix-rt = "2"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
tokio = {{ version = "1", features = ["full"] }}
sea-orm = {{ version = "0.12", features = ["sqlx-postgres", "runtime-tokio-rustls"] }}
jsonwebtoken = "9"
argon2 = "0.5"
"#,
            name = name
        );
        files.push(("Cargo.toml".to_string(), cargo_toml));

        files
    }

    fn emit_domain_expr(
        &self,
        _expr: &ExprIR,
        _emitter: &dyn LanguageEmitter,
        _ctx: &mut EmitContext,
    ) -> Option<String> {
        None
    }

    fn emit_middleware_def(
        &self,
        mw: &MiddlewareIR,
        _emitter: &dyn LanguageEmitter,
        _ctx: &mut EmitContext,
    ) -> String {
        format!(
            r#"use actix_web::dev::{{ServiceRequest, ServiceResponse, Transform, Service}};
use std::future::{{Ready, ready}};

pub struct {name};

// TODO: implement {mw_type} middleware
impl<S, B> Transform<S, ServiceRequest> for {name}
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
{{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = {name}Middleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {{
        ready(Ok({name}Middleware {{ service }}))
    }}
}}

pub struct {name}Middleware<S> {{
    service: S,
}}"#,
            name = mw.name,
            mw_type = mw.middleware_type
        )
    }

    fn ctx_access_pattern(&self) -> CtxAccessPattern {
        CtxAccessPattern::Extractors
    }

    fn wrap_route_file(&self, imports: &str, route_blocks: &str, _ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        lines.push("use actix_web::web;".to_string());
        if !imports.is_empty() {
            lines.push(imports.to_string());
        }
        lines.push(String::new());
        lines.push("pub fn configure(cfg: &mut web::ServiceConfig) {".to_string());
        lines.push("    cfg".to_string());
        // Indent route blocks inside configure function
        for line in route_blocks.lines() {
            if !line.is_empty() {
                lines.push(format!("        {}", line));
            }
        }
        lines.push("        ;".to_string());
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn normalize_path(&self, path: &str) -> String {
        // Actix uses {param} instead of :param
        convert_colon_params_to_braces(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::IndentStyle;
    use indexmap::IndexMap;
    use rash_ir::types::{EndpointIR, RequestIR};
    use rash_spec::types::common::HttpMethod;

    #[test]
    fn test_emit_route_registration() {
        let adapter = ActixAdapter;
        let emitter = crate::emitters::rust_lang::RustEmitter;
        let mut ctx = EmitContext::new(IndentStyle::Spaces(4));

        let route = RouteIR {
            path: "/api/users".to_string(),
            methods: {
                let mut m = IndexMap::new();
                m.insert(
                    HttpMethod::Get,
                    EndpointIR {
                        operation_id: "listUsers".to_string(),
                        summary: None,
                        handler_ref: "list_users".to_string(),
                        middleware: vec![],
                        request: RequestIR {
                            query_schema: None,
                            body_schema: None,
                            content_type: None,
                        },
                        response: IndexMap::new(),
                    },
                );
                m
            },
            tags: vec![],
        };

        let code = adapter.emit_route_registration(&route, &emitter, &mut ctx);
        assert!(code.contains(".route(\"/api/users\", web::get().to(list_users))"));
    }

    #[test]
    fn test_emit_entrypoint() {
        let adapter = ActixAdapter;
        let project = ProjectIR {
            config: serde_json::json!({
                "name": "test-app",
                "server": { "port": 9090 }
            }),
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        };

        let mut ctx = EmitContext::new(IndentStyle::Spaces(4));
        let code = adapter.emit_entrypoint(&project, &mut ctx);
        assert!(code.contains("actix_web"));
        assert!(code.contains("9090"));
    }
}
