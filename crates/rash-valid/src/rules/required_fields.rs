use rash_spec::loader::LoadedProject;
use rash_spec::types::error::{ErrorEntry, ValidationReport, E_MISSING_FIELD};

/// Check that required fields are present in spec files.
pub fn check(project: &LoadedProject, report: &mut ValidationReport) {
    // Check config required fields
    if project.config.name.is_empty() {
        report.push(
            ErrorEntry::error(
                E_MISSING_FIELD,
                "Project name is required",
                "rash.config.json",
                "$.name",
            )
            .with_suggestion("Add a 'name' field to rash.config.json"),
        );
    }

    if project.config.version.is_empty() {
        report.push(
            ErrorEntry::error(
                E_MISSING_FIELD,
                "Project version is required",
                "rash.config.json",
                "$.version",
            )
            .with_suggestion("Add a 'version' field (e.g., '1.0.0') to rash.config.json"),
        );
    }

    // Check route required fields
    for (file, route) in &project.routes {
        if route.path.is_empty() {
            report.push(
                ErrorEntry::error(E_MISSING_FIELD, "Route path is required", file, "$.path")
                    .with_suggestion("Add a 'path' field (e.g., '/v1/users')"),
            );
        }

        if route.methods.is_empty() {
            report.push(
                ErrorEntry::error(
                    E_MISSING_FIELD,
                    "Route must have at least one HTTP method",
                    file,
                    "$.methods",
                )
                .with_suggestion("Add at least one method (GET, POST, etc.) to 'methods'"),
            );
        }

        for (method, endpoint) in &route.methods {
            if endpoint.handler.reference.is_empty() {
                let method_str = serde_json::to_value(method)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                report.push(
                    ErrorEntry::error(
                        E_MISSING_FIELD,
                        format!("Handler reference is required for {} {}", method_str, route.path),
                        file,
                        &format!("$.methods.{}.handler.ref", method_str),
                    )
                    .with_suggestion("Add a handler reference (e.g., { \"ref\": \"users.getUser\" })"),
                );
            }
        }
    }

    // Check schema required fields
    for (file, schema) in &project.schemas {
        if schema.name.is_empty() {
            report.push(
                ErrorEntry::error(E_MISSING_FIELD, "Schema name is required", file, "$.name")
                    .with_suggestion("Add a 'name' field to the schema"),
            );
        }
    }

    // Check model required fields
    for (file, model) in &project.models {
        if model.name.is_empty() {
            report.push(
                ErrorEntry::error(E_MISSING_FIELD, "Model name is required", file, "$.name")
                    .with_suggestion("Add a 'name' field to the model"),
            );
        }

        if model.columns.is_empty() {
            report.push(
                ErrorEntry::error(
                    E_MISSING_FIELD,
                    "Model must have at least one column",
                    file,
                    "$.columns",
                )
                .with_suggestion("Add at least one column definition"),
            );
        }
    }

    // Check middleware required fields
    for (file, mw) in &project.middleware {
        if mw.name.is_empty() {
            report.push(
                ErrorEntry::error(
                    E_MISSING_FIELD,
                    "Middleware name is required",
                    file,
                    "$.name",
                )
                .with_suggestion("Add a 'name' field to the middleware"),
            );
        }
    }

    // Check handler required fields
    for (file, handler) in &project.handlers {
        if handler.name.is_empty() {
            report.push(
                ErrorEntry::error(
                    E_MISSING_FIELD,
                    "Handler name is required",
                    file,
                    "$.name",
                )
                .with_suggestion("Add a 'name' field to the handler"),
            );
        }
    }
}
