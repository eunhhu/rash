use serde::{Deserialize, Serialize};

use crate::expr::{ExprIR, TypeIR};

/// IR-level statement — a single instruction in a handler body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StatementIR {
    /// Variable declaration: `let name: type = value`
    Let {
        name: String,
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        type_: Option<TypeIR>,
        value: ExprIR,
    },

    /// Assignment: `target = value`
    Assign { target: ExprIR, value: ExprIR },

    /// Return statement: `return value`
    Return {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<ExprIR>,
    },

    /// Conditional: `if (condition) { then } else { else }`
    If {
        condition: ExprIR,
        then_: Vec<StatementIR>,
        #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
        else_: Option<Vec<StatementIR>>,
    },

    /// For-in loop: `for (binding in iterable) { body }`
    For {
        binding: String,
        iterable: ExprIR,
        body: Vec<StatementIR>,
    },

    /// While loop: `while (condition) { body }`
    While {
        condition: ExprIR,
        body: Vec<StatementIR>,
    },

    /// Pattern matching
    Match { expr: ExprIR, arms: Vec<MatchArmIR> },

    /// Try/catch/finally
    TryCatch {
        #[serde(rename = "try")]
        try_: Vec<StatementIR>,
        #[serde(rename = "catch")]
        catch_: CatchClauseIR,
        #[serde(rename = "finally", skip_serializing_if = "Option::is_none")]
        finally_: Option<Vec<StatementIR>>,
    },

    /// Throw expression as statement
    Throw { value: ExprIR },

    /// Expression used as statement
    Expression { expr: ExprIR },
}

/// A single arm of a match expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArmIR {
    /// Pattern to match against
    pub pattern: ExprIR,
    /// Body to execute when pattern matches
    pub body: Vec<StatementIR>,
}

/// Catch clause in a try/catch statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchClauseIR {
    /// Error binding name
    pub binding: String,
    /// Catch body
    pub body: Vec<StatementIR>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_let_statement_roundtrip() {
        let stmt = StatementIR::Let {
            name: "userId".to_string(),
            type_: Some(TypeIR::String),
            value: ExprIR::ident("ctx.params.id"),
        };

        let json = serde_json::to_value(&stmt).unwrap();
        assert_eq!(json["kind"], "Let");
        assert_eq!(json["name"], "userId");

        let deserialized: StatementIR = serde_json::from_value(json).unwrap();
        match deserialized {
            StatementIR::Let { name, type_, .. } => {
                assert_eq!(name, "userId");
                assert!(type_.is_some());
            }
            _ => panic!("Expected Let"),
        }
    }

    #[test]
    fn test_if_statement_with_else() {
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

        let json = serde_json::to_value(&stmt).unwrap();
        assert_eq!(json["kind"], "If");

        let deserialized: StatementIR = serde_json::from_value(json).unwrap();
        match deserialized {
            StatementIR::If { then_, else_, .. } => {
                assert_eq!(then_.len(), 1);
                assert!(else_.is_none());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_try_catch_finally() {
        let stmt = StatementIR::TryCatch {
            try_: vec![StatementIR::Expression {
                expr: ExprIR::Call {
                    callee: Box::new(ExprIR::ident("riskyOp")),
                    args: vec![],
                },
            }],
            catch_: CatchClauseIR {
                binding: "err".to_string(),
                body: vec![StatementIR::Throw {
                    value: ExprIR::ident("err"),
                }],
            },
            finally_: Some(vec![StatementIR::Expression {
                expr: ExprIR::Call {
                    callee: Box::new(ExprIR::ident("cleanup")),
                    args: vec![],
                },
            }]),
        };

        let json = serde_json::to_value(&stmt).unwrap();
        assert_eq!(json["kind"], "TryCatch");
        assert!(json["finally"].is_array());

        let deserialized: StatementIR = serde_json::from_value(json).unwrap();
        match deserialized {
            StatementIR::TryCatch {
                try_,
                catch_,
                finally_,
            } => {
                assert_eq!(try_.len(), 1);
                assert_eq!(catch_.binding, "err");
                assert!(finally_.is_some());
            }
            _ => panic!("Expected TryCatch"),
        }
    }

    #[test]
    fn test_match_statement() {
        let stmt = StatementIR::Match {
            expr: ExprIR::ident("role"),
            arms: vec![
                MatchArmIR {
                    pattern: ExprIR::literal(serde_json::json!("admin")),
                    body: vec![StatementIR::Return {
                        value: Some(ExprIR::literal(serde_json::json!(true))),
                    }],
                },
                MatchArmIR {
                    pattern: ExprIR::literal(serde_json::json!("user")),
                    body: vec![StatementIR::Return {
                        value: Some(ExprIR::literal(serde_json::json!(false))),
                    }],
                },
            ],
        };

        let json = serde_json::to_value(&stmt).unwrap();
        let deserialized: StatementIR = serde_json::from_value(json).unwrap();
        match deserialized {
            StatementIR::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
            }
            _ => panic!("Expected Match"),
        }
    }

    #[test]
    fn test_for_loop() {
        let stmt = StatementIR::For {
            binding: "item".to_string(),
            iterable: ExprIR::ident("items"),
            body: vec![StatementIR::Expression {
                expr: ExprIR::Call {
                    callee: Box::new(ExprIR::ident("process")),
                    args: vec![ExprIR::ident("item")],
                },
            }],
        };

        let json = serde_json::to_value(&stmt).unwrap();
        let deserialized: StatementIR = serde_json::from_value(json).unwrap();
        match deserialized {
            StatementIR::For { binding, body, .. } => {
                assert_eq!(binding, "item");
                assert_eq!(body.len(), 1);
            }
            _ => panic!("Expected For"),
        }
    }
}
