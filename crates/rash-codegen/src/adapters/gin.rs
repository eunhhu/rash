use rash_spec::types::common::{Framework, Language};

use crate::context::EmitContext;
use crate::traits::{CtxAccessPattern, FrameworkAdapter, LanguageEmitter};
use rash_ir::expr::ExprIR;
use rash_ir::types::{HandlerIR, MiddlewareIR, ProjectIR, RouteIR};

/// Gin framework adapter for Go.
pub struct GinAdapter;

impl FrameworkAdapter for GinAdapter {
    fn framework(&self) -> Framework {
        Framework::Gin
    }

    fn compatible_language(&self) -> Language {
        Language::Go
    }

    fn emit_route_registration(
        &self,
        route: &RouteIR,
        _emitter: &dyn LanguageEmitter,
        _ctx: &mut EmitContext,
    ) -> String {
        let mut lines = Vec::new();
        for (method, endpoint) in &route.methods {
            let method_upper = format!("{:?}", method).to_uppercase();
            let mw_chain: Vec<String> = endpoint
                .middleware
                .iter()
                .map(|mw| mw.to_string())
                .collect();

            let handlers = if mw_chain.is_empty() {
                endpoint.handler_ref.clone()
            } else {
                format!("{}, {}", mw_chain.join(", "), endpoint.handler_ref)
            };

            lines.push(format!(
                "router.{}(\"{}\", {})",
                method_upper, route.path, handlers
            ));
        }
        lines.join("\n")
    }

    fn emit_middleware_apply(&self, mw_ref: &str, _ctx: &mut EmitContext) -> String {
        format!("router.Use({})", mw_ref)
    }

    fn emit_handler(
        &self,
        handler: &HandlerIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String {
        let mut lines = Vec::new();

        lines.push(format!(
            "func {}(c *gin.Context) {{",
            handler.name
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

        let name = project
            .config
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("rash-app");

        format!(
            r#"package main

import (
	"fmt"
	"os"

	"github.com/gin-gonic/gin"
)

func main() {{
	r := gin.Default()

	registerRoutes(r)

	port := os.Getenv("PORT")
	if port == "" {{
		port = "{port}"
	}}
	fmt.Printf("{name} running on port %s\n", port)
	r.Run(":" + port)
}}
"#,
            port = port,
            name = name
        )
    }

    fn emit_project_config(&self, project: &ProjectIR) -> Vec<(String, String)> {
        let mut files = Vec::new();

        let name = project
            .config
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("rash-app");

        let go_mod = format!(
            r#"module {name}

go 1.21

require (
	github.com/gin-gonic/gin v1.9.1
	gorm.io/gorm v1.25.0
	gorm.io/driver/postgres v1.5.0
	golang.org/x/crypto v0.17.0
	github.com/golang-jwt/jwt/v5 v5.2.0
)
"#,
            name = name
        );
        files.push(("go.mod".to_string(), go_mod));

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
            "func {}() gin.HandlerFunc {{\n\treturn func(c *gin.Context) {{\n\t\t// TODO: implement {} middleware\n\t\tc.Next()\n\t}}\n}}",
            mw.name, mw.middleware_type
        )
    }

    fn ctx_access_pattern(&self) -> CtxAccessPattern {
        CtxAccessPattern::SingleContext
    }

    fn wrap_route_file(&self, imports: &str, route_blocks: &str, _ctx: &mut EmitContext) -> String {
        let mut lines = vec![
            "package routes".to_string(),
            String::new(),
            "import (".to_string(),
            "\t\"github.com/gin-gonic/gin\"".to_string(),
            ")".to_string(),
        ];
        if !imports.is_empty() {
            lines.push(imports.to_string());
        }
        lines.push(String::new());
        lines.push("func RegisterRoutes(router *gin.Engine) {".to_string());
        // Indent route blocks
        for line in route_blocks.lines() {
            if !line.is_empty() {
                lines.push(format!("\t{}", line));
            }
        }
        lines.push("}".to_string());
        lines.join("\n")
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
        let adapter = GinAdapter;
        let emitter = crate::emitters::go_lang::GoEmitter;
        let mut ctx = EmitContext::new(IndentStyle::Tabs);

        let route = RouteIR {
            path: "/api/users".to_string(),
            methods: {
                let mut m = IndexMap::new();
                m.insert(
                    HttpMethod::Get,
                    EndpointIR {
                        operation_id: "listUsers".to_string(),
                        summary: None,
                        handler_ref: "ListUsers".to_string(),
                        middleware: vec!["AuthMiddleware".to_string()],
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
        assert!(code.contains("router.GET(\"/api/users\", AuthMiddleware, ListUsers)"));
    }

    #[test]
    fn test_emit_entrypoint() {
        let adapter = GinAdapter;
        let project = ProjectIR {
            config: serde_json::json!({
                "name": "test-app",
                "server": { "port": 3000 }
            }),
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        };

        let mut ctx = EmitContext::new(IndentStyle::Tabs);
        let code = adapter.emit_entrypoint(&project, &mut ctx);
        assert!(code.contains("gin.Default()"));
        assert!(code.contains("3000"));
        assert!(code.contains("test-app"));
    }
}
