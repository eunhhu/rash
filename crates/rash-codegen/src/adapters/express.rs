use rash_spec::types::common::{Framework, Language};

use crate::context::EmitContext;
use crate::generator::{normalize_identifier, normalize_filename};
use crate::traits::{CtxAccessPattern, FrameworkAdapter, LanguageEmitter};
use rash_ir::expr::ExprIR;
use rash_ir::types::{
    HandlerIR, MiddlewareIR, ProjectIR, RouteIR,
};

/// Express.js framework adapter for TypeScript.
pub struct ExpressAdapter;

impl FrameworkAdapter for ExpressAdapter {
    fn framework(&self) -> Framework {
        Framework::Express
    }

    fn compatible_language(&self) -> Language {
        Language::Typescript
    }

    fn emit_route_registration(
        &self,
        route: &RouteIR,
        _emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String {
        let mut lines = Vec::new();
        for (method, endpoint) in &route.methods {
            let method_lower = format!("{:?}", method).to_lowercase();
            let handler_id = normalize_identifier(&endpoint.handler_ref);
            let handler_file = normalize_filename(&endpoint.handler_ref);

            let mw_chain: Vec<String> = endpoint
                .middleware
                .iter()
                .map(|mw| {
                    let mw_id = normalize_identifier(mw);
                    let mw_file = normalize_filename(mw);
                    ctx.add_import(
                        format!("{{ {} }}", mw_id),
                        format!("../middleware/{}", mw_file),
                    );
                    mw_id
                })
                .collect();

            // Import handler
            ctx.add_import(
                format!("{{ {} }}", handler_id),
                format!("../handlers/{}", handler_file),
            );

            let mw_str = if mw_chain.is_empty() {
                String::new()
            } else {
                format!("{}, ", mw_chain.join(", "))
            };

            lines.push(format!(
                "router.{}(\"{}\", {}{});",
                method_lower, route.path, mw_str, handler_id
            ));
        }
        lines.join("\n")
    }

    fn emit_middleware_apply(&self, mw_ref: &str, _ctx: &mut EmitContext) -> String {
        let mw_id = normalize_identifier(mw_ref);
        format!("app.use({});", mw_id)
    }

    fn emit_handler(
        &self,
        handler: &HandlerIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String {
        ctx.add_import("{ Request, Response, NextFunction }", "express");

        let async_kw = if handler.is_async { "async " } else { "" };
        let mut lines = Vec::new();

        lines.push(format!(
            "export {}function {}(req: Request, res: Response, next: NextFunction) {{",
            async_kw, handler.name
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
            .unwrap_or(3000);

        let name = project
            .config
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("rash-app");

        format!(
            r#"import express from "express";
import {{ registerRoutes }} from "./routes";

const app = express();
app.use(express.json());

registerRoutes(app);

const PORT = process.env.PORT || {port};
app.listen(PORT, () => {{
  console.log(`{name} running on port ${{PORT}}`);
}});

export default app;
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

        // package.json
        let package_json = serde_json::json!({
            "name": name,
            "version": "0.1.0",
            "private": true,
            "scripts": {
                "dev": "tsx watch src/index.ts",
                "build": "tsc",
                "start": "node dist/index.js",
                "db:generate": "prisma generate",
                "db:push": "prisma db push"
            },
            "dependencies": {
                "express": "^4.18.0",
                "zod": "^3.22.0",
                "@prisma/client": "^5.0.0"
            },
            "devDependencies": {
                "typescript": "^5.3.0",
                "@types/express": "^4.17.0",
                "@types/node": "^20.0.0",
                "tsx": "^4.0.0",
                "prisma": "^5.0.0"
            }
        });
        files.push((
            "package.json".to_string(),
            serde_json::to_string_pretty(&package_json).unwrap(),
        ));

        // tsconfig.json
        let tsconfig = serde_json::json!({
            "compilerOptions": {
                "target": "ES2022",
                "module": "NodeNext",
                "moduleResolution": "NodeNext",
                "outDir": "./dist",
                "rootDir": "./src",
                "strict": true,
                "esModuleInterop": true,
                "skipLibCheck": true,
                "forceConsistentCasingInFileNames": true,
                "resolveJsonModule": true,
                "declaration": true,
                "declarationMap": true,
                "sourceMap": true
            },
            "include": ["src/**/*"],
            "exclude": ["node_modules", "dist"]
        });
        files.push((
            "tsconfig.json".to_string(),
            serde_json::to_string_pretty(&tsconfig).unwrap(),
        ));

        // prisma/schema.prisma
        let prisma_schema = generate_prisma_schema(project);
        if !prisma_schema.is_empty() {
            files.push(("prisma/schema.prisma".to_string(), prisma_schema));
        }

        files
    }

    fn emit_domain_expr(
        &self,
        _expr: &ExprIR,
        _emitter: &dyn LanguageEmitter,
        _ctx: &mut EmitContext,
    ) -> Option<String> {
        // Use default TypeScript emitter for domain expressions
        None
    }

    fn emit_middleware_def(
        &self,
        mw: &MiddlewareIR,
        _emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String {
        ctx.add_import("{ Request, Response, NextFunction }", "express");
        format!(
            "export function {}(req: Request, res: Response, next: NextFunction) {{\n  // TODO: implement {} middleware\n  next();\n}}",
            mw.name, mw.middleware_type
        )
    }

    fn ctx_access_pattern(&self) -> CtxAccessPattern {
        CtxAccessPattern::ReqRes
    }

    fn wrap_route_file(&self, imports: &str, route_blocks: &str, _ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        lines.push("import { Router, Application } from \"express\";".to_string());
        if !imports.is_empty() {
            lines.push(imports.to_string());
        }
        lines.push(String::new());
        lines.push("const router = Router();".to_string());
        lines.push(String::new());
        if !route_blocks.is_empty() {
            lines.push(route_blocks.to_string());
        }
        lines.push(String::new());
        lines.push("export function registerRoutes(app: Application) {".to_string());
        lines.push("  app.use(router);".to_string());
        lines.push("}".to_string());
        lines.push(String::new());
        lines.push("export default router;".to_string());
        lines.join("\n")
    }
}

/// Generate Prisma schema from project models.
fn generate_prisma_schema(project: &ProjectIR) -> String {
    if project.models.is_empty() {
        return String::new();
    }

    let db_provider = project
        .config
        .get("database")
        .and_then(|d| d.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("postgresql");

    let mut lines = Vec::new();
    lines.push("generator client {".to_string());
    lines.push("  provider = \"prisma-client-js\"".to_string());
    lines.push("}".to_string());
    lines.push(String::new());
    lines.push("datasource db {".to_string());
    lines.push(format!("  provider = \"{}\"", db_provider));
    lines.push("  url      = env(\"DATABASE_URL\")".to_string());
    lines.push("}".to_string());

    for model in &project.models {
        lines.push(String::new());
        lines.push(format!("model {} {{", model.name));
        for (col_name, col_def) in &model.columns {
            let prisma_line = col_to_prisma(col_name, col_def);
            lines.push(format!("  {}", prisma_line));
        }
        for (rel_name, rel_def) in &model.relations {
            let prisma_line = rel_to_prisma(rel_name, rel_def);
            lines.push(format!("  {}", prisma_line));
        }
        if !model.indexes.is_empty() {
            lines.push(String::new());
            for idx in &model.indexes {
                let idx_line = index_to_prisma(idx);
                lines.push(format!("  {}", idx_line));
            }
        }
        lines.push("}".to_string());
    }

    lines.join("\n")
}

fn col_to_prisma(name: &str, def: &serde_json::Value) -> String {
    let col_type = def
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("String");
    let prisma_type = match col_type {
        "string" | "varchar" | "text" => "String",
        "integer" | "int" => "Int",
        "bigint" | "serial" => "BigInt",
        "float" | "double" | "decimal" | "numeric" => "Float",
        "boolean" | "bool" => "Boolean",
        "datetime" | "timestamp" => "DateTime",
        "date" => "DateTime",
        "uuid" => "String",
        "json" | "jsonb" => "Json",
        other => other,
    };

    let nullable = def
        .get("nullable")
        .and_then(|n| n.as_bool())
        .unwrap_or(false);
    let primary_key = def
        .get("primaryKey")
        .and_then(|p| p.as_bool())
        .unwrap_or(false);
    let unique = def
        .get("unique")
        .and_then(|u| u.as_bool())
        .unwrap_or(false);
    let has_default = def.get("default").is_some();

    let mut attrs = Vec::new();
    if primary_key {
        attrs.push("@id".to_string());
    }
    if unique {
        attrs.push("@unique".to_string());
    }
    if has_default {
        let default_val = def.get("default").unwrap();
        let default_str = match default_val.as_str() {
            Some("autoincrement") => "@default(autoincrement())".to_string(),
            Some("uuid") => "@default(uuid())".to_string(),
            Some("cuid") => "@default(cuid())".to_string(),
            Some("now") => "@default(now())".to_string(),
            _ => format!("@default({})", emit_prisma_default(default_val)),
        };
        attrs.push(default_str);
    }
    if col_type == "uuid" && !has_default {
        attrs.push("@db.Uuid".to_string());
    }

    let nullable_mark = if nullable { "?" } else { "" };
    let attrs_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    };

    format!("{} {}{}{}", name, prisma_type, nullable_mark, attrs_str)
}

fn rel_to_prisma(name: &str, def: &serde_json::Value) -> String {
    let model = def
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("Unknown");
    let rel_type = def
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("one-to-many");

    match rel_type {
        "one-to-one" => format!("{} {}?", name, model),
        "one-to-many" => format!("{} {}[]", name, model),
        "many-to-one" => {
            let default_fk = format!("{}Id", name);
            let fk = def
                .get("foreignKey")
                .and_then(|f| f.as_str())
                .unwrap_or(&default_fk);
            format!(
                "{} {} @relation(fields: [{}], references: [id])",
                name, model, fk
            )
        }
        _ => format!("{} {}[]", name, model),
    }
}

fn index_to_prisma(idx: &serde_json::Value) -> String {
    let fields = idx
        .get("fields")
        .and_then(|f| f.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let unique = idx
        .get("unique")
        .and_then(|u| u.as_bool())
        .unwrap_or(false);

    if unique {
        format!("@@unique([{}])", fields)
    } else {
        format!("@@index([{}])", fields)
    }
}

fn emit_prisma_default(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => "\"\"".to_string(),
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
        let adapter = ExpressAdapter;
        let emitter = crate::emitters::typescript::TypeScriptEmitter;
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));

        let route = RouteIR {
            path: "/api/users".to_string(),
            methods: {
                let mut m = IndexMap::new();
                m.insert(
                    HttpMethod::Get,
                    EndpointIR {
                        operation_id: "listUsers".to_string(),
                        summary: None,
                        handler_ref: "listUsers".to_string(),
                        middleware: vec!["auth".to_string()],
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
        assert!(code.contains("router.get(\"/api/users\", auth, listUsers);"));
    }

    #[test]
    fn test_emit_entrypoint() {
        let adapter = ExpressAdapter;
        let project = ProjectIR {
            config: serde_json::json!({
                "name": "test-app",
                "server": { "port": 8080 }
            }),
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        };

        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        let code = adapter.emit_entrypoint(&project, &mut ctx);
        assert!(code.contains("express()"));
        assert!(code.contains("8080"));
        assert!(code.contains("test-app"));
    }

    #[test]
    fn test_prisma_schema_generation() {
        let project = ProjectIR {
            config: serde_json::json!({
                "database": { "type": "postgresql" }
            }),
            routes: vec![],
            schemas: vec![],
            models: vec![rash_ir::types::ModelIR {
                name: "User".to_string(),
                table_name: "users".to_string(),
                columns: {
                    let mut c = IndexMap::new();
                    c.insert(
                        "id".to_string(),
                        serde_json::json!({ "type": "uuid", "primaryKey": true, "default": "uuid" }),
                    );
                    c.insert(
                        "email".to_string(),
                        serde_json::json!({ "type": "string", "unique": true }),
                    );
                    c
                },
                relations: IndexMap::new(),
                indexes: vec![],
            }],
            middleware: vec![],
            handlers: vec![],
        };

        let schema = generate_prisma_schema(&project);
        assert!(schema.contains("model User {"));
        assert!(schema.contains("id String @id @default(uuid())"));
        assert!(schema.contains("email String @unique"));
        assert!(schema.contains("provider = \"postgresql\""));
    }
}
