use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::common::{Language, Tier, TypeRef};

/// AST Node — all handler logic is represented as a tree of these nodes.
/// Uses internal tagging: `#[serde(tag = "type")]`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AstNode {
    // ── Statements (Tier 0) ──

    /// Variable declaration: `let name = value`
    LetStatement {
        tier: Tier,
        name: String,
        #[serde(rename = "valueType", skip_serializing_if = "Option::is_none")]
        value_type: Option<TypeRef>,
        value: Box<AstNode>,
    },

    /// Variable assignment: `target = value`
    AssignStatement {
        tier: Tier,
        target: Box<AstNode>,
        value: Box<AstNode>,
    },

    /// Return statement
    ReturnStatement {
        tier: Tier,
        value: Box<AstNode>,
    },

    /// If/else conditional
    IfStatement {
        tier: Tier,
        condition: Box<AstNode>,
        then: Vec<AstNode>,
        #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
        else_branch: Option<Vec<AstNode>>,
    },

    /// For-in loop
    ForStatement {
        tier: Tier,
        binding: String,
        iterable: Box<AstNode>,
        body: Vec<AstNode>,
    },

    /// While loop
    WhileStatement {
        tier: Tier,
        condition: Box<AstNode>,
        body: Vec<AstNode>,
    },

    /// Pattern matching
    MatchStatement {
        tier: Tier,
        expr: Box<AstNode>,
        arms: Vec<MatchArm>,
    },

    /// Try/catch/finally
    TryCatchStatement {
        tier: Tier,
        #[serde(rename = "try")]
        try_block: Vec<AstNode>,
        #[serde(rename = "catch")]
        catch_block: CatchClause,
        #[serde(rename = "finally", skip_serializing_if = "Option::is_none")]
        finally_block: Option<Vec<AstNode>>,
    },

    /// Throw an error
    ThrowStatement {
        tier: Tier,
        value: Box<AstNode>,
    },

    /// Expression used as statement
    ExpressionStatement {
        tier: Tier,
        expr: Box<AstNode>,
    },

    // ── Expressions (Tier 0) ──

    /// Literal value (string, number, boolean, null)
    Literal {
        tier: Tier,
        value: serde_json::Value,
    },

    /// Variable reference
    Identifier {
        tier: Tier,
        name: String,
    },

    /// Binary operation: `left op right`
    BinaryExpr {
        tier: Tier,
        operator: String,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },

    /// Unary operation: `op operand`
    UnaryExpr {
        tier: Tier,
        operator: String,
        operand: Box<AstNode>,
    },

    /// Function call: `callee(args...)`
    CallExpr {
        tier: Tier,
        callee: Box<AstNode>,
        args: Vec<AstNode>,
    },

    /// Member access: `object.property`
    MemberExpr {
        tier: Tier,
        object: Box<AstNode>,
        property: String,
    },

    /// Index access: `object[index]`
    IndexExpr {
        tier: Tier,
        object: Box<AstNode>,
        index: Box<AstNode>,
    },

    /// Object literal: `{ key: value, ... }`
    ObjectExpr {
        tier: Tier,
        properties: IndexMap<String, AstNode>,
    },

    /// Array literal: `[elem, ...]`
    ArrayExpr {
        tier: Tier,
        elements: Vec<AstNode>,
    },

    /// Arrow/anonymous function
    ArrowFn {
        tier: Tier,
        params: Vec<String>,
        body: Vec<AstNode>,
    },

    /// Await expression
    AwaitExpr {
        tier: Tier,
        expr: Box<AstNode>,
    },

    /// Pipe expression: `a |> b |> c`
    PipeExpr {
        tier: Tier,
        stages: Vec<AstNode>,
    },

    /// Template string: `Hello ${name}`
    TemplateString {
        tier: Tier,
        parts: Vec<TemplatePart>,
    },

    // ── Domain Nodes (Tier 1) ──

    /// Database query (findOne, findMany, count, etc.)
    DbQuery {
        tier: Tier,
        model: String,
        operation: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        r#where: Option<serde_json::Value>,
        #[serde(rename = "orderBy", skip_serializing_if = "Option::is_none")]
        order_by: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        skip: Option<Box<AstNode>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        take: Option<Box<AstNode>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        select: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        include: Option<serde_json::Value>,
    },

    /// Database mutation (insert, update, delete)
    DbMutate {
        tier: Tier,
        model: String,
        operation: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Box<AstNode>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        r#where: Option<serde_json::Value>,
    },

    /// HTTP response
    HttpRespond {
        tier: Tier,
        status: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<IndexMap<String, AstNode>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Box<AstNode>>,
    },

    /// Request context access (params, query, body, headers)
    CtxGet {
        tier: Tier,
        path: String,
    },

    /// Schema validation
    Validate {
        tier: Tier,
        schema: String,
        data: Box<AstNode>,
    },

    /// Password hashing
    HashPassword {
        tier: Tier,
        input: Box<AstNode>,
        algorithm: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        rounds: Option<u32>,
    },

    /// Password verification
    VerifyPassword {
        tier: Tier,
        password: Box<AstNode>,
        hash: Box<AstNode>,
        algorithm: String,
    },

    /// JWT signing
    SignToken {
        tier: Tier,
        payload: Box<AstNode>,
        #[serde(skip_serializing_if = "Option::is_none")]
        secret: Option<Box<AstNode>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        options: Option<serde_json::Value>,
    },

    /// JWT verification
    VerifyToken {
        tier: Tier,
        token: Box<AstNode>,
        #[serde(skip_serializing_if = "Option::is_none")]
        secret: Option<Box<AstNode>>,
    },

    /// Send email
    SendEmail {
        tier: Tier,
        to: Box<AstNode>,
        subject: Box<AstNode>,
        body: Box<AstNode>,
    },

    /// Emit event
    EmitEvent {
        tier: Tier,
        event: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Box<AstNode>>,
    },

    /// Log message
    LogMessage {
        tier: Tier,
        level: String,
        message: Box<AstNode>,
    },

    // ── Bridge Node (Tier 3) ──

    /// Native language bridge — locks to a specific language
    NativeBridge {
        tier: Tier,
        language: Language,
        package: String,
        import: NativeBridgeImport,
        call: NativeBridgeCall,
        #[serde(rename = "returnType", skip_serializing_if = "Option::is_none")]
        return_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fallback: Option<NativeBridgeFallback>,
    },
}

/// Match arm for MatchStatement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: AstNode,
    pub body: Vec<AstNode>,
}

/// Catch clause for TryCatchStatement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatchClause {
    pub binding: String,
    pub body: Vec<AstNode>,
}

/// Template string part
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TemplatePart {
    #[serde(rename = "text")]
    Text { value: String },
    #[serde(rename = "expr")]
    Expr { value: Box<AstNode> },
}

/// NativeBridge import declaration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeBridgeImport {
    pub name: String,
    pub from: String,
}

/// NativeBridge function call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeBridgeCall {
    pub method: String,
    pub args: Vec<AstNode>,
}

/// NativeBridge fallback node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeBridgeFallback {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub node: Box<AstNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_let_statement_roundtrip() {
        let json = serde_json::json!({
            "type": "LetStatement",
            "tier": 0,
            "name": "userId",
            "value": {
                "type": "CtxGet",
                "tier": 1,
                "path": "params.id"
            }
        });

        let node: AstNode = serde_json::from_value(json.clone()).unwrap();
        match &node {
            AstNode::LetStatement { name, tier, .. } => {
                assert_eq!(name, "userId");
                assert_eq!(*tier, Tier::Universal);
            }
            _ => panic!("Expected LetStatement"),
        }

        // Roundtrip
        let serialized = serde_json::to_value(&node).unwrap();
        let node2: AstNode = serde_json::from_value(serialized).unwrap();
        assert_eq!(node, node2);
    }

    #[test]
    fn test_if_statement_with_nested_nodes() {
        let json = serde_json::json!({
            "type": "IfStatement",
            "tier": 0,
            "condition": {
                "type": "BinaryExpr",
                "tier": 0,
                "operator": "==",
                "left": { "type": "Identifier", "tier": 0, "name": "user" },
                "right": { "type": "Literal", "tier": 0, "value": null }
            },
            "then": [
                {
                    "type": "ReturnStatement",
                    "tier": 0,
                    "value": {
                        "type": "HttpRespond",
                        "tier": 1,
                        "status": 404,
                        "body": {
                            "type": "ObjectExpr",
                            "tier": 0,
                            "properties": {
                                "message": { "type": "Literal", "tier": 0, "value": "User not found" },
                                "code": { "type": "Literal", "tier": 0, "value": "NOT_FOUND" }
                            }
                        }
                    }
                }
            ],
            "else": null
        });

        let node: AstNode = serde_json::from_value(json).unwrap();
        match &node {
            AstNode::IfStatement { then, .. } => {
                assert_eq!(then.len(), 1);
            }
            _ => panic!("Expected IfStatement"),
        }
    }

    #[test]
    fn test_db_query_deserialization() {
        let json = serde_json::json!({
            "type": "DbQuery",
            "tier": 1,
            "model": "User",
            "operation": "findUnique",
            "where": {
                "id": {
                    "type": "Identifier",
                    "tier": 0,
                    "name": "userId"
                }
            }
        });

        let node: AstNode = serde_json::from_value(json).unwrap();
        match &node {
            AstNode::DbQuery {
                model, operation, ..
            } => {
                assert_eq!(model, "User");
                assert_eq!(operation, "findUnique");
            }
            _ => panic!("Expected DbQuery"),
        }
    }

    #[test]
    fn test_native_bridge_deserialization() {
        let json = serde_json::json!({
            "type": "NativeBridge",
            "tier": 3,
            "language": "typescript",
            "package": "npm:bcrypt",
            "import": { "name": "bcrypt", "from": "bcrypt" },
            "call": {
                "method": "bcrypt.hash",
                "args": [
                    { "type": "Identifier", "tier": 0, "name": "password" },
                    { "type": "Literal", "tier": 0, "value": 10 }
                ]
            },
            "returnType": "string",
            "fallback": {
                "description": "다른 언어로 변환 시 대체 노드",
                "node": {
                    "type": "HashPassword",
                    "tier": 1,
                    "input": { "type": "Identifier", "tier": 0, "name": "password" },
                    "algorithm": "bcrypt",
                    "rounds": 10
                }
            }
        });

        let node: AstNode = serde_json::from_value(json).unwrap();
        match &node {
            AstNode::NativeBridge {
                tier,
                language,
                package,
                fallback,
                ..
            } => {
                assert_eq!(*tier, Tier::Bridge);
                assert_eq!(*language, Language::Typescript);
                assert_eq!(package, "npm:bcrypt");
                assert!(fallback.is_some());
            }
            _ => panic!("Expected NativeBridge"),
        }
    }

    #[test]
    fn test_full_handler_body_from_docs() {
        let json = serde_json::json!([
            {
                "type": "LetStatement",
                "tier": 0,
                "name": "userId",
                "value": {
                    "type": "CtxGet",
                    "tier": 1,
                    "path": "params.id"
                }
            },
            {
                "type": "LetStatement",
                "tier": 0,
                "name": "user",
                "value": {
                    "type": "AwaitExpr",
                    "tier": 1,
                    "expr": {
                        "type": "DbQuery",
                        "tier": 1,
                        "model": "User",
                        "operation": "findUnique",
                        "where": {
                            "id": { "type": "Identifier", "tier": 0, "name": "userId" }
                        }
                    }
                }
            },
            {
                "type": "IfStatement",
                "tier": 0,
                "condition": {
                    "type": "BinaryExpr",
                    "tier": 0,
                    "operator": "==",
                    "left": { "type": "Identifier", "tier": 0, "name": "user" },
                    "right": { "type": "Literal", "tier": 0, "value": null }
                },
                "then": [
                    {
                        "type": "ReturnStatement",
                        "tier": 0,
                        "value": {
                            "type": "HttpRespond",
                            "tier": 1,
                            "status": 404,
                            "body": {
                                "type": "ObjectExpr",
                                "tier": 0,
                                "properties": {
                                    "message": { "type": "Literal", "tier": 0, "value": "User not found" },
                                    "code": { "type": "Literal", "tier": 0, "value": "NOT_FOUND" }
                                }
                            }
                        }
                    }
                ],
                "else": null
            },
            {
                "type": "ReturnStatement",
                "tier": 0,
                "value": {
                    "type": "HttpRespond",
                    "tier": 1,
                    "status": 200,
                    "body": { "type": "Identifier", "tier": 0, "name": "user" }
                }
            }
        ]);

        let body: Vec<AstNode> = serde_json::from_value(json).unwrap();
        assert_eq!(body.len(), 4);

        // Verify roundtrip
        let serialized = serde_json::to_value(&body).unwrap();
        let body2: Vec<AstNode> = serde_json::from_value(serialized).unwrap();
        assert_eq!(body, body2);
    }
}
