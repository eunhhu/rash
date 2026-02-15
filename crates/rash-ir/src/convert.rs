use std::collections::HashSet;

use indexmap::IndexMap;
use thiserror::Error;

use rash_spec::loader::LoadedProject;
use rash_spec::types::ast::{AstNode, CatchClause, MatchArm, TemplatePart};
use rash_spec::types::common::{Language, Tier, TypeRef};
use rash_spec::types::handler::HandlerSpec;
use rash_spec::types::middleware::MiddlewareSpec;
use rash_spec::types::model::ModelSpec;
use rash_spec::types::route::{EndpointSpec, RouteSpec};
use rash_spec::types::schema::SchemaSpec;

use crate::expr::{
    DbMutateIR, DbQueryIR, ExprIR, HttpRespondIR, NativeBridgeCallIR, NativeBridgeIR,
    NativeBridgeImportIR, TemplatePartIR, TypeIR,
};
use crate::statement::{CatchClauseIR, MatchArmIR, StatementIR};
use crate::types::{
    EndpointIR, HandlerIR, MiddlewareIR, ModelIR, ParamIR, ProjectIR, RequestIR, ResponseIR,
    RouteIR, SchemaIR,
};

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("handler '{name}' has no body statements")]
    EmptyHandler { name: String },
}

/// Convert a loaded project (spec types) into the intermediate representation.
pub fn convert_project(project: &LoadedProject) -> Result<ProjectIR, ConvertError> {
    let config = serde_json::to_value(&project.config).unwrap_or_default();

    let routes = project
        .routes
        .iter()
        .map(|(_, route)| convert_route(route))
        .collect();

    let schemas = project
        .schemas
        .iter()
        .map(|(_, schema)| convert_schema(schema))
        .collect();

    let models = project
        .models
        .iter()
        .map(|(_, model)| convert_model(model))
        .collect();

    let middleware = project
        .middleware
        .iter()
        .map(|(_, mw)| convert_middleware(mw))
        .collect();

    let handlers = project
        .handlers
        .iter()
        .map(|(_, handler)| convert_handler(handler))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProjectIR {
        config,
        routes,
        schemas,
        models,
        middleware,
        handlers,
    })
}

fn convert_route(route: &RouteSpec) -> RouteIR {
    let methods = route
        .methods
        .iter()
        .map(|(method, endpoint)| (*method, convert_endpoint(endpoint)))
        .collect();

    RouteIR {
        path: route.path.clone(),
        methods,
        tags: route.tags.clone(),
    }
}

fn convert_endpoint(endpoint: &EndpointSpec) -> EndpointIR {
    let operation_id = endpoint
        .operation_id
        .clone()
        .unwrap_or_else(|| endpoint.handler.reference.clone());

    let middleware = endpoint
        .middleware
        .iter()
        .map(|r| r.reference.clone())
        .collect();

    let request = match &endpoint.request {
        Some(req) => RequestIR {
            query_schema: req.query.as_ref().map(|r| r.reference.clone()),
            body_schema: req.body.as_ref().map(|b| b.reference.clone()),
            content_type: req.body.as_ref().and_then(|b| b.content_type.clone()),
        },
        None => RequestIR {
            query_schema: None,
            body_schema: None,
            content_type: None,
        },
    };

    let response = endpoint
        .response
        .as_ref()
        .map(|resp_map| {
            resp_map
                .iter()
                .filter_map(|(status_str, resp)| {
                    status_str.parse::<u16>().ok().map(|code| {
                        (
                            code,
                            ResponseIR {
                                description: resp.description.clone(),
                                schema_ref: resp.schema.as_ref().map(|r| r.reference.clone()),
                            },
                        )
                    })
                })
                .collect::<IndexMap<_, _>>()
        })
        .unwrap_or_default();

    EndpointIR {
        operation_id,
        summary: endpoint.summary.clone(),
        handler_ref: endpoint.handler.reference.clone(),
        middleware,
        request,
        response,
    }
}

fn convert_schema(schema: &SchemaSpec) -> SchemaIR {
    SchemaIR {
        name: schema.name.clone(),
        definitions: schema.definitions.clone(),
    }
}

fn convert_model(model: &ModelSpec) -> ModelIR {
    let table_name = model
        .table_name
        .clone()
        .unwrap_or_else(|| format!("{}s", model.name.to_lowercase()));

    let columns = model
        .columns
        .iter()
        .map(|(name, col)| (name.clone(), serde_json::to_value(col).unwrap_or_default()))
        .collect();

    let relations = model
        .relations
        .iter()
        .map(|(name, rel)| (name.clone(), serde_json::to_value(rel).unwrap_or_default()))
        .collect();

    let indexes = model
        .indexes
        .iter()
        .map(|idx| serde_json::to_value(idx).unwrap_or_default())
        .collect();

    ModelIR {
        name: model.name.clone(),
        table_name,
        columns,
        relations,
        indexes,
    }
}

fn convert_middleware(mw: &MiddlewareSpec) -> MiddlewareIR {
    let middleware_type = serde_json::to_value(mw.middleware_type)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "request".to_string());

    MiddlewareIR {
        name: mw.name.clone(),
        middleware_type,
        handler_ref: mw.handler.as_ref().map(|r| r.reference.clone()),
    }
}

fn convert_handler(handler: &HandlerSpec) -> Result<HandlerIR, ConvertError> {
    let params = handler
        .params
        .as_ref()
        .map(|p| {
            p.iter()
                .map(|(name, param)| ParamIR {
                    name: name.clone(),
                    type_ir: convert_type_ref_str(&param.param_type),
                })
                .collect()
        })
        .unwrap_or_default();

    let return_type = handler
        .return_type
        .as_ref()
        .map(|tr| convert_type_ref(tr))
        .unwrap_or(TypeIR::Void);

    let mut max_tier = Tier::Universal;
    let mut bridge_languages = HashSet::new();

    let body: Vec<StatementIR> = handler
        .body
        .iter()
        .map(|node| convert_ast_to_statement(node, &mut max_tier, &mut bridge_languages))
        .collect();

    Ok(HandlerIR {
        name: handler.name.clone(),
        is_async: handler.is_async,
        params,
        return_type,
        body,
        max_tier,
        bridge_languages,
    })
}

fn convert_type_ref(tr: &TypeRef) -> TypeIR {
    match tr {
        TypeRef::Simple(s) => convert_type_ref_str(s),
        TypeRef::Reference(r) => TypeIR::Ref(r.reference.clone()),
        TypeRef::Complex { reference, .. } => TypeIR::Ref(reference.clone()),
    }
}

fn convert_type_ref_str(s: &str) -> TypeIR {
    match s {
        "string" | "String" => TypeIR::String,
        "number" | "Number" | "int" | "float" | "i32" | "i64" | "f32" | "f64" => TypeIR::Number,
        "boolean" | "Boolean" | "bool" => TypeIR::Boolean,
        "null" | "None" | "nil" => TypeIR::Null,
        "void" | "unit" | "()" => TypeIR::Void,
        "any" | "Any" => TypeIR::Any,
        other => TypeIR::Ref(other.to_string()),
    }
}

fn update_tier(current: &mut Tier, node_tier: Tier) {
    if node_tier > *current {
        *current = node_tier;
    }
}

fn convert_ast_to_statement(
    node: &AstNode,
    max_tier: &mut Tier,
    bridge_langs: &mut HashSet<Language>,
) -> StatementIR {
    match node {
        AstNode::LetStatement {
            tier,
            name,
            value_type,
            value,
        } => {
            update_tier(max_tier, *tier);
            StatementIR::Let {
                name: name.clone(),
                type_: value_type.as_ref().map(convert_type_ref),
                value: convert_ast_to_expr(value, max_tier, bridge_langs),
            }
        }
        AstNode::AssignStatement {
            tier,
            target,
            value,
        } => {
            update_tier(max_tier, *tier);
            StatementIR::Assign {
                target: convert_ast_to_expr(target, max_tier, bridge_langs),
                value: convert_ast_to_expr(value, max_tier, bridge_langs),
            }
        }
        AstNode::ReturnStatement { tier, value } => {
            update_tier(max_tier, *tier);
            StatementIR::Return {
                value: Some(convert_ast_to_expr(value, max_tier, bridge_langs)),
            }
        }
        AstNode::IfStatement {
            tier,
            condition,
            then,
            else_branch,
        } => {
            update_tier(max_tier, *tier);
            StatementIR::If {
                condition: convert_ast_to_expr(condition, max_tier, bridge_langs),
                then_: then
                    .iter()
                    .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
                    .collect(),
                else_: else_branch.as_ref().map(|stmts| {
                    stmts
                        .iter()
                        .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
                        .collect()
                }),
            }
        }
        AstNode::ForStatement {
            tier,
            binding,
            iterable,
            body,
        } => {
            update_tier(max_tier, *tier);
            StatementIR::For {
                binding: binding.clone(),
                iterable: convert_ast_to_expr(iterable, max_tier, bridge_langs),
                body: body
                    .iter()
                    .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
                    .collect(),
            }
        }
        AstNode::WhileStatement {
            tier,
            condition,
            body,
        } => {
            update_tier(max_tier, *tier);
            StatementIR::While {
                condition: convert_ast_to_expr(condition, max_tier, bridge_langs),
                body: body
                    .iter()
                    .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
                    .collect(),
            }
        }
        AstNode::MatchStatement { tier, expr, arms } => {
            update_tier(max_tier, *tier);
            StatementIR::Match {
                expr: convert_ast_to_expr(expr, max_tier, bridge_langs),
                arms: arms
                    .iter()
                    .map(|arm| convert_match_arm(arm, max_tier, bridge_langs))
                    .collect(),
            }
        }
        AstNode::TryCatchStatement {
            tier,
            try_block,
            catch_block,
            finally_block,
        } => {
            update_tier(max_tier, *tier);
            StatementIR::TryCatch {
                try_: try_block
                    .iter()
                    .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
                    .collect(),
                catch_: convert_catch_clause(catch_block, max_tier, bridge_langs),
                finally_: finally_block.as_ref().map(|stmts| {
                    stmts
                        .iter()
                        .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
                        .collect()
                }),
            }
        }
        AstNode::ThrowStatement { tier, value } => {
            update_tier(max_tier, *tier);
            StatementIR::Throw {
                value: convert_ast_to_expr(value, max_tier, bridge_langs),
            }
        }
        AstNode::ExpressionStatement { tier, expr } => {
            update_tier(max_tier, *tier);
            StatementIR::Expression {
                expr: convert_ast_to_expr(expr, max_tier, bridge_langs),
            }
        }
        // Expression nodes used in statement position → wrap as Expression statement
        other => StatementIR::Expression {
            expr: convert_ast_to_expr(other, max_tier, bridge_langs),
        },
    }
}

fn convert_ast_to_expr(
    node: &AstNode,
    max_tier: &mut Tier,
    bridge_langs: &mut HashSet<Language>,
) -> ExprIR {
    match node {
        AstNode::Literal { tier, value } => {
            update_tier(max_tier, *tier);
            ExprIR::Literal {
                value: value.clone(),
            }
        }
        AstNode::Identifier { tier, name } => {
            update_tier(max_tier, *tier);
            ExprIR::Identifier { name: name.clone() }
        }
        AstNode::BinaryExpr {
            tier,
            operator,
            left,
            right,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::Binary {
                op: operator.clone(),
                left: Box::new(convert_ast_to_expr(left, max_tier, bridge_langs)),
                right: Box::new(convert_ast_to_expr(right, max_tier, bridge_langs)),
            }
        }
        AstNode::UnaryExpr {
            tier,
            operator,
            operand,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::Unary {
                op: operator.clone(),
                operand: Box::new(convert_ast_to_expr(operand, max_tier, bridge_langs)),
            }
        }
        AstNode::CallExpr {
            tier,
            callee,
            args,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::Call {
                callee: Box::new(convert_ast_to_expr(callee, max_tier, bridge_langs)),
                args: args
                    .iter()
                    .map(|a| convert_ast_to_expr(a, max_tier, bridge_langs))
                    .collect(),
            }
        }
        AstNode::MemberExpr {
            tier,
            object,
            property,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::Member {
                object: Box::new(convert_ast_to_expr(object, max_tier, bridge_langs)),
                property: property.clone(),
            }
        }
        AstNode::IndexExpr {
            tier,
            object,
            index,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::Index {
                object: Box::new(convert_ast_to_expr(object, max_tier, bridge_langs)),
                index: Box::new(convert_ast_to_expr(index, max_tier, bridge_langs)),
            }
        }
        AstNode::ObjectExpr { tier, properties } => {
            update_tier(max_tier, *tier);
            ExprIR::Object {
                properties: properties
                    .iter()
                    .map(|(k, v)| (k.clone(), convert_ast_to_expr(v, max_tier, bridge_langs)))
                    .collect(),
            }
        }
        AstNode::ArrayExpr { tier, elements } => {
            update_tier(max_tier, *tier);
            ExprIR::Array {
                elements: elements
                    .iter()
                    .map(|e| convert_ast_to_expr(e, max_tier, bridge_langs))
                    .collect(),
            }
        }
        AstNode::ArrowFn {
            tier,
            params,
            body,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::ArrowFn {
                params: params.clone(),
                body: body
                    .iter()
                    .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
                    .collect(),
            }
        }
        AstNode::AwaitExpr { tier, expr } => {
            update_tier(max_tier, *tier);
            ExprIR::Await {
                expr: Box::new(convert_ast_to_expr(expr, max_tier, bridge_langs)),
            }
        }
        AstNode::PipeExpr { tier, stages } => {
            update_tier(max_tier, *tier);
            ExprIR::Pipe {
                stages: stages
                    .iter()
                    .map(|s| convert_ast_to_expr(s, max_tier, bridge_langs))
                    .collect(),
            }
        }
        AstNode::TemplateString { tier, parts } => {
            update_tier(max_tier, *tier);
            ExprIR::Template {
                parts: parts
                    .iter()
                    .map(|p| convert_template_part(p, max_tier, bridge_langs))
                    .collect(),
            }
        }
        // Domain nodes (Tier 1)
        AstNode::DbQuery {
            tier,
            model,
            operation,
            r#where,
            order_by,
            skip,
            take,
            select,
            include,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::DbQuery(DbQueryIR {
                model: model.clone(),
                operation: operation.clone(),
                r#where: r#where.clone(),
                order_by: order_by.clone(),
                skip: skip
                    .as_ref()
                    .map(|s| Box::new(convert_ast_to_expr(s, max_tier, bridge_langs))),
                take: take
                    .as_ref()
                    .map(|t| Box::new(convert_ast_to_expr(t, max_tier, bridge_langs))),
                select: select.clone(),
                include: include.clone(),
            })
        }
        AstNode::DbMutate {
            tier,
            model,
            operation,
            data,
            r#where,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::DbMutate(DbMutateIR {
                model: model.clone(),
                operation: operation.clone(),
                data: data
                    .as_ref()
                    .map(|d| Box::new(convert_ast_to_expr(d, max_tier, bridge_langs))),
                r#where: r#where.clone(),
            })
        }
        AstNode::HttpRespond {
            tier,
            status,
            headers,
            body,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::HttpRespond(HttpRespondIR {
                status: *status,
                headers: headers.as_ref().map(|h| {
                    h.iter()
                        .map(|(k, v)| (k.clone(), convert_ast_to_expr(v, max_tier, bridge_langs)))
                        .collect()
                }),
                body: body
                    .as_ref()
                    .map(|b| Box::new(convert_ast_to_expr(b, max_tier, bridge_langs))),
            })
        }
        AstNode::CtxGet { tier, path } => {
            update_tier(max_tier, *tier);
            ExprIR::CtxGet { path: path.clone() }
        }
        AstNode::Validate { tier, schema, data } => {
            update_tier(max_tier, *tier);
            ExprIR::Validate {
                schema: schema.clone(),
                data: Box::new(convert_ast_to_expr(data, max_tier, bridge_langs)),
            }
        }
        AstNode::HashPassword {
            tier,
            input,
            algorithm,
            rounds,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::HashPassword {
                input: Box::new(convert_ast_to_expr(input, max_tier, bridge_langs)),
                algorithm: algorithm.clone(),
                rounds: *rounds,
            }
        }
        AstNode::VerifyPassword {
            tier,
            password,
            hash,
            algorithm,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::VerifyPassword {
                password: Box::new(convert_ast_to_expr(password, max_tier, bridge_langs)),
                hash: Box::new(convert_ast_to_expr(hash, max_tier, bridge_langs)),
                algorithm: algorithm.clone(),
            }
        }
        AstNode::SignToken {
            tier,
            payload,
            options,
            ..
        } => {
            update_tier(max_tier, *tier);
            ExprIR::SignToken {
                payload: Box::new(convert_ast_to_expr(payload, max_tier, bridge_langs)),
                options: options.clone(),
            }
        }
        AstNode::VerifyToken { tier, token, .. } => {
            update_tier(max_tier, *tier);
            ExprIR::VerifyToken {
                token: Box::new(convert_ast_to_expr(token, max_tier, bridge_langs)),
            }
        }
        // Utility nodes → map to Call expressions
        AstNode::SendEmail {
            tier,
            to,
            subject,
            body,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::Call {
                callee: Box::new(ExprIR::ident("sendEmail")),
                args: vec![
                    convert_ast_to_expr(to, max_tier, bridge_langs),
                    convert_ast_to_expr(subject, max_tier, bridge_langs),
                    convert_ast_to_expr(body, max_tier, bridge_langs),
                ],
            }
        }
        AstNode::EmitEvent { tier, event, data } => {
            update_tier(max_tier, *tier);
            let mut args = vec![ExprIR::literal(serde_json::json!(event))];
            if let Some(d) = data {
                args.push(convert_ast_to_expr(d, max_tier, bridge_langs));
            }
            ExprIR::Call {
                callee: Box::new(ExprIR::ident("emitEvent")),
                args,
            }
        }
        AstNode::LogMessage {
            tier,
            level,
            message,
        } => {
            update_tier(max_tier, *tier);
            ExprIR::Call {
                callee: Box::new(ExprIR::Member {
                    object: Box::new(ExprIR::ident("logger")),
                    property: level.clone(),
                }),
                args: vec![convert_ast_to_expr(message, max_tier, bridge_langs)],
            }
        }
        // Bridge node
        AstNode::NativeBridge {
            tier,
            language,
            package,
            import,
            call,
            return_type,
            ..
        } => {
            update_tier(max_tier, *tier);
            bridge_langs.insert(*language);
            ExprIR::NativeBridge(NativeBridgeIR {
                language: *language,
                package: package.clone(),
                import: NativeBridgeImportIR {
                    name: import.name.clone(),
                    from: import.from.clone(),
                },
                call: NativeBridgeCallIR {
                    method: call.method.clone(),
                    args: call
                        .args
                        .iter()
                        .map(|a| convert_ast_to_expr(a, max_tier, bridge_langs))
                        .collect(),
                },
                return_type: return_type.clone(),
            })
        }
        // Statement nodes that appear in expression position → fallback to literal null
        AstNode::LetStatement { .. }
        | AstNode::AssignStatement { .. }
        | AstNode::ReturnStatement { .. }
        | AstNode::IfStatement { .. }
        | AstNode::ForStatement { .. }
        | AstNode::WhileStatement { .. }
        | AstNode::MatchStatement { .. }
        | AstNode::TryCatchStatement { .. }
        | AstNode::ThrowStatement { .. }
        | AstNode::ExpressionStatement { .. } => ExprIR::Literal {
            value: serde_json::Value::Null,
        },
    }
}

fn convert_match_arm(
    arm: &MatchArm,
    max_tier: &mut Tier,
    bridge_langs: &mut HashSet<Language>,
) -> MatchArmIR {
    MatchArmIR {
        pattern: convert_ast_to_expr(&arm.pattern, max_tier, bridge_langs),
        body: arm
            .body
            .iter()
            .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
            .collect(),
    }
}

fn convert_catch_clause(
    clause: &CatchClause,
    max_tier: &mut Tier,
    bridge_langs: &mut HashSet<Language>,
) -> CatchClauseIR {
    CatchClauseIR {
        binding: clause.binding.clone(),
        body: clause
            .body
            .iter()
            .map(|n| convert_ast_to_statement(n, max_tier, bridge_langs))
            .collect(),
    }
}

fn convert_template_part(
    part: &TemplatePart,
    max_tier: &mut Tier,
    bridge_langs: &mut HashSet<Language>,
) -> TemplatePartIR {
    match part {
        TemplatePart::Text { value } => TemplatePartIR::Text {
            value: value.clone(),
        },
        TemplatePart::Expr { value } => TemplatePartIR::Expr {
            value: Box::new(convert_ast_to_expr(value, max_tier, bridge_langs)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    use rash_spec::types::common::{Framework, Runtime};
    use rash_spec::types::config::{RashConfig, ServerConfig, TargetConfig};

    fn minimal_project() -> LoadedProject {
        LoadedProject {
            root: PathBuf::from("/tmp/test"),
            config: RashConfig {
                schema: None,
                version: "1.0.0".to_string(),
                name: "test".to_string(),
                description: None,
                target: TargetConfig {
                    language: Language::Typescript,
                    framework: Framework::Express,
                    runtime: Runtime::Bun,
                },
                server: ServerConfig {
                    port: 3000,
                    host: "0.0.0.0".to_string(),
                    protocol: None,
                    base_path: None,
                },
                database: None,
                codegen: None,
                middleware: None,
                plugins: vec![],
                meta: None,
            },
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        }
    }

    #[test]
    fn test_convert_empty_project() {
        let project = minimal_project();
        let ir = convert_project(&project).unwrap();

        assert!(ir.routes.is_empty());
        assert!(ir.schemas.is_empty());
        assert!(ir.models.is_empty());
        assert!(ir.middleware.is_empty());
        assert!(ir.handlers.is_empty());
        assert_eq!(ir.config["name"], "test");
    }

    #[test]
    fn test_convert_route_with_endpoint() {
        let mut project = minimal_project();

        let route_json = serde_json::json!({
            "path": "/v1/users",
            "methods": {
                "GET": {
                    "operationId": "listUsers",
                    "summary": "List users",
                    "handler": { "ref": "users.listUsers" },
                    "middleware": [{ "ref": "auth" }],
                    "request": {
                        "query": { "ref": "ListUsersQuery" }
                    },
                    "response": {
                        "200": {
                            "description": "OK",
                            "schema": { "ref": "UserListResponse" }
                        }
                    }
                }
            },
            "tags": ["users"]
        });
        let route: RouteSpec = serde_json::from_value(route_json).unwrap();
        project.routes.push(("routes/users.route.json".into(), route));

        let ir = convert_project(&project).unwrap();
        assert_eq!(ir.routes.len(), 1);
        assert_eq!(ir.routes[0].path, "/v1/users");
        assert_eq!(ir.routes[0].tags, vec!["users"]);

        let endpoint = &ir.routes[0].methods[&rash_spec::types::common::HttpMethod::Get];
        assert_eq!(endpoint.operation_id, "listUsers");
        assert_eq!(endpoint.handler_ref, "users.listUsers");
        assert_eq!(endpoint.middleware, vec!["auth"]);
        assert_eq!(
            endpoint.request.query_schema.as_deref(),
            Some("ListUsersQuery")
        );
        assert_eq!(
            endpoint.response[&200].schema_ref.as_deref(),
            Some("UserListResponse")
        );
    }

    #[test]
    fn test_convert_handler_with_body() {
        let mut project = minimal_project();

        let handler_json = serde_json::json!({
            "name": "getUser",
            "async": true,
            "params": {
                "ctx": { "type": "RequestContext" }
            },
            "returnType": { "ref": "HttpResponse" },
            "body": [
                {
                    "type": "LetStatement",
                    "tier": 0,
                    "name": "userId",
                    "value": { "type": "CtxGet", "tier": 1, "path": "params.id" }
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
                            "where": { "id": { "type": "Identifier", "tier": 0, "name": "userId" } }
                        }
                    }
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
            ]
        });
        let handler: HandlerSpec = serde_json::from_value(handler_json).unwrap();
        project
            .handlers
            .push(("handlers/users.handler.json".into(), handler));

        let ir = convert_project(&project).unwrap();
        assert_eq!(ir.handlers.len(), 1);

        let h = &ir.handlers[0];
        assert_eq!(h.name, "getUser");
        assert!(h.is_async);
        assert_eq!(h.params.len(), 1);
        assert_eq!(h.params[0].name, "ctx");
        assert_eq!(h.return_type, TypeIR::Ref { name: "HttpResponse".into() });
        assert_eq!(h.body.len(), 3);
        assert_eq!(h.max_tier, Tier::Domain);
        assert!(h.bridge_languages.is_empty());
    }

    #[test]
    fn test_convert_handler_with_native_bridge() {
        let mut project = minimal_project();

        let handler_json = serde_json::json!({
            "name": "hashPass",
            "async": true,
            "body": [
                {
                    "type": "ReturnStatement",
                    "tier": 0,
                    "value": {
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
                        "returnType": "string"
                    }
                }
            ]
        });
        let handler: HandlerSpec = serde_json::from_value(handler_json).unwrap();
        project.handlers.push(("handlers/hash.handler.json".into(), handler));

        let ir = convert_project(&project).unwrap();
        let h = &ir.handlers[0];
        assert_eq!(h.max_tier, Tier::Bridge);
        assert!(h.bridge_languages.contains(&Language::Typescript));
    }

    #[test]
    fn test_convert_schema() {
        let mut project = minimal_project();

        let schema = SchemaSpec {
            schema: None,
            name: "User".to_string(),
            description: None,
            definitions: {
                let mut d = IndexMap::new();
                d.insert(
                    "CreateUserBody".to_string(),
                    serde_json::json!({ "type": "object" }),
                );
                d
            },
            meta: None,
        };
        project.schemas.push(("schemas/user.schema.json".into(), schema));

        let ir = convert_project(&project).unwrap();
        assert_eq!(ir.schemas.len(), 1);
        assert_eq!(ir.schemas[0].name, "User");
        assert_eq!(ir.schemas[0].definitions.len(), 1);
    }

    #[test]
    fn test_convert_model_with_auto_table_name() {
        let mut project = minimal_project();

        let model_json = serde_json::json!({
            "name": "User",
            "columns": {
                "id": { "type": "uuid", "primaryKey": true }
            }
        });
        let model: ModelSpec = serde_json::from_value(model_json).unwrap();
        project.models.push(("models/user.model.json".into(), model));

        let ir = convert_project(&project).unwrap();
        assert_eq!(ir.models.len(), 1);
        assert_eq!(ir.models[0].name, "User");
        assert_eq!(ir.models[0].table_name, "users"); // auto-generated
    }

    #[test]
    fn test_convert_middleware() {
        let mut project = minimal_project();

        let mw_json = serde_json::json!({
            "name": "auth",
            "type": "request",
            "handler": { "ref": "auth.verifyToken" }
        });
        let mw: MiddlewareSpec = serde_json::from_value(mw_json).unwrap();
        project
            .middleware
            .push(("middleware/auth.middleware.json".into(), mw));

        let ir = convert_project(&project).unwrap();
        assert_eq!(ir.middleware.len(), 1);
        assert_eq!(ir.middleware[0].name, "auth");
        assert_eq!(ir.middleware[0].middleware_type, "request");
        assert_eq!(
            ir.middleware[0].handler_ref.as_deref(),
            Some("auth.verifyToken")
        );
    }

    #[test]
    fn test_convert_type_ref_str_variants() {
        assert_eq!(convert_type_ref_str("string"), TypeIR::String);
        assert_eq!(convert_type_ref_str("number"), TypeIR::Number);
        assert_eq!(convert_type_ref_str("boolean"), TypeIR::Boolean);
        assert_eq!(convert_type_ref_str("void"), TypeIR::Void);
        assert_eq!(convert_type_ref_str("any"), TypeIR::Any);
        assert_eq!(
            convert_type_ref_str("MyCustomType"),
            TypeIR::Ref { name: "MyCustomType".into() }
        );
    }

    #[test]
    fn test_convert_golden_fixture() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("fixtures/golden-user-crud");

        if !fixture_path.exists() {
            return; // Skip if fixture not available
        }

        let (loaded, _report) = rash_spec::loader::load_project(&fixture_path).unwrap();
        // May have validation warnings for missing handler refs, that's OK
        let ir = convert_project(&loaded).unwrap();

        assert!(!ir.routes.is_empty());
        assert!(!ir.schemas.is_empty());
        assert!(!ir.models.is_empty());
        assert_eq!(ir.config["name"], "golden-user-crud");
    }
}
