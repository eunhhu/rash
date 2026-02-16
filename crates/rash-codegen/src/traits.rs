use rash_spec::types::common::{Framework, Language};

use crate::context::{EmitContext, IndentStyle};
use rash_ir::expr::{ExprIR, TypeIR};
use rash_ir::statement::StatementIR;
use rash_ir::types::{
    HandlerIR, MiddlewareIR, ModelIR, ProjectIR, RouteIR, SchemaIR,
};

/// Language-specific code emitter.
///
/// Each target language implements this trait to convert IR nodes
/// into syntactically correct code strings.
pub trait LanguageEmitter {
    /// Which language this emitter targets.
    fn language(&self) -> Language;

    /// Convert a statement IR node to a code string.
    fn emit_statement(&self, stmt: &StatementIR, ctx: &mut EmitContext) -> String;

    /// Convert an expression IR node to a code string.
    fn emit_expression(&self, expr: &ExprIR, ctx: &mut EmitContext) -> String;

    /// Convert a type IR to the language's type annotation string.
    fn emit_type(&self, type_ir: &TypeIR) -> String;

    /// Generate DTO/validation code for a schema (e.g., Zod for TS).
    fn emit_schema(&self, schema: &SchemaIR, ctx: &mut EmitContext) -> String;

    /// Generate ORM model code (e.g., Prisma model definition).
    fn emit_model(&self, model: &ModelIR, ctx: &mut EmitContext) -> String;

    /// Generate import statements from collected imports.
    fn emit_imports(&self, ctx: &mut EmitContext) -> String;

    /// File extension for this language (e.g., "ts", "rs", "py", "go").
    fn file_extension(&self) -> &str;

    /// Indentation style for this language.
    fn indent_style(&self) -> IndentStyle;
}

/// Request context access pattern — how the framework exposes
/// request data (params, query, body, headers) to handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtxAccessPattern {
    /// Express-style: separate `req` and `res` objects
    ReqRes,
    /// Single context object (e.g., Hono `c`, Gin `c`)
    SingleContext,
    /// Actix-style: extractors in function params
    Extractors,
    /// FastAPI-style: function parameter injection
    ParamInjection,
}

/// Framework-specific adapter that wraps language-emitted code
/// into the framework's conventions and project structure.
pub trait FrameworkAdapter {
    /// Which framework this adapter targets.
    fn framework(&self) -> Framework;

    /// Which language this framework is compatible with.
    fn compatible_language(&self) -> Language;

    /// Generate route registration code for a single route.
    fn emit_route_registration(
        &self,
        route: &RouteIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String;

    /// Generate middleware application code.
    fn emit_middleware_apply(&self, mw_ref: &str, ctx: &mut EmitContext) -> String;

    /// Generate handler function wrapper (signature + body).
    fn emit_handler(
        &self,
        handler: &HandlerIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String;

    /// Generate the application entrypoint file.
    fn emit_entrypoint(&self, project: &ProjectIR, ctx: &mut EmitContext) -> String;

    /// Generate project configuration files (package.json, Cargo.toml, etc.).
    /// Returns a list of (relative_path, content) pairs.
    fn emit_project_config(&self, project: &ProjectIR) -> Vec<(String, String)>;

    /// Optionally override how a domain expression is emitted for this framework.
    /// Returns `None` to fall back to the language emitter's default.
    fn emit_domain_expr(
        &self,
        expr: &ExprIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> Option<String>;

    /// Generate middleware definition code.
    fn emit_middleware_def(
        &self,
        mw: &MiddlewareIR,
        emitter: &dyn LanguageEmitter,
        ctx: &mut EmitContext,
    ) -> String;

    /// How this framework exposes request context to handlers.
    fn ctx_access_pattern(&self) -> CtxAccessPattern;

    /// Generate global middleware application code for the entrypoint.
    /// Returns a list of code lines to insert into the entrypoint.
    fn emit_global_middleware(
        &self,
        middleware_refs: &[String],
        ctx: &mut EmitContext,
    ) -> Vec<String> {
        middleware_refs
            .iter()
            .map(|mw_ref| self.emit_middleware_apply(mw_ref, ctx))
            .collect()
    }

    /// Wrap the collected route registration blocks into a complete route file.
    /// `route_blocks` is the concatenated output of all `emit_route_registration` calls.
    /// `imports` is the collected import statements.
    /// Default: just concatenate imports + route blocks.
    fn wrap_route_file(&self, imports: &str, route_blocks: &str, _ctx: &mut EmitContext) -> String {
        if imports.is_empty() {
            route_blocks.to_string()
        } else {
            format!("{}\n\n{}", imports, route_blocks)
        }
    }

    /// Convert a path parameter from the canonical `:param` format
    /// to the framework-specific format. Default: pass through unchanged.
    fn normalize_path(&self, path: &str) -> String {
        path.to_string()
    }
}
