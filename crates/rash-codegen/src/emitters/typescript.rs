use rash_spec::types::common::Language;

use crate::context::{EmitContext, IndentStyle};
use crate::traits::LanguageEmitter;
use rash_ir::expr::{ExprIR, TypeIR};
use rash_ir::statement::StatementIR;
use rash_ir::types::{ModelIR, SchemaIR};

/// TypeScript code emitter.
pub struct TypeScriptEmitter;

impl LanguageEmitter for TypeScriptEmitter {
    fn language(&self) -> Language {
        Language::Typescript
    }

    fn emit_statement(&self, stmt: &StatementIR, ctx: &mut EmitContext) -> String {
        let ind = ctx.indent();
        match stmt {
            StatementIR::Let { name, type_, value } => {
                let type_ann = type_
                    .as_ref()
                    .map(|t| format!(": {}", self.emit_type(t)))
                    .unwrap_or_default();
                let val = self.emit_expression(value, ctx);
                format!("{}const {}{} = {};", ind, name, type_ann, val)
            }
            StatementIR::Assign { target, value } => {
                let t = self.emit_expression(target, ctx);
                let v = self.emit_expression(value, ctx);
                format!("{}{} = {};", ind, t, v)
            }
            StatementIR::Return { value } => match value {
                Some(expr) => {
                    let v = self.emit_expression(expr, ctx);
                    format!("{}return {};", ind, v)
                }
                None => format!("{}return;", ind),
            },
            StatementIR::If {
                condition,
                then_,
                else_,
            } => {
                let cond = self.emit_expression(condition, ctx);
                let then_body = self.emit_block(then_, ctx);
                match else_ {
                    Some(else_stmts) => {
                        let else_body = self.emit_block(else_stmts, ctx);
                        format!(
                            "{}if ({}) {{\n{}\n{}}} else {{\n{}\n{}}}",
                            ind, cond, then_body, ind, else_body, ind
                        )
                    }
                    None => {
                        format!("{}if ({}) {{\n{}\n{}}}", ind, cond, then_body, ind)
                    }
                }
            }
            StatementIR::For {
                binding,
                iterable,
                body,
            } => {
                let iter = self.emit_expression(iterable, ctx);
                let body_code = self.emit_block(body, ctx);
                format!(
                    "{}for (const {} of {}) {{\n{}\n{}}}",
                    ind, binding, iter, body_code, ind
                )
            }
            StatementIR::While { condition, body } => {
                let cond = self.emit_expression(condition, ctx);
                let body_code = self.emit_block(body, ctx);
                format!("{}while ({}) {{\n{}\n{}}}", ind, cond, body_code, ind)
            }
            StatementIR::Match { expr, arms } => {
                let val = self.emit_expression(expr, ctx);
                let mut lines = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    let pattern = self.emit_expression(&arm.pattern, ctx);
                    let body_code = self.emit_block(&arm.body, ctx);
                    if i == 0 {
                        lines.push(format!(
                            "{}if ({} === {}) {{\n{}\n{}}}",
                            ind, val, pattern, body_code, ind
                        ));
                    } else {
                        lines.push(format!(
                            " else if ({} === {}) {{\n{}\n{}}}",
                            val, pattern, body_code, ind
                        ));
                    }
                }
                lines.join("")
            }
            StatementIR::TryCatch {
                try_,
                catch_,
                finally_,
            } => {
                let try_body = self.emit_block(try_, ctx);
                let catch_body = self.emit_block(&catch_.body, ctx);
                let mut result = format!(
                    "{}try {{\n{}\n{}}} catch ({}) {{\n{}\n{}}}",
                    ind, try_body, ind, catch_.binding, catch_body, ind
                );
                if let Some(finally_stmts) = finally_ {
                    let finally_body = self.emit_block(finally_stmts, ctx);
                    result.push_str(&format!(" finally {{\n{}\n{}}}", finally_body, ind));
                }
                result
            }
            StatementIR::Throw { value } => {
                let v = self.emit_expression(value, ctx);
                format!("{}throw {};", ind, v)
            }
            StatementIR::Expression { expr } => {
                let e = self.emit_expression(expr, ctx);
                format!("{}{};", ind, e)
            }
        }
    }

    fn emit_expression(&self, expr: &ExprIR, ctx: &mut EmitContext) -> String {
        match expr {
            ExprIR::Literal { value } => emit_json_literal(value),
            ExprIR::Identifier { name } => name.clone(),
            ExprIR::Binary { op, left, right } => {
                let l = self.emit_expression(left, ctx);
                let r = self.emit_expression(right, ctx);
                format!("{} {} {}", l, op, r)
            }
            ExprIR::Unary { op, operand } => {
                let o = self.emit_expression(operand, ctx);
                format!("{}{}", op, o)
            }
            ExprIR::Call { callee, args } => {
                let c = self.emit_expression(callee, ctx);
                let a: Vec<String> = args.iter().map(|a| self.emit_expression(a, ctx)).collect();
                format!("{}({})", c, a.join(", "))
            }
            ExprIR::Member { object, property } => {
                let obj = self.emit_expression(object, ctx);
                format!("{}.{}", obj, property)
            }
            ExprIR::Index { object, index } => {
                let obj = self.emit_expression(object, ctx);
                let idx = self.emit_expression(index, ctx);
                format!("{}[{}]", obj, idx)
            }
            ExprIR::Object { properties } => {
                if properties.is_empty() {
                    return "{}".to_string();
                }
                let entries: Vec<String> = properties
                    .iter()
                    .map(|(k, v)| {
                        let val = self.emit_expression(v, ctx);
                        format!("{}: {}", k, val)
                    })
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            }
            ExprIR::Array { elements } => {
                let elems: Vec<String> =
                    elements.iter().map(|e| self.emit_expression(e, ctx)).collect();
                format!("[{}]", elems.join(", "))
            }
            ExprIR::ArrowFn { params, body } => {
                let params_str = params.join(", ");
                let body_code = self.emit_block(body, ctx);
                format!("({}) => {{\n{}\n{}}}", params_str, body_code, ctx.indent())
            }
            ExprIR::Await { expr } => {
                let e = self.emit_expression(expr, ctx);
                format!("await {}", e)
            }
            ExprIR::Pipe { stages } => {
                // Transform pipe into nested calls: a |> b |> c → c(b(a))
                if stages.is_empty() {
                    return "undefined".to_string();
                }
                let mut result = self.emit_expression(&stages[0], ctx);
                for stage in &stages[1..] {
                    let f = self.emit_expression(stage, ctx);
                    result = format!("{}({})", f, result);
                }
                result
            }
            ExprIR::Template { parts } => {
                let mut s = String::from("`");
                for part in parts {
                    match part {
                        rash_ir::expr::TemplatePartIR::Text { value } => s.push_str(value),
                        rash_ir::expr::TemplatePartIR::Expr { value } => {
                            let e = self.emit_expression(value, ctx);
                            s.push_str(&format!("${{{}}}", e));
                        }
                    }
                }
                s.push('`');
                s
            }
            // Domain nodes (Tier 1)
            ExprIR::DbQuery(q) => {
                ctx.add_import("{ prisma }", "./prisma");
                let model = to_camel_case(&q.model);
                let op = &q.operation;
                let mut args = String::new();
                let mut arg_parts = Vec::new();
                if let Some(w) = &q.r#where {
                    arg_parts.push(format!("where: {}", emit_json_literal(w)));
                }
                if let Some(ob) = &q.order_by {
                    arg_parts.push(format!("orderBy: {}", emit_json_literal(ob)));
                }
                if let Some(skip) = &q.skip {
                    arg_parts.push(format!("skip: {}", self.emit_expression(skip, ctx)));
                }
                if let Some(take) = &q.take {
                    arg_parts.push(format!("take: {}", self.emit_expression(take, ctx)));
                }
                if let Some(sel) = &q.select {
                    let fields: Vec<String> = sel.iter().map(|f| format!("{}: true", f)).collect();
                    arg_parts.push(format!("select: {{ {} }}", fields.join(", ")));
                }
                if let Some(inc) = &q.include {
                    arg_parts.push(format!("include: {}", emit_json_literal(inc)));
                }
                if !arg_parts.is_empty() {
                    args = format!("{{ {} }}", arg_parts.join(", "));
                }
                format!("prisma.{}.{}({})", model, op, args)
            }
            ExprIR::DbMutate(m) => {
                ctx.add_import("{ prisma }", "./prisma");
                let model = to_camel_case(&m.model);
                let op = &m.operation;
                let mut arg_parts = Vec::new();
                if let Some(data) = &m.data {
                    arg_parts.push(format!("data: {}", self.emit_expression(data, ctx)));
                }
                if let Some(w) = &m.r#where {
                    arg_parts.push(format!("where: {}", emit_json_literal(w)));
                }
                let args = if arg_parts.is_empty() {
                    String::new()
                } else {
                    format!("{{ {} }}", arg_parts.join(", "))
                };
                format!("prisma.{}.{}({})", model, op, args)
            }
            ExprIR::HttpRespond(r) => {
                let body = r
                    .body
                    .as_ref()
                    .map(|b| self.emit_expression(b, ctx))
                    .unwrap_or_else(|| "undefined".to_string());
                format!("res.status({}).json({})", r.status, body)
            }
            ExprIR::CtxGet { path } => {
                // Map ctx paths to Express req accessors
                let parts: Vec<&str> = path.splitn(2, '.').collect();
                match parts.as_slice() {
                    ["params", rest] => format!("req.params.{}", rest),
                    ["query", rest] => format!("req.query.{}", rest),
                    ["body"] => "req.body".to_string(),
                    ["body", rest] => format!("req.body.{}", rest),
                    ["headers", rest] => format!("req.headers[\"{}\"]", rest),
                    _ => format!("req.{}", path),
                }
            }
            ExprIR::Validate { schema, data } => {
                ctx.add_import(
                    format!("{{ {} }}", schema),
                    format!("./schemas/{}", schema.to_lowercase()),
                );
                let d = self.emit_expression(data, ctx);
                format!("{}.parse({})", schema, d)
            }
            ExprIR::HashPassword {
                input,
                algorithm,
                rounds,
            } => {
                let i = self.emit_expression(input, ctx);
                match algorithm.as_str() {
                    "bcrypt" => {
                        ctx.add_import("bcrypt", "bcrypt");
                        let r = rounds.unwrap_or(10);
                        format!("bcrypt.hash({}, {})", i, r)
                    }
                    _ => format!("hashPassword({}, \"{}\")", i, algorithm),
                }
            }
            ExprIR::VerifyPassword {
                password,
                hash,
                algorithm,
            } => {
                let p = self.emit_expression(password, ctx);
                let h = self.emit_expression(hash, ctx);
                match algorithm.as_str() {
                    "bcrypt" => {
                        ctx.add_import("bcrypt", "bcrypt");
                        format!("bcrypt.compare({}, {})", p, h)
                    }
                    _ => format!("verifyPassword({}, {}, \"{}\")", p, h, algorithm),
                }
            }
            ExprIR::SignToken { payload, options } => {
                ctx.add_import("jwt", "jsonwebtoken");
                let p = self.emit_expression(payload, ctx);
                match options {
                    Some(opts) => {
                        format!(
                            "jwt.sign({}, process.env.JWT_SECRET!, {})",
                            p,
                            emit_json_literal(opts)
                        )
                    }
                    None => format!("jwt.sign({}, process.env.JWT_SECRET!)", p),
                }
            }
            ExprIR::VerifyToken { token } => {
                ctx.add_import("jwt", "jsonwebtoken");
                let t = self.emit_expression(token, ctx);
                format!("jwt.verify({}, process.env.JWT_SECRET!)", t)
            }
            ExprIR::NativeBridge(nb) => {
                ctx.add_import(&nb.import.name, &nb.import.from);
                let args: Vec<String> = nb
                    .call
                    .args
                    .iter()
                    .map(|a| self.emit_expression(a, ctx))
                    .collect();
                format!("{}({})", nb.call.method, args.join(", "))
            }
        }
    }

    fn emit_type(&self, type_ir: &TypeIR) -> String {
        match type_ir {
            TypeIR::String => "string".to_string(),
            TypeIR::Number => "number".to_string(),
            TypeIR::Boolean => "boolean".to_string(),
            TypeIR::Null => "null".to_string(),
            TypeIR::Void => "void".to_string(),
            TypeIR::Any => "any".to_string(),
            TypeIR::Array { inner } => format!("{}[]", self.emit_type(inner)),
            TypeIR::Optional { inner } => format!("{} | null", self.emit_type(inner)),
            TypeIR::Ref { name } => name.clone(),
            TypeIR::Object { fields } => {
                let entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.emit_type(v)))
                    .collect();
                format!("{{ {} }}", entries.join("; "))
            }
            TypeIR::Union { variants } => {
                let types: Vec<String> = variants.iter().map(|v| self.emit_type(v)).collect();
                types.join(" | ")
            }
        }
    }

    fn emit_schema(&self, schema: &SchemaIR, ctx: &mut EmitContext) -> String {
        ctx.add_import("{ z }", "zod");
        let mut lines = Vec::new();
        for (name, value) in &schema.definitions {
            let zod_code = json_schema_to_zod(value);
            lines.push(format!("export const {} = {};", name, zod_code));
            lines.push(format!(
                "export type {} = z.infer<typeof {}>;",
                name, name
            ));
            lines.push(String::new());
        }
        lines.join("\n")
    }

    fn emit_model(&self, model: &ModelIR, ctx: &mut EmitContext) -> String {
        // For TypeScript + Prisma, models are defined in schema.prisma
        // Here we generate TypeScript type definitions
        let _ = ctx;
        let mut lines = Vec::new();
        lines.push(format!("export interface {} {{", model.name));
        for (col_name, col_def) in &model.columns {
            let ts_type = prisma_col_to_ts_type(col_def);
            lines.push(format!("  {}: {};", col_name, ts_type));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn emit_imports(&self, ctx: &mut EmitContext) -> String {
        let imports = ctx.take_imports();
        let mut lines: Vec<String> = imports
            .iter()
            .map(|imp| format!("import {} from \"{}\";", imp.names, imp.from))
            .collect();
        lines.sort();
        lines.join("\n")
    }

    fn file_extension(&self) -> &str {
        "ts"
    }

    fn indent_style(&self) -> IndentStyle {
        IndentStyle::Spaces(2)
    }
}

impl TypeScriptEmitter {
    /// Emit a block of statements with increased indentation.
    fn emit_block(&self, stmts: &[StatementIR], ctx: &mut EmitContext) -> String {
        ctx.push_indent();
        let lines: Vec<String> = stmts.iter().map(|s| self.emit_statement(s, ctx)).collect();
        ctx.pop_indent();
        lines.join("\n")
    }
}

/// Convert a serde_json::Value to a TypeScript literal string.
fn emit_json_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        serde_json::Value::Array(arr) => {
            let elems: Vec<String> = arr.iter().map(emit_json_literal).collect();
            format!("[{}]", elems.join(", "))
        }
        serde_json::Value::Object(map) => {
            let entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, emit_json_literal(v)))
                .collect();
            format!("{{ {} }}", entries.join(", "))
        }
    }
}

/// Convert a JSON Schema value to a Zod schema expression string.
fn json_schema_to_zod(schema: &serde_json::Value) -> String {
    let obj = match schema.as_object() {
        Some(o) => o,
        None => return "z.any()".to_string(),
    };

    let type_val = obj.get("type").and_then(|t| t.as_str());

    match type_val {
        Some("string") => {
            let mut chain = "z.string()".to_string();
            if let Some(serde_json::Value::String(fmt)) = obj.get("format") {
                match fmt.as_str() {
                    "email" => chain.push_str(".email()"),
                    "uuid" => chain.push_str(".uuid()"),
                    "url" | "uri" => chain.push_str(".url()"),
                    _ => {}
                }
            }
            if let Some(min) = obj.get("minLength").and_then(|v| v.as_u64()) {
                chain.push_str(&format!(".min({})", min));
            }
            if let Some(max) = obj.get("maxLength").and_then(|v| v.as_u64()) {
                chain.push_str(&format!(".max({})", max));
            }
            chain
        }
        Some("number") | Some("integer") => {
            let base = if type_val == Some("integer") {
                "z.number().int()"
            } else {
                "z.number()"
            };
            let mut chain = base.to_string();
            if let Some(min) = obj.get("minimum").and_then(|v| v.as_f64()) {
                chain.push_str(&format!(".min({})", min));
            }
            if let Some(max) = obj.get("maximum").and_then(|v| v.as_f64()) {
                chain.push_str(&format!(".max({})", max));
            }
            chain
        }
        Some("boolean") => "z.boolean()".to_string(),
        Some("array") => {
            let items = obj
                .get("items")
                .map(json_schema_to_zod)
                .unwrap_or_else(|| "z.any()".to_string());
            format!("z.array({})", items)
        }
        Some("object") => {
            let properties = obj.get("properties").and_then(|p| p.as_object());
            let required: Vec<String> = obj
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            match properties {
                Some(props) => {
                    let fields: Vec<String> = props
                        .iter()
                        .map(|(k, v)| {
                            let mut zod = json_schema_to_zod(v);
                            if !required.contains(&k.to_string()) {
                                zod.push_str(".optional()");
                            }
                            format!("  {}: {}", k, zod)
                        })
                        .collect();
                    format!("z.object({{\n{}\n}})", fields.join(",\n"))
                }
                None => "z.object({})".to_string(),
            }
        }
        Some("null") => "z.null()".to_string(),
        _ => {
            // Check for enum
            if let Some(enum_vals) = obj.get("enum").and_then(|e| e.as_array()) {
                let vals: Vec<String> = enum_vals.iter().map(emit_json_literal).collect();
                if vals.len() == 1 {
                    format!("z.literal({})", vals[0])
                } else {
                    let literals: Vec<String> =
                        vals.iter().map(|v| format!("z.literal({})", v)).collect();
                    format!("z.union([{}])", literals.join(", "))
                }
            } else if obj.contains_key("$ref") || obj.contains_key("ref") {
                // Reference to another schema — use lazy reference
                let ref_name = obj
                    .get("$ref")
                    .or_else(|| obj.get("ref"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                // Extract name from #/definitions/Name format
                let name = ref_name
                    .rsplit('/')
                    .next()
                    .unwrap_or(ref_name);
                format!("z.lazy(() => {})", name)
            } else {
                "z.any()".to_string()
            }
        }
    }
}

/// Convert a Prisma column definition to a TypeScript type.
fn prisma_col_to_ts_type(col_def: &serde_json::Value) -> String {
    let type_str = col_def
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("any");
    let nullable = col_def
        .get("nullable")
        .and_then(|n| n.as_bool())
        .unwrap_or(false);

    let base = match type_str {
        "string" | "text" | "varchar" | "uuid" => "string",
        "integer" | "int" | "bigint" | "serial" => "number",
        "float" | "double" | "decimal" | "numeric" => "number",
        "boolean" | "bool" => "boolean",
        "datetime" | "timestamp" | "date" => "Date",
        "json" | "jsonb" => "unknown",
        _ => type_str,
    };

    if nullable {
        format!("{} | null", base)
    } else {
        base.to_string()
    }
}

/// Convert PascalCase/snake_case model name to camelCase for Prisma client.
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        result.push(first.to_lowercase().next().unwrap_or(first));
    }
    result.extend(chars);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rash_ir::expr::HttpRespondIR;

    #[test]
    fn test_emit_let_statement() {
        let emitter = TypeScriptEmitter;
        let stmt = StatementIR::Let {
            name: "userId".to_string(),
            type_: Some(TypeIR::String),
            value: ExprIR::literal(serde_json::json!("abc")),
        };
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        let code = emitter.emit_statement(&stmt, &mut ctx);
        assert_eq!(code, "const userId: string = \"abc\";");
    }

    #[test]
    fn test_emit_if_statement() {
        let emitter = TypeScriptEmitter;
        let stmt = StatementIR::If {
            condition: ExprIR::Binary {
                op: "==".to_string(),
                left: Box::new(ExprIR::ident("user")),
                right: Box::new(ExprIR::literal(serde_json::Value::Null)),
            },
            then_: vec![StatementIR::Return {
                value: Some(ExprIR::HttpRespond(HttpRespondIR {
                    status: 404,
                    headers: None,
                    body: Some(Box::new(ExprIR::Object {
                        properties: vec![(
                            "message".to_string(),
                            ExprIR::literal(serde_json::json!("Not found")),
                        )],
                    })),
                })),
            }],
            else_: None,
        };
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        let code = emitter.emit_statement(&stmt, &mut ctx);
        assert!(code.contains("if (user == null)"));
        assert!(code.contains("res.status(404).json"));
    }

    #[test]
    fn test_emit_type() {
        let emitter = TypeScriptEmitter;
        assert_eq!(emitter.emit_type(&TypeIR::String), "string");
        assert_eq!(emitter.emit_type(&TypeIR::Number), "number");
        assert_eq!(
            emitter.emit_type(&TypeIR::Array {
                inner: Box::new(TypeIR::String)
            }),
            "string[]"
        );
        assert_eq!(
            emitter.emit_type(&TypeIR::Optional {
                inner: Box::new(TypeIR::Ref {
                    name: "User".to_string()
                })
            }),
            "User | null"
        );
    }

    #[test]
    fn test_emit_ctx_get() {
        let emitter = TypeScriptEmitter;
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        assert_eq!(
            emitter.emit_expression(
                &ExprIR::CtxGet {
                    path: "params.id".to_string()
                },
                &mut ctx
            ),
            "req.params.id"
        );
        assert_eq!(
            emitter.emit_expression(
                &ExprIR::CtxGet {
                    path: "body".to_string()
                },
                &mut ctx
            ),
            "req.body"
        );
    }

    #[test]
    fn test_json_schema_to_zod_basic() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "email": { "type": "string", "format": "email" },
                "age": { "type": "integer", "minimum": 0 }
            },
            "required": ["email"]
        });
        let zod = json_schema_to_zod(&schema);
        assert!(zod.contains("z.object("));
        assert!(zod.contains("email: z.string().email()"));
        assert!(zod.contains("age: z.number().int().min(0).optional()"));
    }

    #[test]
    fn test_emit_template_string() {
        let emitter = TypeScriptEmitter;
        let expr = ExprIR::Template {
            parts: vec![
                rash_ir::expr::TemplatePartIR::Text {
                    value: "Hello ".to_string(),
                },
                rash_ir::expr::TemplatePartIR::Expr {
                    value: Box::new(ExprIR::ident("name")),
                },
            ],
        };
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        let code = emitter.emit_expression(&expr, &mut ctx);
        assert_eq!(code, "`Hello ${name}`");
    }
}
