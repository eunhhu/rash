use rash_spec::types::common::Language;

use crate::context::{EmitContext, IndentStyle};
use crate::traits::LanguageEmitter;
use rash_ir::expr::{ExprIR, TypeIR};
use rash_ir::statement::StatementIR;
use rash_ir::types::{ModelIR, SchemaIR};

/// Python code emitter.
pub struct PythonEmitter;

impl LanguageEmitter for PythonEmitter {
    fn language(&self) -> Language {
        Language::Python
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
                format!("{}{}{} = {}", ind, name, type_ann, val)
            }
            StatementIR::Assign { target, value } => {
                let t = self.emit_expression(target, ctx);
                let v = self.emit_expression(value, ctx);
                format!("{}{} = {}", ind, t, v)
            }
            StatementIR::Return { value } => match value {
                Some(expr) => {
                    let v = self.emit_expression(expr, ctx);
                    format!("{}return {}", ind, v)
                }
                None => format!("{}return", ind),
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
                            "{}if {}:\n{}\n{}else:\n{}",
                            ind, cond, then_body, ind, else_body
                        )
                    }
                    None => format!("{}if {}:\n{}", ind, cond, then_body),
                }
            }
            StatementIR::For {
                binding,
                iterable,
                body,
            } => {
                let iter = self.emit_expression(iterable, ctx);
                let body_code = self.emit_block(body, ctx);
                format!("{}for {} in {}:\n{}", ind, binding, iter, body_code)
            }
            StatementIR::While { condition, body } => {
                let cond = self.emit_expression(condition, ctx);
                let body_code = self.emit_block(body, ctx);
                format!("{}while {}:\n{}", ind, cond, body_code)
            }
            StatementIR::Match { expr, arms } => {
                let val = self.emit_expression(expr, ctx);
                let mut lines = vec![format!("{}match {}:", ind, val)];
                ctx.push_indent();
                for arm in arms {
                    let pattern = self.emit_expression(&arm.pattern, ctx);
                    let body_code = self.emit_block(&arm.body, ctx);
                    lines.push(format!("{}case {}:", ctx.indent(), pattern));
                    lines.push(body_code);
                }
                ctx.pop_indent();
                lines.join("\n")
            }
            StatementIR::TryCatch {
                try_,
                catch_,
                finally_,
            } => {
                let try_body = self.emit_block(try_, ctx);
                let catch_body = self.emit_block(&catch_.body, ctx);
                let mut result = format!(
                    "{}try:\n{}\n{}except Exception as {}:\n{}",
                    ind, try_body, ind, catch_.binding, catch_body
                );
                if let Some(finally_stmts) = finally_ {
                    let finally_body = self.emit_block(finally_stmts, ctx);
                    result.push_str(&format!("\n{}finally:\n{}", ind, finally_body));
                }
                result
            }
            StatementIR::Throw { value } => {
                let v = self.emit_expression(value, ctx);
                format!("{}raise Exception({})", ind, v)
            }
            StatementIR::Expression { expr } => {
                let e = self.emit_expression(expr, ctx);
                format!("{}{}", ind, e)
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn emit_expression(&self, expr: &ExprIR, ctx: &mut EmitContext) -> String {
        match expr {
            ExprIR::Literal { value } => emit_python_literal(value),
            ExprIR::Identifier { name } => name.clone(),
            ExprIR::Binary { op, left, right } => {
                let l = self.emit_expression(left, ctx);
                let r = self.emit_expression(right, ctx);
                let py_op = match op.as_str() {
                    "&&" => "and",
                    "||" => "or",
                    _ => op,
                };
                format!("{} {} {}", l, py_op, r)
            }
            ExprIR::Unary { op, operand } => {
                let o = self.emit_expression(operand, ctx);
                let py_op = if op == "!" { "not " } else { op };
                format!("{}{}", py_op, o)
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
                    .map(|(k, v)| format!("\"{}\": {}", k, self.emit_expression(v, ctx)))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            ExprIR::Array { elements } => {
                let elems: Vec<String> =
                    elements.iter().map(|e| self.emit_expression(e, ctx)).collect();
                format!("[{}]", elems.join(", "))
            }
            ExprIR::ArrowFn { params, body } => {
                if body.len() == 1 {
                    if let StatementIR::Return {
                        value: Some(ret_expr),
                    } = &body[0]
                    {
                        let e = self.emit_expression(ret_expr, ctx);
                        return format!("lambda {}: {}", params.join(", "), e);
                    }
                }
                format!("lambda {}: None  # complex lambda", params.join(", "))
            }
            ExprIR::Await { expr } => {
                let e = self.emit_expression(expr, ctx);
                format!("await {}", e)
            }
            ExprIR::Pipe { stages } => {
                if stages.is_empty() {
                    return "None".to_string();
                }
                let mut result = self.emit_expression(&stages[0], ctx);
                for stage in &stages[1..] {
                    let f = self.emit_expression(stage, ctx);
                    result = format!("{}({})", f, result);
                }
                result
            }
            ExprIR::Template { parts } => {
                let mut s = String::from("f\"");
                for part in parts {
                    match part {
                        rash_ir::expr::TemplatePartIR::Text { value } => s.push_str(value),
                        rash_ir::expr::TemplatePartIR::Expr { value } => {
                            let e = self.emit_expression(value, ctx);
                            s.push_str(&format!("{{{}}}", e));
                        }
                    }
                }
                s.push('"');
                s
            }
            ExprIR::DbQuery(q) => {
                let model = &q.model;
                match q.operation.as_str() {
                    "findUnique" => format!("await {}.get_or_none(id=id)", model),
                    "findMany" => format!("await {}.filter().all()", model),
                    "count" => format!("await {}.filter().count()", model),
                    _ => format!("await {}.{}()", model, q.operation),
                }
            }
            ExprIR::DbMutate(m) => {
                let model = &m.model;
                match m.operation.as_str() {
                    "create" => format!("await {}.create(**data)", model),
                    "delete" => "await instance.delete()".to_string(),
                    _ => format!("await {}.{}()", model, m.operation),
                }
            }
            ExprIR::HttpRespond(r) => {
                let body = r
                    .body
                    .as_ref()
                    .map(|b| self.emit_expression(b, ctx))
                    .unwrap_or_else(|| "None".to_string());
                format!("JSONResponse(status_code={}, content={})", r.status, body)
            }
            ExprIR::CtxGet { path } => {
                let parts: Vec<&str> = path.splitn(2, '.').collect();
                match parts.as_slice() {
                    ["params", rest] => format!("request.path_params[\"{}\"]", rest),
                    ["query", rest] => format!("request.query_params.get(\"{}\")", rest),
                    ["body"] => "await request.json()".to_string(),
                    ["body", rest] => format!("(await request.json())[\"{}\"]", rest),
                    _ => format!("request.{}", path),
                }
            }
            ExprIR::Validate { schema, data } => {
                let d = self.emit_expression(data, ctx);
                format!("{}.model_validate({})", schema, d)
            }
            ExprIR::HashPassword { input, .. } => {
                let i = self.emit_expression(input, ctx);
                format!("bcrypt.hashpw({}.encode(), bcrypt.gensalt())", i)
            }
            ExprIR::VerifyPassword {
                password, hash, ..
            } => {
                let p = self.emit_expression(password, ctx);
                let h = self.emit_expression(hash, ctx);
                format!("bcrypt.checkpw({}.encode(), {})", p, h)
            }
            ExprIR::SignToken { payload, .. } => {
                let p = self.emit_expression(payload, ctx);
                format!("jwt.encode({}, SECRET_KEY, algorithm=\"HS256\")", p)
            }
            ExprIR::VerifyToken { token } => {
                let t = self.emit_expression(token, ctx);
                format!("jwt.decode({}, SECRET_KEY, algorithms=[\"HS256\"])", t)
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
            TypeIR::String => "str".to_string(),
            TypeIR::Number => "float".to_string(),
            TypeIR::Boolean => "bool".to_string(),
            TypeIR::Null => "None".to_string(),
            TypeIR::Void => "None".to_string(),
            TypeIR::Any => "Any".to_string(),
            TypeIR::Array { inner } => format!("list[{}]", self.emit_type(inner)),
            TypeIR::Optional { inner } => format!("Optional[{}]", self.emit_type(inner)),
            TypeIR::Ref { name } => name.clone(),
            TypeIR::Object { .. } => "dict".to_string(),
            TypeIR::Union { variants } => {
                let types: Vec<String> = variants.iter().map(|v| self.emit_type(v)).collect();
                format!("Union[{}]", types.join(", "))
            }
        }
    }

    fn emit_schema(&self, schema: &SchemaIR, _ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        lines.push("from pydantic import BaseModel".to_string());
        lines.push("from typing import Optional, List".to_string());
        lines.push(String::new());
        for (name, value) in &schema.definitions {
            lines.push(emit_pydantic_model(name, value));
            lines.push(String::new());
        }
        lines.join("\n")
    }

    fn emit_model(&self, model: &ModelIR, _ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        lines.push("from pydantic import BaseModel".to_string());
        lines.push("from typing import Optional".to_string());
        lines.push(String::new());
        lines.push(format!("class {}(BaseModel):", model.name));
        for (col_name, col_def) in &model.columns {
            let py_type = json_type_to_python(
                col_def
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("str"),
            );
            let nullable = col_def
                .get("nullable")
                .and_then(|n| n.as_bool())
                .unwrap_or(false);
            if nullable {
                lines.push(format!("    {}: Optional[{}] = None", col_name, py_type));
            } else {
                lines.push(format!("    {}: {}", col_name, py_type));
            }
        }
        lines.join("\n")
    }

    fn emit_imports(&self, ctx: &mut EmitContext) -> String {
        let imports = ctx.take_imports();
        let lines: Vec<String> = imports
            .iter()
            .map(|imp| format!("from {} import {}", imp.from, imp.names))
            .collect();
        lines.join("\n")
    }

    fn file_extension(&self) -> &str {
        "py"
    }

    fn indent_style(&self) -> IndentStyle {
        IndentStyle::Spaces(4)
    }
}

impl PythonEmitter {
    fn emit_block(&self, stmts: &[StatementIR], ctx: &mut EmitContext) -> String {
        ctx.push_indent();
        let lines: Vec<String> = stmts.iter().map(|s| self.emit_statement(s, ctx)).collect();
        ctx.pop_indent();
        if lines.is_empty() {
            format!("{}pass", ctx.indent())
        } else {
            lines.join("\n")
        }
    }
}

fn emit_python_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(b) => {
            if *b {
                "True"
            } else {
                "False"
            }
            .to_string()
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            format!(
                "\"{}\"",
                s.replace('\\', "\\\\").replace('"', "\\\"")
            )
        }
        serde_json::Value::Array(arr) => {
            let elems: Vec<String> = arr.iter().map(emit_python_literal).collect();
            format!("[{}]", elems.join(", "))
        }
        serde_json::Value::Object(map) => {
            let entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, emit_python_literal(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
    }
}

fn emit_pydantic_model(name: &str, schema: &serde_json::Value) -> String {
    let mut lines = Vec::new();
    lines.push(format!("class {}(BaseModel):", name));
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
            let py_type = json_schema_type_to_python(field_schema);
            if required.contains(field) {
                lines.push(format!("    {}: {}", field, py_type));
            } else {
                lines.push(format!("    {}: Optional[{}] = None", field, py_type));
            }
        }
    }
    if lines.len() == 1 {
        lines.push("    pass".to_string());
    }
    lines.join("\n")
}

fn json_schema_type_to_python(schema: &serde_json::Value) -> String {
    let type_str = schema
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("Any");
    json_type_to_python(type_str)
}

fn json_type_to_python(type_str: &str) -> String {
    match type_str {
        "string" | "varchar" | "text" | "uuid" => "str".to_string(),
        "number" | "float" | "double" | "decimal" => "float".to_string(),
        "integer" | "int" | "bigint" | "serial" => "int".to_string(),
        "boolean" | "bool" => "bool".to_string(),
        "datetime" | "timestamp" | "date" => "datetime".to_string(),
        "array" => "list".to_string(),
        "object" => "dict".to_string(),
        _ => "Any".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_type() {
        let emitter = PythonEmitter;
        assert_eq!(emitter.emit_type(&TypeIR::String), "str");
        assert_eq!(emitter.emit_type(&TypeIR::Number), "float");
        assert_eq!(
            emitter.emit_type(&TypeIR::Array {
                inner: Box::new(TypeIR::String)
            }),
            "list[str]"
        );
    }

    #[test]
    fn test_emit_if_python() {
        let emitter = PythonEmitter;
        let stmt = StatementIR::If {
            condition: ExprIR::Binary {
                op: "==".to_string(),
                left: Box::new(ExprIR::ident("user")),
                right: Box::new(ExprIR::literal(serde_json::Value::Null)),
            },
            then_: vec![StatementIR::Return {
                value: Some(ExprIR::literal(serde_json::json!(404))),
            }],
            else_: None,
        };
        let mut ctx = EmitContext::new(IndentStyle::Spaces(4));
        let code = emitter.emit_statement(&stmt, &mut ctx);
        assert!(code.contains("if user == None:"));
        assert!(code.contains("return 404"));
    }
}
