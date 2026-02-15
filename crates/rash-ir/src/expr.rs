use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use rash_spec::types::common::Language;

use crate::statement::StatementIR;

/// IR-level expression — a value-producing node in the AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ExprIR {
    /// Literal value (string, number, boolean, null)
    Literal { value: serde_json::Value },

    /// Variable reference
    Identifier { name: String },

    /// Binary operation: `left op right`
    Binary {
        op: String,
        left: Box<ExprIR>,
        right: Box<ExprIR>,
    },

    /// Unary operation: `op operand`
    Unary {
        op: String,
        operand: Box<ExprIR>,
    },

    /// Function call: `callee(args...)`
    Call {
        callee: Box<ExprIR>,
        args: Vec<ExprIR>,
    },

    /// Member access: `object.property`
    Member {
        object: Box<ExprIR>,
        property: String,
    },

    /// Index access: `object[index]`
    Index {
        object: Box<ExprIR>,
        index: Box<ExprIR>,
    },

    /// Object literal: `{ key: value, ... }`
    Object {
        properties: Vec<(String, ExprIR)>,
    },

    /// Array literal: `[elem, ...]`
    Array { elements: Vec<ExprIR> },

    /// Arrow / anonymous function
    ArrowFn {
        params: Vec<String>,
        body: Vec<StatementIR>,
    },

    /// Await expression
    Await { expr: Box<ExprIR> },

    /// Pipe expression: `a |> b |> c`
    Pipe { stages: Vec<ExprIR> },

    /// Template string: `` `Hello ${name}` ``
    Template { parts: Vec<TemplatePartIR> },

    // ── Domain (Tier 1) ──
    /// Database query
    DbQuery(DbQueryIR),

    /// Database mutation
    DbMutate(DbMutateIR),

    /// HTTP response construction
    HttpRespond(HttpRespondIR),

    /// Request context access (e.g., "params.id", "body", "query.page")
    CtxGet { path: String },

    /// Schema validation
    Validate {
        schema: String,
        data: Box<ExprIR>,
    },

    /// Password hashing
    HashPassword {
        input: Box<ExprIR>,
        algorithm: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        rounds: Option<u32>,
    },

    /// Password verification
    VerifyPassword {
        password: Box<ExprIR>,
        hash: Box<ExprIR>,
        algorithm: String,
    },

    /// JWT signing
    SignToken {
        payload: Box<ExprIR>,
        #[serde(skip_serializing_if = "Option::is_none")]
        options: Option<serde_json::Value>,
    },

    /// JWT verification
    VerifyToken { token: Box<ExprIR> },

    // ── Bridge (Tier 3) ──
    /// Native language bridge — locks to a specific language/package
    NativeBridge(NativeBridgeIR),
}

// Convenience constructors for common ExprIR patterns
impl ExprIR {
    /// Create a Literal expression from a serde_json::Value.
    pub fn literal(value: serde_json::Value) -> Self {
        ExprIR::Literal { value }
    }

    /// Create an Identifier expression.
    pub fn ident(name: impl Into<String>) -> Self {
        ExprIR::Identifier { name: name.into() }
    }
}

/// Database query expression (Tier 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbQueryIR {
    /// Target model name (e.g., "User")
    pub model: String,
    /// Query operation (e.g., "findUnique", "findMany", "count")
    pub operation: String,
    /// WHERE clause (opaque JSON for now)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<serde_json::Value>,
    /// ORDER BY clause
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<serde_json::Value>,
    /// Skip count expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip: Option<Box<ExprIR>>,
    /// Take count expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take: Option<Box<ExprIR>>,
    /// Fields to select
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<Vec<String>>,
    /// Relations to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<serde_json::Value>,
}

/// Database mutation expression (Tier 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMutateIR {
    /// Target model name
    pub model: String,
    /// Mutation operation (e.g., "create", "update", "delete")
    pub operation: String,
    /// Data payload expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Box<ExprIR>>,
    /// WHERE clause (opaque JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<serde_json::Value>,
}

/// HTTP response construction expression (Tier 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRespondIR {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, ExprIR>>,
    /// Response body expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Box<ExprIR>>,
}

/// Native bridge expression (Tier 3) — calls into language-specific packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeBridgeIR {
    /// Locked language
    pub language: Language,
    /// Package identifier (e.g., "npm:bcrypt", "crate:argon2")
    pub package: String,
    /// Import declaration
    pub import: NativeBridgeImportIR,
    /// Function call
    pub call: NativeBridgeCallIR,
    /// Expected return type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
}

/// Import declaration for a native bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeBridgeImportIR {
    /// Import name
    pub name: String,
    /// Package/module path
    pub from: String,
}

/// Function call within a native bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeBridgeCallIR {
    /// Method name (e.g., "bcrypt.hash")
    pub method: String,
    /// Call arguments
    pub args: Vec<ExprIR>,
}

/// A part of a template string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "part_kind")]
pub enum TemplatePartIR {
    /// Static text segment
    Text { value: String },
    /// Interpolated expression segment
    Expr { value: Box<ExprIR> },
}

/// IR-level type representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type_kind")]
pub enum TypeIR {
    String,
    Number,
    Boolean,
    Null,
    Void,
    Any,
    /// Array of inner type
    Array { inner: Box<TypeIR> },
    /// Optional (nullable) inner type
    Optional { inner: Box<TypeIR> },
    /// Named type reference (e.g., schema name, model name)
    Ref { name: String },
    /// Object type with named fields
    Object { fields: Vec<(String, TypeIR)> },
    /// Union of multiple types
    Union { variants: Vec<TypeIR> },
}

// Convenience constructors for TypeIR
impl TypeIR {
    /// Create an Array type wrapping the given inner type.
    pub fn array(inner: TypeIR) -> Self {
        TypeIR::Array {
            inner: Box::new(inner),
        }
    }

    /// Create an Optional type wrapping the given inner type.
    pub fn optional(inner: TypeIR) -> Self {
        TypeIR::Optional {
            inner: Box::new(inner),
        }
    }
}

// Allow constructing TypeIR::Ref from a string conveniently
impl TypeIR {
    /// Create a Ref type.
    #[allow(non_snake_case)]
    pub fn Ref(name: impl Into<String>) -> Self {
        TypeIR::Ref { name: name.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_expr_roundtrip() {
        let expr = ExprIR::Literal {
            value: serde_json::json!(42),
        };

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "Literal");
        assert_eq!(json["value"], 42);

        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::Literal { value } => assert_eq!(value, 42),
            _ => panic!("Expected Literal"),
        }
    }

    #[test]
    fn test_binary_expr() {
        let expr = ExprIR::Binary {
            op: "+".to_string(),
            left: Box::new(ExprIR::literal(serde_json::json!(1))),
            right: Box::new(ExprIR::literal(serde_json::json!(2))),
        };

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "Binary");
        assert_eq!(json["op"], "+");

        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::Binary { op, .. } => assert_eq!(op, "+"),
            _ => panic!("Expected Binary"),
        }
    }

    #[test]
    fn test_db_query_ir() {
        let expr = ExprIR::DbQuery(DbQueryIR {
            model: "User".to_string(),
            operation: "findUnique".to_string(),
            r#where: Some(serde_json::json!({ "id": "123" })),
            order_by: None,
            skip: None,
            take: None,
            select: Some(vec!["id".to_string(), "email".to_string()]),
            include: None,
        });

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "DbQuery");
        assert_eq!(json["model"], "User");

        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::DbQuery(q) => {
                assert_eq!(q.model, "User");
                assert_eq!(q.operation, "findUnique");
                assert_eq!(q.select.unwrap().len(), 2);
            }
            _ => panic!("Expected DbQuery"),
        }
    }

    #[test]
    fn test_http_respond_ir() {
        let expr = ExprIR::HttpRespond(HttpRespondIR {
            status: 200,
            headers: None,
            body: Some(Box::new(ExprIR::ident("user"))),
        });

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "HttpRespond");
        assert_eq!(json["status"], 200);

        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::HttpRespond(r) => assert_eq!(r.status, 200),
            _ => panic!("Expected HttpRespond"),
        }
    }

    #[test]
    fn test_native_bridge_ir() {
        let expr = ExprIR::NativeBridge(NativeBridgeIR {
            language: Language::Typescript,
            package: "npm:bcrypt".to_string(),
            import: NativeBridgeImportIR {
                name: "bcrypt".to_string(),
                from: "bcrypt".to_string(),
            },
            call: NativeBridgeCallIR {
                method: "bcrypt.hash".to_string(),
                args: vec![
                    ExprIR::ident("password"),
                    ExprIR::literal(serde_json::json!(10)),
                ],
            },
            return_type: Some("string".to_string()),
        });

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "NativeBridge");
        assert_eq!(json["language"], "typescript");

        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::NativeBridge(nb) => {
                assert_eq!(nb.language, Language::Typescript);
                assert_eq!(nb.package, "npm:bcrypt");
                assert_eq!(nb.call.args.len(), 2);
            }
            _ => panic!("Expected NativeBridge"),
        }
    }

    #[test]
    fn test_template_string() {
        let expr = ExprIR::Template {
            parts: vec![
                TemplatePartIR::Text {
                    value: "Hello, ".to_string(),
                },
                TemplatePartIR::Expr {
                    value: Box::new(ExprIR::ident("name")),
                },
                TemplatePartIR::Text {
                    value: "!".to_string(),
                },
            ],
        };

        let json = serde_json::to_value(&expr).unwrap();
        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::Template { parts } => assert_eq!(parts.len(), 3),
            _ => panic!("Expected Template"),
        }
    }

    #[test]
    fn test_type_ir_roundtrip() {
        let type_ = TypeIR::Array {
            inner: Box::new(TypeIR::Optional {
                inner: Box::new(TypeIR::Ref {
                    name: "User".to_string(),
                }),
            }),
        };

        let json = serde_json::to_value(&type_).unwrap();
        let deserialized: TypeIR = serde_json::from_value(json).unwrap();
        assert_eq!(type_, deserialized);
    }

    #[test]
    fn test_ctx_get() {
        let expr = ExprIR::CtxGet {
            path: "params.id".to_string(),
        };

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "CtxGet");
        assert_eq!(json["path"], "params.id");
    }

    #[test]
    fn test_object_expr() {
        let expr = ExprIR::Object {
            properties: vec![
                ("key".to_string(), ExprIR::literal(serde_json::json!("value"))),
                ("count".to_string(), ExprIR::literal(serde_json::json!(42))),
            ],
        };

        let json = serde_json::to_value(&expr).unwrap();
        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::Object { properties } => assert_eq!(properties.len(), 2),
            _ => panic!("Expected Object"),
        }
    }

    #[test]
    fn test_arrow_fn_with_body() {
        let expr = ExprIR::ArrowFn {
            params: vec!["x".to_string()],
            body: vec![StatementIR::Return {
                value: Some(ExprIR::Binary {
                    op: "*".to_string(),
                    left: Box::new(ExprIR::ident("x")),
                    right: Box::new(ExprIR::literal(serde_json::json!(2))),
                }),
            }],
        };

        let json = serde_json::to_value(&expr).unwrap();
        let deserialized: ExprIR = serde_json::from_value(json).unwrap();
        match deserialized {
            ExprIR::ArrowFn { params, body } => {
                assert_eq!(params, vec!["x"]);
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected ArrowFn"),
        }
    }

    #[test]
    fn test_validate_expr() {
        let expr = ExprIR::Validate {
            schema: "CreateUserBody".to_string(),
            data: Box::new(ExprIR::CtxGet {
                path: "body".to_string(),
            }),
        };

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "Validate");
        assert_eq!(json["schema"], "CreateUserBody");
    }

    #[test]
    fn test_hash_password_expr() {
        let expr = ExprIR::HashPassword {
            input: Box::new(ExprIR::ident("password")),
            algorithm: "bcrypt".to_string(),
            rounds: Some(10),
        };

        let json = serde_json::to_value(&expr).unwrap();
        assert_eq!(json["kind"], "HashPassword");
        assert_eq!(json["algorithm"], "bcrypt");
        assert_eq!(json["rounds"], 10);
    }

    #[test]
    fn test_sign_verify_token() {
        let sign = ExprIR::SignToken {
            payload: Box::new(ExprIR::ident("claims")),
            options: Some(serde_json::json!({ "expiresIn": "1h" })),
        };

        let json = serde_json::to_value(&sign).unwrap();
        assert_eq!(json["kind"], "SignToken");

        let verify = ExprIR::VerifyToken {
            token: Box::new(ExprIR::ident("token")),
        };

        let json = serde_json::to_value(&verify).unwrap();
        assert_eq!(json["kind"], "VerifyToken");
    }
}
