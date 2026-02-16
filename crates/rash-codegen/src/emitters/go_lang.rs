use rash_spec::types::common::Language;

use crate::context::{EmitContext, IndentStyle};
use crate::traits::LanguageEmitter;
use rash_ir::expr::{ExprIR, TemplatePartIR, TypeIR};
use rash_ir::statement::StatementIR;
use rash_ir::types::{ModelIR, SchemaIR};

/// Go language emitter.
pub struct GoEmitter;

impl LanguageEmitter for GoEmitter {
    fn language(&self) -> Language {
        Language::Go
    }

    fn emit_statement(&self, stmt: &StatementIR, ctx: &mut EmitContext) -> String {
        let indent = ctx.indent();
        match stmt {
            StatementIR::Let { name, value, .. } => {
                format!("{}{} := {}", indent, name, self.emit_expression(value, ctx))
            }
            StatementIR::Assign { target, value } => {
                format!(
                    "{}{} = {}",
                    indent,
                    self.emit_expression(target, ctx),
                    self.emit_expression(value, ctx)
                )
            }
            StatementIR::Return { value } => match value {
                Some(v) => format!("{}return {}", indent, self.emit_expression(v, ctx)),
                None => format!("{}return", indent),
            },
            StatementIR::If {
                condition,
                then_,
                else_,
            } => {
                let cond = self.emit_expression(condition, ctx);
                let mut lines = vec![format!("{}if {} {{", indent, cond)];
                ctx.push_indent();
                for s in then_ {
                    lines.push(self.emit_statement(s, ctx));
                }
                ctx.pop_indent();
                if let Some(else_body) = else_ {
                    lines.push(format!("{}}} else {{", indent));
                    ctx.push_indent();
                    for s in else_body {
                        lines.push(self.emit_statement(s, ctx));
                    }
                    ctx.pop_indent();
                }
                lines.push(format!("{}}}", indent));
                lines.join("\n")
            }
            StatementIR::For {
                binding,
                iterable,
                body,
            } => {
                let iter = self.emit_expression(iterable, ctx);
                let mut lines = vec![format!(
                    "{}for _, {} := range {} {{",
                    indent, binding, iter
                )];
                ctx.push_indent();
                for s in body {
                    lines.push(self.emit_statement(s, ctx));
                }
                ctx.pop_indent();
                lines.push(format!("{}}}", indent));
                lines.join("\n")
            }
            StatementIR::While { condition, body } => {
                let cond = self.emit_expression(condition, ctx);
                let mut lines = vec![format!("{}for {} {{", indent, cond)];
                ctx.push_indent();
                for s in body {
                    lines.push(self.emit_statement(s, ctx));
                }
                ctx.pop_indent();
                lines.push(format!("{}}}", indent));
                lines.join("\n")
            }
            StatementIR::Match { expr, arms } => {
                let val = self.emit_expression(expr, ctx);
                let mut lines = vec![format!("{}switch {} {{", indent, val)];
                for arm in arms {
                    let pattern = self.emit_expression(&arm.pattern, ctx);
                    lines.push(format!("{}case {}:", indent, pattern));
                    ctx.push_indent();
                    for s in &arm.body {
                        lines.push(self.emit_statement(s, ctx));
                    }
                    ctx.pop_indent();
                }
                lines.push(format!("{}}}", indent));
                lines.join("\n")
            }
            StatementIR::TryCatch {
                try_,
                catch_,
                finally_,
            } => {
                // Go doesn't have try/catch — emit as function with recover
                let mut lines = Vec::new();
                if let Some(fin) = finally_ {
                    lines.push(format!("{}defer func() {{", indent));
                    ctx.push_indent();
                    for s in fin {
                        lines.push(self.emit_statement(s, ctx));
                    }
                    ctx.pop_indent();
                    lines.push(format!("{}}}()", indent));
                }
                for s in try_ {
                    lines.push(self.emit_statement(s, ctx));
                }
                lines.push(format!(
                    "{}// error handling: {}",
                    indent, catch_.binding
                ));
                lines.join("\n")
            }
            StatementIR::Throw { value } => {
                format!(
                    "{}panic({})",
                    indent,
                    self.emit_expression(value, ctx)
                )
            }
            StatementIR::Expression { expr } => {
                format!("{}{}", indent, self.emit_expression(expr, ctx))
            }
        }
    }

    fn emit_expression(&self, expr: &ExprIR, ctx: &mut EmitContext) -> String {
        match expr {
            ExprIR::Literal { value } => emit_go_literal(value),
            ExprIR::Identifier { name } => name.clone(),
            ExprIR::Binary { op, left, right } => {
                let go_op = match op.as_str() {
                    "&&" | "and" => "&&",
                    "||" | "or" => "||",
                    other => other,
                };
                format!(
                    "{} {} {}",
                    self.emit_expression(left, ctx),
                    go_op,
                    self.emit_expression(right, ctx)
                )
            }
            ExprIR::Unary { op, operand } => {
                let go_op = match op.as_str() {
                    "not" => "!",
                    other => other,
                };
                format!("{}{}", go_op, self.emit_expression(operand, ctx))
            }
            ExprIR::Call { callee, args } => {
                let args_str: Vec<String> =
                    args.iter().map(|a| self.emit_expression(a, ctx)).collect();
                format!(
                    "{}({})",
                    self.emit_expression(callee, ctx),
                    args_str.join(", ")
                )
            }
            ExprIR::Member { object, property } => {
                format!(
                    "{}.{}",
                    self.emit_expression(object, ctx),
                    to_pascal_case(property)
                )
            }
            ExprIR::Index { object, index } => {
                format!(
                    "{}[{}]",
                    self.emit_expression(object, ctx),
                    self.emit_expression(index, ctx)
                )
            }
            ExprIR::Object { properties } => {
                if properties.is_empty() {
                    return "map[string]interface{}{}".to_string();
                }
                let fields: Vec<String> = properties
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.emit_expression(v, ctx)))
                    .collect();
                format!("map[string]interface{{}}{{{}}}", fields.join(", "))
            }
            ExprIR::Array { elements } => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| self.emit_expression(e, ctx))
                    .collect();
                format!("[]interface{{}}{{{}}}", elems.join(", "))
            }
            ExprIR::ArrowFn { params, body } => {
                let params_str = params
                    .iter()
                    .map(|p| format!("{} interface{{}}", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut lines = vec![format!("func({}) {{", params_str)];
                ctx.push_indent();
                for s in body {
                    lines.push(self.emit_statement(s, ctx));
                }
                ctx.pop_indent();
                lines.push("}".to_string());
                lines.join("\n")
            }
            ExprIR::Await { expr } => {
                // Go uses goroutines, no await keyword
                self.emit_expression(expr, ctx)
            }
            ExprIR::Pipe { stages } => {
                // Flatten pipe to nested calls
                if stages.is_empty() {
                    return "nil".to_string();
                }
                let mut result = self.emit_expression(&stages[0], ctx);
                for stage in &stages[1..] {
                    result = format!("{}({})", self.emit_expression(stage, ctx), result);
                }
                result
            }
            ExprIR::Template { parts } => {
                let mut format_str = String::new();
                let mut args = Vec::new();
                for part in parts {
                    match part {
                        TemplatePartIR::Text { value } => format_str.push_str(value),
                        TemplatePartIR::Expr { value } => {
                            format_str.push_str("%v");
                            args.push(self.emit_expression(value, ctx));
                        }
                    }
                }
                if args.is_empty() {
                    format!("\"{}\"", format_str)
                } else {
                    ctx.add_import("fmt", "fmt");
                    format!("fmt.Sprintf(\"{}\", {})", format_str, args.join(", "))
                }
            }

            // Domain (Tier 1)
            ExprIR::DbQuery(q) => {
                // GORM-style query
                let mut chain = format!("db.Model(&{}{{}})", q.model);
                if let Some(w) = &q.r#where {
                    chain.push_str(&format!(".Where({})", emit_go_literal(w)));
                }
                if let Some(ob) = &q.order_by {
                    chain.push_str(&format!(".Order({})", emit_go_literal(ob)));
                }
                if let Some(skip) = &q.skip {
                    chain.push_str(&format!(".Offset({})", self.emit_expression(skip, ctx)));
                }
                if let Some(take) = &q.take {
                    chain.push_str(&format!(".Limit({})", self.emit_expression(take, ctx)));
                }
                match q.operation.as_str() {
                    "findUnique" | "findFirst" => chain.push_str(".First(&result)"),
                    "findMany" => chain.push_str(".Find(&results)"),
                    "count" => chain.push_str(".Count(&count)"),
                    _ => chain.push_str(&format!(".{}()", to_pascal_case(&q.operation))),
                }
                chain
            }
            ExprIR::DbMutate(m) => {
                match m.operation.as_str() {
                    "create" => {
                        if let Some(data) = &m.data {
                            format!(
                                "db.Create({})",
                                self.emit_expression(data, ctx)
                            )
                        } else {
                            format!("db.Create(&{}{{}})", m.model)
                        }
                    }
                    "update" => {
                        let mut chain = format!("db.Model(&{}{{}})", m.model);
                        if let Some(w) = &m.r#where {
                            chain.push_str(&format!(".Where({})", emit_go_literal(w)));
                        }
                        if let Some(data) = &m.data {
                            chain.push_str(&format!(
                                ".Updates({})",
                                self.emit_expression(data, ctx)
                            ));
                        }
                        chain
                    }
                    "delete" => {
                        let mut chain = format!("db.Delete(&{}{{}})", m.model);
                        if let Some(w) = &m.r#where {
                            chain.push_str(&format!(".Where({})", emit_go_literal(w)));
                        }
                        chain
                    }
                    _ => format!("db.{}", to_pascal_case(&m.operation)),
                }
            }
            ExprIR::HttpRespond(r) => {
                if let Some(body) = &r.body {
                    format!(
                        "c.JSON({}, {})",
                        r.status,
                        self.emit_expression(body, ctx)
                    )
                } else {
                    format!("c.Status({})", r.status)
                }
            }
            ExprIR::CtxGet { path } => {
                let parts: Vec<&str> = path.splitn(2, '.').collect();
                match parts[0] {
                    "params" if parts.len() > 1 => {
                        format!("c.Param(\"{}\")", parts[1])
                    }
                    "query" if parts.len() > 1 => {
                        format!("c.Query(\"{}\")", parts[1])
                    }
                    "body" => "c.ShouldBindJSON(&body)".to_string(),
                    "headers" if parts.len() > 1 => {
                        format!("c.GetHeader(\"{}\")", parts[1])
                    }
                    _ => format!("c.Get(\"{}\")", path),
                }
            }
            ExprIR::Validate { schema, data } => {
                let data_str = self.emit_expression(data, ctx);
                format!("validate{}({})", schema, data_str)
            }
            ExprIR::HashPassword { input, algorithm, rounds } => {
                ctx.add_import("golang.org/x/crypto/bcrypt", "golang.org/x/crypto/bcrypt");
                let input_str = self.emit_expression(input, ctx);
                let cost = rounds.unwrap_or(10);
                let _ = algorithm;
                format!("bcrypt.GenerateFromPassword([]byte({}), {})", input_str, cost)
            }
            ExprIR::VerifyPassword { password, hash, .. } => {
                ctx.add_import("golang.org/x/crypto/bcrypt", "golang.org/x/crypto/bcrypt");
                format!(
                    "bcrypt.CompareHashAndPassword([]byte({}), []byte({}))",
                    self.emit_expression(hash, ctx),
                    self.emit_expression(password, ctx)
                )
            }
            ExprIR::SignToken { payload, options } => {
                ctx.add_import("github.com/golang-jwt/jwt/v5", "github.com/golang-jwt/jwt/v5");
                let payload_str = self.emit_expression(payload, ctx);
                let _ = options;
                format!(
                    "jwt.NewWithClaims(jwt.SigningMethodHS256, {}).SignedString(jwtSecret)",
                    payload_str
                )
            }
            ExprIR::VerifyToken { token } => {
                ctx.add_import("github.com/golang-jwt/jwt/v5", "github.com/golang-jwt/jwt/v5");
                format!(
                    "jwt.Parse({}, func(token *jwt.Token) (interface{{}}, error) {{ return jwtSecret, nil }})",
                    self.emit_expression(token, ctx)
                )
            }
            ExprIR::NativeBridge(nb) => {
                ctx.add_import(&nb.import.name, &nb.import.from);
                let args_str: Vec<String> = nb
                    .call
                    .args
                    .iter()
                    .map(|a| self.emit_expression(a, ctx))
                    .collect();
                format!("{}({})", nb.call.method, args_str.join(", "))
            }
        }
    }

    fn emit_type(&self, type_ir: &TypeIR) -> String {
        match type_ir {
            TypeIR::String => "string".to_string(),
            TypeIR::Number => "float64".to_string(),
            TypeIR::Boolean => "bool".to_string(),
            TypeIR::Null => "nil".to_string(),
            TypeIR::Void => "".to_string(),
            TypeIR::Any => "interface{}".to_string(),
            TypeIR::Array { inner } => format!("[]{}", self.emit_type(inner)),
            TypeIR::Optional { inner } => format!("*{}", self.emit_type(inner)),
            TypeIR::Ref { name } => name.clone(),
            TypeIR::Object { .. } => "map[string]interface{}".to_string(),
            TypeIR::Union { .. } => "interface{}".to_string(),
        }
    }

    fn emit_schema(&self, schema: &SchemaIR, ctx: &mut EmitContext) -> String {
        let mut lines = Vec::new();
        for (def_name, def_value) in &schema.definitions {
            lines.push(emit_go_struct(def_name, def_value, ctx));
        }
        lines.join("\n\n")
    }

    fn emit_model(&self, model: &ModelIR, ctx: &mut EmitContext) -> String {
        let _ = ctx;
        let mut lines = vec![format!("type {} struct {{", model.name)];
        for (col_name, col_def) in &model.columns {
            let go_type = go_col_type(col_def);
            let json_tag = to_snake_case(col_name);
            let gorm_tag = go_gorm_tag(col_name, col_def);
            lines.push(format!(
                "\t{} {} `json:\"{}\" gorm:\"{}\"`",
                to_pascal_case(col_name),
                go_type,
                json_tag,
                gorm_tag
            ));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn emit_imports(&self, ctx: &mut EmitContext) -> String {
        let imports = ctx.take_imports();
        if imports.is_empty() {
            return String::new();
        }
        let mut lines = vec!["import (".to_string()];
        for imp in &imports {
            lines.push(format!("\t\"{}\"", imp.from));
        }
        lines.push(")".to_string());
        lines.join("\n")
    }

    fn file_extension(&self) -> &str {
        "go"
    }

    fn indent_style(&self) -> IndentStyle {
        IndentStyle::Tabs
    }
}

fn emit_go_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "nil".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(emit_go_literal).collect();
            format!("[]interface{{}}{{{}}}", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let fields: Vec<String> = obj
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, emit_go_literal(v)))
                .collect();
            format!("map[string]interface{{}}{{{}}}", fields.join(", "))
        }
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                Some(first) => {
                    first.to_uppercase().to_string() + &c.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }
    result
}

fn go_col_type(def: &serde_json::Value) -> &str {
    let col_type = def
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("string");
    match col_type {
        "string" | "varchar" | "text" | "uuid" => "string",
        "integer" | "int" => "int",
        "bigint" | "serial" => "int64",
        "float" | "double" | "decimal" | "numeric" => "float64",
        "boolean" | "bool" => "bool",
        "datetime" | "timestamp" | "date" => "time.Time",
        "json" | "jsonb" => "json.RawMessage",
        _ => "interface{}",
    }
}

fn go_gorm_tag(name: &str, def: &serde_json::Value) -> String {
    let mut parts = vec![format!("column:{}", to_snake_case(name))];
    if def.get("primaryKey").and_then(|p| p.as_bool()).unwrap_or(false) {
        parts.push("primaryKey".to_string());
    }
    if def.get("unique").and_then(|u| u.as_bool()).unwrap_or(false) {
        parts.push("unique".to_string());
    }
    if !def.get("nullable").and_then(|n| n.as_bool()).unwrap_or(false) {
        parts.push("not null".to_string());
    }
    parts.join(";")
}

fn emit_go_struct(name: &str, schema: &serde_json::Value, _ctx: &mut EmitContext) -> String {
    let mut lines = vec![format!("type {} struct {{", name)];
    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        for (field_name, field_def) in props {
            let go_type = json_schema_to_go_type(field_def);
            let json_tag = to_snake_case(field_name);
            lines.push(format!(
                "\t{} {} `json:\"{}\"`",
                to_pascal_case(field_name),
                go_type,
                json_tag
            ));
        }
    }
    lines.push("}".to_string());
    lines.join("\n")
}

fn json_schema_to_go_type(schema: &serde_json::Value) -> &str {
    let type_str = schema
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("any");
    match type_str {
        "string" => "string",
        "number" | "integer" => "int",
        "boolean" => "bool",
        "array" => "[]interface{}",
        "object" => "map[string]interface{}",
        _ => "interface{}",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::IndentStyle;
    use rash_ir::expr::DbQueryIR;

    #[test]
    fn test_emit_let_statement() {
        let emitter = GoEmitter;
        let mut ctx = EmitContext::new(IndentStyle::Tabs);

        let stmt = StatementIR::Let {
            name: "user".to_string(),
            type_: None,
            value: ExprIR::literal(serde_json::json!(42)),
        };

        let code = emitter.emit_statement(&stmt, &mut ctx);
        assert_eq!(code, "user := 42");
    }

    #[test]
    fn test_emit_db_query() {
        let emitter = GoEmitter;
        let mut ctx = EmitContext::new(IndentStyle::Tabs);

        let expr = ExprIR::DbQuery(DbQueryIR {
            model: "User".to_string(),
            operation: "findMany".to_string(),
            r#where: None,
            order_by: None,
            skip: None,
            take: None,
            select: None,
            include: None,
        });

        let code = emitter.emit_expression(&expr, &mut ctx);
        assert!(code.contains("db.Model(&User{})"));
        assert!(code.contains(".Find(&results)"));
    }
}
