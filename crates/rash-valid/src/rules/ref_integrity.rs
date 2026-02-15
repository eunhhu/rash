use rash_spec::loader::LoadedProject;
use rash_spec::resolver::{RefContext, Resolver};
use rash_spec::types::error::ValidationReport;

/// Check that all references in the project resolve to existing symbols.
pub fn check(project: &LoadedProject, resolver: &Resolver, report: &mut ValidationReport) {
    // Check route references
    for (file, route) in &project.routes {
        for (method, endpoint) in &route.methods {
            let method_str = serde_json::to_value(method)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            // Check handler ref
            let handler_path = format!("$.methods.{}.handler.ref", method_str);
            if let Err(err) = resolver.resolve_or_error(
                &endpoint.handler.reference,
                RefContext::Handler,
                file,
                &handler_path,
            ) {
                report.push(err);
            }

            // Check middleware refs
            for (i, mw_ref) in endpoint.middleware.iter().enumerate() {
                let mw_path = format!("$.methods.{}.middleware[{}].ref", method_str, i);
                if let Err(err) = resolver.resolve_or_error(
                    &mw_ref.reference,
                    RefContext::Middleware,
                    file,
                    &mw_path,
                ) {
                    report.push(err);
                }
            }

            // Check request schema refs
            if let Some(req) = &endpoint.request {
                if let Some(query_ref) = &req.query {
                    let path = format!("$.methods.{}.request.query.ref", method_str);
                    if let Err(err) = resolver.resolve_or_error(
                        &query_ref.reference,
                        RefContext::Schema,
                        file,
                        &path,
                    ) {
                        report.push(err);
                    }
                }
                if let Some(body) = &req.body {
                    let path = format!("$.methods.{}.request.body.ref", method_str);
                    if let Err(err) = resolver.resolve_or_error(
                        &body.reference,
                        RefContext::Schema,
                        file,
                        &path,
                    ) {
                        report.push(err);
                    }
                }
            }

            // Check response schema refs
            if let Some(responses) = &endpoint.response {
                for (status, resp) in responses {
                    if let Some(schema_ref) = &resp.schema {
                        let path =
                            format!("$.methods.{}.response.{}.schema.ref", method_str, status);
                        if let Err(err) = resolver.resolve_or_error(
                            &schema_ref.reference,
                            RefContext::Schema,
                            file,
                            &path,
                        ) {
                            report.push(err);
                        }
                    }
                }
            }
        }
    }

    // Check global middleware refs in config
    if let Some(mw_config) = &project.config.middleware {
        for (i, mw_ref) in mw_config.global.iter().enumerate() {
            let path = format!("$.middleware.global[{}].ref", i);
            if let Err(err) = resolver.resolve_or_error(
                &mw_ref.reference,
                RefContext::Middleware,
                "rash.config.json",
                &path,
            ) {
                report.push(err);
            }
        }
    }

    // Check middleware handler refs
    for (file, mw) in &project.middleware {
        if let Some(handler_ref) = &mw.handler {
            if let Err(err) = resolver.resolve_or_error(
                &handler_ref.reference,
                RefContext::Handler,
                file,
                "$.handler.ref",
            ) {
                report.push(err);
            }
        }
    }

    // Check model relation target refs
    for (file, model) in &project.models {
        for (rel_name, rel) in &model.relations {
            let path = format!("$.relations.{}.target", rel_name);
            if let Err(err) = resolver.resolve_or_error(
                &rel.target,
                RefContext::Model,
                file,
                &path,
            ) {
                report.push(err);
            }
        }
    }
}
