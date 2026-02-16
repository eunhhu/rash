use regex::Regex;

use rash_spec::types::ast::AstNode;
use rash_spec::types::common::Tier;

/// Extract AST nodes from handler function body source code.
///
/// Recognizes Express patterns:
/// - `req.params.xxx` → CtxGet { path: "params.xxx" }
/// - `req.query.xxx`  → CtxGet { path: "query.xxx" }
/// - `req.body`       → CtxGet { path: "body" }
/// - `res.json(data)` → HttpRespond { status: 200, body: data }
/// - `res.status(N).json(data)` → HttpRespond { status: N, body: data }
/// - `prisma.xxx.findUnique(...)` → DbQuery
/// - `prisma.xxx.findMany(...)`   → DbQuery
/// - `prisma.xxx.create(...)`     → DbMutate
/// - `prisma.xxx.update(...)`     → DbMutate
/// - `prisma.xxx.delete(...)`     → DbMutate
pub fn extract_handler_body(body_source: &str, warnings: &mut Vec<String>) -> Vec<AstNode> {
    let mut statements = Vec::new();

    // Split into lines and process each statement
    for line in body_source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if let Some(node) = try_parse_statement(trimmed, warnings) {
            statements.push(node);
        }
    }

    // If we got nothing meaningful, wrap the entire body as a NativeBridge
    if statements.is_empty() && !body_source.trim().is_empty() {
        warnings.push(
            "Could not parse handler body, wrapping as NativeBridge".to_string(),
        );
        statements.push(make_native_bridge_raw(body_source.trim()));
    }

    statements
}

/// Try to parse a single statement line into an AstNode.
fn try_parse_statement(line: &str, warnings: &mut Vec<String>) -> Option<AstNode> {
    // const/let/var xxx = ...
    let let_re = Regex::new(r"^(?:const|let|var)\s+(\w+)\s*=\s*(.+?)\s*;?\s*$").unwrap();
    if let Some(cap) = let_re.captures(line) {
        let var_name = cap[1].to_string();
        let rhs = cap[2].trim();
        let value = parse_expression(rhs, warnings);
        return Some(AstNode::LetStatement {
            tier: Tier::Universal,
            name: var_name,
            value_type: None,
            value: Box::new(value),
        });
    }

    // return res.status(N).json(data) or return res.json(data)
    if line.starts_with("return ") {
        let rest = line.strip_prefix("return ").unwrap().trim().trim_end_matches(';');
        let value = parse_expression(rest, warnings);
        return Some(AstNode::ReturnStatement {
            tier: Tier::Universal,
            value: Box::new(value),
        });
    }

    // res.status(N).json(data)
    if let Some(node) = try_parse_res_response(line) {
        return Some(AstNode::ExpressionStatement {
            tier: Tier::Universal,
            expr: Box::new(node),
        });
    }

    // res.json(data)
    if let Some(node) = try_parse_res_json(line) {
        return Some(AstNode::ExpressionStatement {
            tier: Tier::Universal,
            expr: Box::new(node),
        });
    }

    // await prisma.xxx.yyy(...)
    if line.contains("prisma.") {
        let expr = parse_expression(line.trim_end_matches(';'), warnings);
        return Some(AstNode::ExpressionStatement {
            tier: Tier::Universal,
            expr: Box::new(expr),
        });
    }

    // Unrecognized line — emit warning for non-trivial lines
    if !line.starts_with('{') && !line.starts_with('}') && !line.starts_with("/*") && !line.ends_with("*/") {
        warnings.push(format!("skipped unrecognized statement: {}", truncate_line(line, 80)));
    }
    None
}

/// Parse an expression string into an AstNode.
pub(crate) fn parse_expression(expr: &str, warnings: &mut Vec<String>) -> AstNode {
    let expr = expr.trim().trim_end_matches(';');

    // await expr
    if let Some(inner) = expr.strip_prefix("await ") {
        let inner_node = parse_expression(inner.trim(), warnings);
        return AstNode::AwaitExpr {
            tier: Tier::Domain,
            expr: Box::new(inner_node),
        };
    }

    // res.status(N).json(data)
    if let Some(node) = try_parse_res_response(expr) {
        return node;
    }

    // res.json(data)
    if let Some(node) = try_parse_res_json(expr) {
        return node;
    }

    // prisma.model.operation(...)
    if let Some(node) = try_parse_prisma(expr) {
        return node;
    }

    // req.params.xxx
    let req_params_re = Regex::new(r"^req\.params\.(\w+)$").unwrap();
    if let Some(cap) = req_params_re.captures(expr) {
        return AstNode::CtxGet {
            tier: Tier::Domain,
            path: format!("params.{}", &cap[1]),
        };
    }

    // req.query.xxx
    let req_query_re = Regex::new(r"^req\.query\.(\w+)$").unwrap();
    if let Some(cap) = req_query_re.captures(expr) {
        return AstNode::CtxGet {
            tier: Tier::Domain,
            path: format!("query.{}", &cap[1]),
        };
    }

    // req.body
    if expr == "req.body" {
        return AstNode::CtxGet {
            tier: Tier::Domain,
            path: "body".to_string(),
        };
    }

    // req.params (entire params object)
    if expr == "req.params" {
        return AstNode::CtxGet {
            tier: Tier::Domain,
            path: "params".to_string(),
        };
    }

    // Simple identifier
    let ident_re = Regex::new(r"^\w+$").unwrap();
    if ident_re.is_match(expr) {
        return AstNode::Identifier {
            tier: Tier::Universal,
            name: expr.to_string(),
        };
    }

    // String literal
    if (expr.starts_with('"') && expr.ends_with('"'))
        || (expr.starts_with('\'') && expr.ends_with('\''))
    {
        let inner = &expr[1..expr.len() - 1];
        return AstNode::Literal {
            tier: Tier::Universal,
            value: serde_json::json!(inner),
        };
    }

    // Number literal
    if let Ok(n) = expr.parse::<f64>() {
        return AstNode::Literal {
            tier: Tier::Universal,
            value: serde_json::json!(n),
        };
    }

    // Fallback: wrap as NativeBridge
    warnings.push(format!("Unrecognized expression, wrapping as NativeBridge: {expr}"));
    make_native_bridge_raw(expr)
}

/// Try to parse `res.status(N).json(data)` pattern.
fn try_parse_res_response(expr: &str) -> Option<AstNode> {
    let re = Regex::new(r"res\.status\((\d+)\)\.json\((.+)\)\s*;?\s*$").unwrap();
    let cap = re.captures(expr)?;
    let status: u16 = cap[1].parse().ok()?;
    let body_expr = cap[2].trim();
    let body_node = simple_expr_to_node(body_expr);
    Some(AstNode::HttpRespond {
        tier: Tier::Domain,
        status,
        headers: None,
        body: Some(Box::new(body_node)),
    })
}

/// Try to parse `res.json(data)` pattern.
fn try_parse_res_json(expr: &str) -> Option<AstNode> {
    let re = Regex::new(r"res\.json\((.+)\)\s*;?\s*$").unwrap();
    // Avoid matching res.status(...).json(...) which is handled by try_parse_res_response
    if expr.contains("res.status(") {
        return None;
    }
    let cap = re.captures(expr)?;
    let body_expr = cap[1].trim();
    let body_node = simple_expr_to_node(body_expr);
    Some(AstNode::HttpRespond {
        tier: Tier::Domain,
        status: 200,
        headers: None,
        body: Some(Box::new(body_node)),
    })
}

/// Try to parse `prisma.model.operation(...)` patterns.
fn try_parse_prisma(expr: &str) -> Option<AstNode> {
    let re = Regex::new(r"prisma\.(\w+)\.(\w+)\(").unwrap();
    let cap = re.captures(expr)?;
    let model_raw = &cap[1];
    // Capitalize first letter for model name
    let model = capitalize(model_raw);
    let operation = &cap[2];

    let is_mutation = matches!(
        operation,
        "create" | "update" | "delete" | "upsert" | "createMany" | "updateMany" | "deleteMany"
    );

    if is_mutation {
        Some(AstNode::DbMutate {
            tier: Tier::Domain,
            model,
            operation: operation.to_string(),
            data: None,
            r#where: None,
        })
    } else {
        Some(AstNode::DbQuery {
            tier: Tier::Domain,
            model,
            operation: operation.to_string(),
            r#where: None,
            order_by: None,
            skip: None,
            take: None,
            select: None,
            include: None,
        })
    }
}

/// Convert a simple expression string to an AstNode without deep parsing.
fn simple_expr_to_node(expr: &str) -> AstNode {
    let expr = expr.trim();

    // Identifier
    let ident_re = Regex::new(r"^\w+$").unwrap();
    if ident_re.is_match(expr) {
        return AstNode::Identifier {
            tier: Tier::Universal,
            name: expr.to_string(),
        };
    }

    // String literal
    if (expr.starts_with('"') && expr.ends_with('"'))
        || (expr.starts_with('\'') && expr.ends_with('\''))
    {
        let inner = &expr[1..expr.len() - 1];
        return AstNode::Literal {
            tier: Tier::Universal,
            value: serde_json::json!(inner),
        };
    }

    // Number literal
    if let Ok(n) = expr.parse::<f64>() {
        return AstNode::Literal {
            tier: Tier::Universal,
            value: serde_json::json!(n),
        };
    }

    // Object literal: { ... }
    if expr.starts_with('{') && expr.ends_with('}') {
        return AstNode::ObjectExpr {
            tier: Tier::Universal,
            properties: indexmap::IndexMap::new(),
        };
    }

    // Fallback: treat as identifier
    AstNode::Identifier {
        tier: Tier::Universal,
        name: expr.to_string(),
    }
}

/// Create a NativeBridge node wrapping raw TypeScript code.
fn make_native_bridge_raw(raw_code: &str) -> AstNode {
    use rash_spec::types::ast::{NativeBridgeCall, NativeBridgeImport};
    use rash_spec::types::common::Language;

    AstNode::NativeBridge {
        tier: Tier::Bridge,
        language: Language::Typescript,
        package: "raw".to_string(),
        import: NativeBridgeImport {
            name: "raw".to_string(),
            from: "inline".to_string(),
        },
        call: NativeBridgeCall {
            method: "eval".to_string(),
            args: vec![AstNode::Literal {
                tier: Tier::Universal,
                value: serde_json::json!(raw_code),
            }],
        },
        return_type: None,
        fallback: None,
    }
}

/// Truncate a line for display in warnings.
fn truncate_line(line: &str, max: usize) -> String {
    if line.len() <= max {
        line.to_string()
    } else {
        format!("{}...", &line[..max])
    }
}

/// Capitalize the first character of a string.
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
    fn test_parse_req_params() {
        let mut w = Vec::new();
        let node = parse_expression("req.params.id", &mut w);
        match node {
            AstNode::CtxGet { path, .. } => assert_eq!(path, "params.id"),
            other => panic!("Expected CtxGet, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_req_query() {
        let mut w = Vec::new();
        let node = parse_expression("req.query.page", &mut w);
        match node {
            AstNode::CtxGet { path, .. } => assert_eq!(path, "query.page"),
            other => panic!("Expected CtxGet, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_req_body() {
        let mut w = Vec::new();
        let node = parse_expression("req.body", &mut w);
        match node {
            AstNode::CtxGet { path, .. } => assert_eq!(path, "body"),
            other => panic!("Expected CtxGet, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_res_json() {
        let node = try_parse_res_json("res.json(users)").unwrap();
        match node {
            AstNode::HttpRespond { status, body, .. } => {
                assert_eq!(status, 200);
                match *body.unwrap() {
                    AstNode::Identifier { name, .. } => assert_eq!(name, "users"),
                    other => panic!("Expected Identifier, got {other:?}"),
                }
            }
            other => panic!("Expected HttpRespond, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_res_status_json() {
        let node = try_parse_res_response("res.status(201).json(user)").unwrap();
        match node {
            AstNode::HttpRespond { status, body, .. } => {
                assert_eq!(status, 201);
                match *body.unwrap() {
                    AstNode::Identifier { name, .. } => assert_eq!(name, "user"),
                    other => panic!("Expected Identifier, got {other:?}"),
                }
            }
            other => panic!("Expected HttpRespond, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_prisma_find_many() {
        let node = try_parse_prisma("prisma.user.findMany()").unwrap();
        match node {
            AstNode::DbQuery {
                model, operation, ..
            } => {
                assert_eq!(model, "User");
                assert_eq!(operation, "findMany");
            }
            other => panic!("Expected DbQuery, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_prisma_find_unique() {
        let node = try_parse_prisma("prisma.user.findUnique({ where: { id } })").unwrap();
        match node {
            AstNode::DbQuery {
                model, operation, ..
            } => {
                assert_eq!(model, "User");
                assert_eq!(operation, "findUnique");
            }
            other => panic!("Expected DbQuery, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_prisma_create() {
        let node = try_parse_prisma("prisma.user.create({ data: body })").unwrap();
        match node {
            AstNode::DbMutate {
                model, operation, ..
            } => {
                assert_eq!(model, "User");
                assert_eq!(operation, "create");
            }
            other => panic!("Expected DbMutate, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_await_prisma() {
        let mut w = Vec::new();
        let node = parse_expression("await prisma.user.findMany()", &mut w);
        match node {
            AstNode::AwaitExpr { expr, .. } => match *expr {
                AstNode::DbQuery {
                    model, operation, ..
                } => {
                    assert_eq!(model, "User");
                    assert_eq!(operation, "findMany");
                }
                other => panic!("Expected DbQuery, got {other:?}"),
            },
            other => panic!("Expected AwaitExpr, got {other:?}"),
        }
    }

    #[test]
    fn test_handler_body_extraction() {
        let body = r#"
    const userId = req.params.id;
    const user = await prisma.user.findUnique({ where: { id: userId } });
    res.json(user);
"#;
        let mut w = Vec::new();
        let nodes = extract_handler_body(body, &mut w);
        assert_eq!(nodes.len(), 3);

        // First: let userId = req.params.id
        match &nodes[0] {
            AstNode::LetStatement { name, value, .. } => {
                assert_eq!(name, "userId");
                match value.as_ref() {
                    AstNode::CtxGet { path, .. } => assert_eq!(path, "params.id"),
                    other => panic!("Expected CtxGet, got {other:?}"),
                }
            }
            other => panic!("Expected LetStatement, got {other:?}"),
        }
    }

    #[test]
    fn test_native_bridge_fallback() {
        let mut w = Vec::new();
        let node = parse_expression("someWeirdLib.doThing(x, y, z)", &mut w);
        match node {
            AstNode::NativeBridge { language, .. } => {
                assert_eq!(language, rash_spec::types::common::Language::Typescript);
            }
            other => panic!("Expected NativeBridge, got {other:?}"),
        }
        assert!(!w.is_empty());
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("user"), "User");
        assert_eq!(capitalize("post"), "Post");
        assert_eq!(capitalize(""), "");
    }
}
