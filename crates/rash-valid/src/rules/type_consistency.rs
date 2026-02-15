use rash_spec::index::SpecIndex;
use rash_spec::loader::LoadedProject;
use rash_spec::types::error::{ErrorEntry, ValidationReport, E_INCOMPATIBLE_TARGET};

/// Check type consistency across the project.
pub fn check(project: &LoadedProject, _index: &SpecIndex, report: &mut ValidationReport) {
    check_language_framework_compatibility(project, report);
}

/// Verify that the target language and framework are compatible
fn check_language_framework_compatibility(
    project: &LoadedProject,
    report: &mut ValidationReport,
) {
    use rash_spec::types::common::{Framework, Language};

    let lang = project.config.target.language;
    let fw = project.config.target.framework;

    let compatible = match lang {
        Language::Typescript => matches!(
            fw,
            Framework::Express
                | Framework::Fastify
                | Framework::Hono
                | Framework::Elysia
                | Framework::NestJS
        ),
        Language::Rust => matches!(fw, Framework::Actix | Framework::Axum | Framework::Rocket),
        Language::Python => {
            matches!(fw, Framework::FastAPI | Framework::Django | Framework::Flask)
        }
        Language::Go => matches!(fw, Framework::Gin | Framework::Echo | Framework::Fiber),
    };

    if !compatible {
        let lang_str = serde_json::to_value(lang)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let fw_str = serde_json::to_value(fw)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        report.push(
            ErrorEntry::error(
                E_INCOMPATIBLE_TARGET,
                format!(
                    "Framework '{}' is not compatible with language '{}'",
                    fw_str, lang_str
                ),
                "rash.config.json",
                "$.target",
            )
            .with_suggestion(format!(
                "Choose a framework compatible with '{}'. See docs for the compatibility table.",
                lang_str
            )),
        );
    }
}
