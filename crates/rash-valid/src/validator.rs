use rash_spec::index::build_index;
use rash_spec::loader::LoadedProject;
use rash_spec::resolver::Resolver;
use rash_spec::types::error::ValidationReport;

use crate::rules;

/// Validate a loaded project.
/// Runs all validation rules and returns a consolidated report.
pub fn validate(project: &LoadedProject) -> ValidationReport {
    let mut report = ValidationReport::success();

    // Build symbol index
    let (index, index_errors) = build_index(project);
    for err in index_errors {
        report.push(err);
    }

    // Run validation rules
    let resolver = Resolver::new(&index);

    rules::ref_integrity::check(project, &resolver, &mut report);
    rules::required_fields::check(project, &mut report);
    rules::type_consistency::check(project, &index, &mut report);
    rules::cycle_detect::check(project, &index, &mut report);

    report
}

/// Validate only reference integrity (useful for targeted checking)
pub fn validate_refs(project: &LoadedProject) -> ValidationReport {
    let mut report = ValidationReport::success();
    let (index, index_errors) = build_index(project);
    for err in index_errors {
        report.push(err);
    }
    let resolver = Resolver::new(&index);
    rules::ref_integrity::check(project, &resolver, &mut report);
    report
}
