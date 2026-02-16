use rash_spec::types::common::Language;

use crate::context::{EmitContext, IndentStyle};
use crate::traits::LanguageEmitter;
use rash_ir::expr::{ExprIR, TypeIR};
use rash_ir::statement::StatementIR;
use rash_ir::types::{ModelIR, SchemaIR};

/// Rust code emitter.
pub struct RustEmitter;

impl LanguageEmitter for RustEmitter {
    fn language(&self) -> Language {
        Language::Rust
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
                format!("{}let {}{} = {};", ind, name, type_ann, val)
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
                            "{}if {} {{\n{}\n{}}} else {{\n{}\n{}}}",
                            ind, cond, then_body, ind, else_body, ind
                        )
                    }
                    None => format!("{}if {} {{\n{}\n{}}}", ind, cond, then_body, ind),
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
                    "{}for {} in {} {{\n{}\n{}}}",
                    ind, binding, iter, body_code, ind
                )
            }
            StatementIR::While { condition, body } => {
                let cond = self.emit_expression(condition, ctx);
                let body_code = self.emit_block(body, ctx);
                format!("{}while {} {{\n{}\n{}}}", ind, cond, body_code, ind)
            }
            StatementIR::Match { expr, arms } => {
                let val = self.emit_expression(expr, ctx);
                let mut lines = vec![format!("{}match {} {{", ind, val)];
                ctx.push_indent();
                for arm in arms {
                    let pattern = self.emit_expression(&arm.pattern, ctx);
                    let body_code = self.emit_block(&arm.body, ctx);
                    lines.push(format!(
                        "{}{} => {{\n{}\n{}}}",
                        ctx.indent(),
                        pattern,
                        body_code,
                        ctx.indent()
                    ));
                }
                ctx.pop_indent();
                lines.push(format!("{}}}", ind));
                lines.join("\n")
            }
            StatementIR::TryCatch { .. } => {
                format!("{}// TODO: error handling (Rust uses Result<T, E>)", ind)
            }
            StatementIR::Throw { value } => {
                let v = self.emit_expression(value, ctx);
                format!("{}return Err({}.into());", ind, v)
            }
            StatementIR::Expression { expr } => {
                let e = self.emit_expression(expr, ctx);
                format!("{}{};", ind, e)
            }
        }
    }

    fn emit_expression(&self, expr: &ExprIR, ctx: &mut EmitContext) -> String {
        match expr {
            ExprIR::Literal { value } => emit_rust_literal(value),
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
                let a: Vec<String> =
                    args.iter().map(|a| self.emit_expression(a, ctx)).collect();
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
                let entries: Vec<String> = properties
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.emit_expression(v, ctx)))
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            }
            ExprIR::Array { elements } => {
                let elems: Vec<String> =
                    elements.iter().map(|e| self.emit_expression(e, ctx)).collect();
                format!("vec![{}]", elems.join(", "))
            }
            ExprIR::ArrowFn { params, body } => {
                let params_str = params.join(", ");
                let body_code = self.emit_block(body, ctx);
                format!("|{}| {{\n{}\n{}}}", params_str, body_code, ctx.indent())
            }
            ExprIR::Await { expr } => {
                let e = self.emit_expression(expr, ctx);
                format!("{}.await", e)
            }
            ExprIR::Pipe { stages } => {
                if stages.is_empty() {
                    return "()".to_string();
                }
                let mut result = self.emit_expression(&stages[0], ctx);
                for stage in &stages[1..] {
                    let f = self.emit_expression(stage, ctx);
                    result = format!("{}({})", f, result);
                }
                result
            }
            ExprIR::Template { parts } => {
                let mut fmt_str = String::new();
                let mut args = Vec::new();
                for part in parts {
                    match part {
                        rash_ir::expr::TemplatePartIR::Text { value } => {
                            fmt_str.push_str(value)
                        }
                        rash_ir::expr::TemplatePartIR::Expr { value } => {
                            fmt_str.push_str("{}");
                            args.push(self.emit_expression(value, ctx));
                        }
                    }
                }
                if args.is_empty() {
                    format!("\"{}\"", fmt_str)
                } else {
                    format!("format!(\"{}\", {})", fmt_str, args.join(", "))
                }
            }
            ExprIR::DbQuery(q) => {
                let model = &q.model;
                match q.operation.as_str() {
                    "findUnique" => format!("{}::find_by_id(id).one(&db).await?", model),
                    "findMany" => format!("{}::find().all(&db).await?", model),
                    "count" => format!("{}::find().count(&db).await?", model),
                    _ => format!("{}::{}(&db).await?", model, q.operation),
                }
            }
            ExprIR::DbMutate(m) => {
                let model = &m.model;
                match m.operation.as_str() {
                    "create" => format!("{}::insert(model).exec(&db).await?", model),
                    "update" => format!("{}::update(model).exec(&db).await?", model),
                    "delete" => format!("{}::delete_by_id(id).exec(&db).await?", model),
                    _ => format!("{}::{}(&db).await?", model, m.operation),
                }
            }
            ExprIR::HttpRespond(r) => {
                let body = r
                    .body
                    .as_ref()
                    .map(|b| self.emit_expression(b, ctx))
                    .unwrap_or_else(|| "()".to_string());
                match r.status {
                    200 => format!("HttpResponse::Ok().json({})", body),
                    201 => format!("HttpResponse::Created().json({})", body),
                    204 => "HttpResponse::NoContent().finish()".to_string(),
                    400 => format!("HttpResponse::BadRequest().json({})", body),
                    404 => format!("HttpResponse::NotFound().json({})", body),
                    _ => format!(
                        "HttpResponse::build(StatusCode::from_u16({}).unwrap()).json({})",
                        r.status, body
                    ),
                }
            }
            ExprIR::CtxGet { path } => {
                let parts: Vec<&str> = path.splitn(2, '.').collect();
                match parts.as_slice() {
                    ["params", rest] => {
                        format!("req.match_info().get(\"{}\").unwrap()", rest)
                    }
                    ["query", rest] => format!("query.{}", rest),
                    ["body"] => "body".to_string(),
                    ["body", rest] => format!("body.{}", rest),
                    _ => format!("req.{}", path),
                }
            }
            ExprIR::Validate { schema, data } => {
                let d = self.emit_expression(data, ctx);
                format!("{}::validate(&{})?", schema, d)
            }
            ExprIR::HashPassword { input, .. } => {
                let i = self.emit_expression(input, ctx);
                format!("hash({}, DEFAULT_COST)?", i)
            }
            ExprIR::VerifyPassword {
                password, hash, ..
            } => {
                let p = self.emit_expression(password, ctx);
                let h = self.emit_expression(hash, ctx);
                format!("verify({}, &{})?", p, h)
            }
            ExprIR::SignToken { payload, .. } => {
                let p = self.emit_expression(payload, ctx);
                format!(
                    "encode(&Header::default(), &{}, &EncodingKey::from_secret(secret))?",
                    p
                )
            }
            ExprIR::VerifyToken { token } => {
                let t = self.emit_expression(token, ctx);
                format!(
                    "decode::<Claims>(&{}, &DecodingKey::from_secret(secret), &Validation::default())?",
                    t
                )
            }
            ExprIR::NativeBridge(nb) => {
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
            TypeIR::String => "String".to_string(),
            TypeIR::Number => "f64".to_string(),
            TypeIR::Boolean => "bool".to_string(),
            TypeIR::Null => "()".to_string(),
            TypeIR::Void => "()".to_string(),
            TypeIR::Any => "serde_json::Value".to_string(),
            TypeIR::Array { inner } => format!("Vec<{}>", self.emit_type(inner)),
            TypeIR::Optional { inner } => format!("Option<{}>", self.emit_type(inner)),
            TypeIR::Ref { name } => name.clone(),
            TypeIR::Object { fields } => {
                let entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.emit_type(v)))
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            }
            TypeIR::Union { variants } => {
                if variants.len() == 1 {
                    self.emit_type(&variants[0])
                } else {
                    "serde_json::Value".to_string()
                }
            }
        }
    }

    fn emit_schema(&self, schema: &SchemaIR, _ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        lines.push("use serde::{Deserialize, Serialize};".to_string());
        lines.push(String::new());
        for (name, value) in &schema.definitions {
            lines.push(emit_rust_struct_from_json_schema(name, value));
            lines.push(String::new());
        }
        lines.join("\n")
    }

    fn emit_model(&self, model: &ModelIR, _ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        lines.push("use serde::{Deserialize, Serialize};".to_string());
        lines.push(String::new());
        lines.push("#[derive(Debug, Clone, Serialize, Deserialize)]".to_string());
        lines.push(format!("pub struct {} {{", model.name));
        for (col_name, col_def) in &model.columns {
            let rust_type = json_type_to_rust(
                col_def
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("String"),
            );
            let nullable = col_def
                .get("nullable")
                .and_then(|n| n.as_bool())
                .unwrap_or(false);
            if nullable {
                lines.push(format!("    pub {}: Option<{}>,", col_name, rust_type));
            } else {
                lines.push(format!("    pub {}: {},", col_name, rust_type));
            }
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn emit_imports(&self, ctx: &mut EmitContext) -> String {
        let imports = ctx.take_imports();
        let lines: Vec<String> = imports
            .iter()
            .map(|imp| format!("use {};", imp.from))
            .collect();
        lines.join("\n")
    }

    fn file_extension(&self) -> &str {
        "rs"
    }

    fn indent_style(&self) -> IndentStyle {
        IndentStyle::Spaces(4)
    }
}

impl RustEmitter {
    fn emit_block(&self, stmts: &[StatementIR], ctx: &mut EmitContext) -> String {
        ctx.push_indent();
        let lines: Vec<String> = stmts.iter().map(|s| self.emit_statement(s, ctx)).collect();
        ctx.pop_indent();
        lines.join("\n")
    }
}

fn emit_rust_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => {
            if n.is_f64() {
                format!("{}_f64", n)
            } else {
                n.to_string()
            }
        }
        serde_json::Value::String(s) => {
            format!(
                "\"{}\".to_string()",
                s.replace('\\', "\\\\").replace('"', "\\\"")
            )
        }
        serde_json::Value::Array(arr) => {
            let elems: Vec<String> = arr.iter().map(emit_rust_literal).collect();
            format!("vec![{}]", elems.join(", "))
        }
        serde_json::Value::Object(map) => {
            let entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("(\"{}\", {})", k, emit_rust_literal(v)))
                .collect();
            format!("json!({{ {} }})", entries.join(", "))
        }
    }
}

fn emit_rust_struct_from_json_schema(name: &str, schema: &serde_json::Value) -> String {
    let mut lines = Vec::new();
    lines.push("#[derive(Debug, Clone, Serialize, Deserialize)]".to_string());
    lines.push(format!("pub struct {} {{", name));
    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        let required: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        for (field, field_schema) in props {
            let rust_type = json_schema_type_to_rust(field_schema);
            if required.contains(field) {
                lines.push(format!("    pub {}: {},", field, rust_type));
            } else {
                lines.push(format!("    pub {}: Option<{}>,", field, rust_type));
            }
        }
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn json_schema_type_to_rust(schema: &serde_json::Value) -> String {
    let type_str = schema
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("any");
    json_type_to_rust(type_str)
}

fn json_type_to_rust(type_str: &str) -> String {
    match type_str {
        "string" | "varchar" | "text" | "uuid" => "String".to_string(),
        "number" | "float" | "double" | "decimal" => "f64".to_string(),
        "integer" | "int" | "bigint" | "serial" => "i64".to_string(),
        "boolean" | "bool" => "bool".to_string(),
        "datetime" | "timestamp" | "date" => "chrono::NaiveDateTime".to_string(),
        "array" => "Vec<serde_json::Value>".to_string(),
        "object" => "serde_json::Value".to_string(),
        _ => "serde_json::Value".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_type() {
        let emitter = RustEmitter;
        assert_eq!(emitter.emit_type(&TypeIR::String), "String");
        assert_eq!(emitter.emit_type(&TypeIR::Number), "f64");
        assert_eq!(
            emitter.emit_type(&TypeIR::Array {
                inner: Box::new(TypeIR::String)
            }),
            "Vec<String>"
        );
        assert_eq!(
            emitter.emit_type(&TypeIR::Optional {
                inner: Box::new(TypeIR::Boolean)
            }),
            "Option<bool>"
        );
    }

    #[test]
    fn test_emit_let() {
        let emitter = RustEmitter;
        let stmt = StatementIR::Let {
            name: "x".to_string(),
            type_: Some(TypeIR::String),
            value: ExprIR::literal(serde_json::json!("hello")),
        };
        let mut ctx = EmitContext::new(IndentStyle::Spaces(4));
        let code = emitter.emit_statement(&stmt, &mut ctx);
        assert!(code.contains("let x: String = \"hello\".to_string();"));
    }
}
