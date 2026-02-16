use rash_spec::types::common::{Framework, Language};

use crate::context::EmitContext;
use crate::traits::{CtxAccessPattern, FrameworkAdapter, LanguageEmitter};
use rash_ir::expr::ExprIR;
use rash_ir::types::{HandlerIR, MiddlewareIR, ProjectIR, RouteIR};

use super::convert_colon_params_to_braces;

/// FastAPI framework adapter for Python.
pub struct FastAPIAdapter;

impl FrameworkAdapter for FastAPIAdapter {
    fn framework(&self) -> Framework {
        Framework::FastAPI
    }

    fn compatible_language(&self) -> Language {
        Language::Python
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
            let mw_str = if endpoint.middleware.is_empty() {
                String::new()
            } else {
                let deps: Vec<String> = endpoint
                    .middleware
                    .iter()
                    .map(|mw| format!("Depends({})", mw))
                    .collect();
                format!(", dependencies=[{}]", deps.join(", "))
            };
            lines.push(format!(
                "@router.{}(\"{}\"{})\\nasync def {}():",
                method_lower, route.path, mw_str, endpoint.handler_ref
            ));
        }
        lines.join("\n\n")
    }

    fn emit_middleware_apply(&self, mw_ref: &str, _ctx: &mut EmitContext) -> String {
        format!("Depends({})", mw_ref)
    }

    fn emit_handler(
        &self,
        handler: &HandlerIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String {
        let async_kw = if handler.is_async { "async " } else { "" };
        let mut lines = Vec::new();

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

        lines.push(format!("{}def {}({}):", async_kw, handler.name, params));

        ctx.push_indent();
        if handler.body.is_empty() {
            lines.push(format!("{}pass", ctx.indent()));
        } else {
            for stmt in &handler.body {
                lines.push(emitter.emit_statement(stmt, ctx));
            }
        }
        ctx.pop_indent();

        lines.join("\n")
    }

    fn emit_entrypoint(&self, project: &ProjectIR, _ctx: &mut EmitContext) -> String {
        let port = project
            .config
            .get("server")
            .and_then(|s| s.get("port"))
            .and_then(|p| p.as_u64())
            .unwrap_or(8000);

        let name = project
            .config
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("rash-app");

        format!(
            r#"import uvicorn
from fastapi import FastAPI
from routes import router

app = FastAPI(title="{name}")
app.include_router(router)

if __name__ == "__main__":
    uvicorn.run("main:app", host="0.0.0.0", port={port}, reload=True)
"#,
            name = name,
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

        let pyproject = format!(
            r#"[project]
name = "{name}"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = [
    "fastapi>=0.104.0",
    "uvicorn[standard]>=0.24.0",
    "pydantic>=2.5.0",
    "sqlalchemy>=2.0.0",
    "passlib[bcrypt]>=1.7.0",
    "python-jose[cryptography]>=3.3.0",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.backends"
"#,
            name = name
        );
        files.push(("pyproject.toml".to_string(), pyproject));

        let requirements = "\
fastapi>=0.104.0
uvicorn[standard]>=0.24.0
pydantic>=2.5.0
sqlalchemy>=2.0.0
passlib[bcrypt]>=1.7.0
python-jose[cryptography]>=3.3.0
";
        files.push(("requirements.txt".to_string(), requirements.to_string()));

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
            "from fastapi import Request\n\nasync def {}(request: Request):\n    # TODO: implement {} middleware\n    pass",
            mw.name, mw.middleware_type
        )
    }

    fn ctx_access_pattern(&self) -> CtxAccessPattern {
        CtxAccessPattern::ParamInjection
    }

    fn wrap_route_file(&self, imports: &str, route_blocks: &str, _ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        lines.push("from fastapi import APIRouter, Depends".to_string());
        if !imports.is_empty() {
            lines.push(imports.to_string());
        }
        lines.push(String::new());
        lines.push("router = APIRouter()".to_string());
        lines.push(String::new());
        if !route_blocks.is_empty() {
            lines.push(route_blocks.to_string());
        }
        lines.join("\n")
    }

    fn normalize_path(&self, path: &str) -> String {
        // FastAPI uses {param} instead of :param
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
        let adapter = FastAPIAdapter;
        let emitter = crate::emitters::python::PythonEmitter;
        let mut ctx = EmitContext::new(IndentStyle::Spaces(4));

        let route = RouteIR {
            path: "/api/users".to_string(),
            methods: {
                let mut m = IndexMap::new();
                m.insert(
                    HttpMethod::Get,
                    EndpointIR {
                        operation_id: "list_users".to_string(),
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
        assert!(code.contains("@router.get(\"/api/users\")"));
        assert!(code.contains("list_users"));
    }

    #[test]
    fn test_emit_entrypoint() {
        let adapter = FastAPIAdapter;
        let project = ProjectIR {
            config: serde_json::json!({
                "name": "test-app",
                "server": { "port": 5000 }
            }),
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        };

        let mut ctx = EmitContext::new(IndentStyle::Spaces(4));
        let code = adapter.emit_entrypoint(&project, &mut ctx);
        assert!(code.contains("FastAPI"));
        assert!(code.contains("5000"));
        assert!(code.contains("test-app"));
    }
}
