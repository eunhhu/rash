pub mod detect;
pub mod express;
pub mod handler_extract;
pub mod schema_extract;

use rash_spec::types::handler::HandlerSpec;
use rash_spec::types::middleware::MiddlewareSpec;
use rash_spec::types::route::RouteSpec;
use rash_spec::types::schema::SchemaSpec;

use crate::error::ReverseParseError;
use detect::DetectedFramework;

/// Result of reverse-parsing source code into Rash spec elements.
#[derive(Debug, Clone)]
pub struct ReverseParseResult {
    pub framework: DetectedFramework,
    pub routes: Vec<RouteSpec>,
    pub schemas: Vec<SchemaSpec>,
    pub middleware: Vec<MiddlewareSpec>,
    pub handlers: Vec<HandlerSpec>,
    pub warnings: Vec<String>,
}

/// Reverse-parse a TypeScript/JavaScript source file into Rash spec elements.
///
/// Currently supports Express. Unknown frameworks produce an error.
pub fn reverse_parse(
    source_code: &str,
    file_name: &str,
) -> Result<ReverseParseResult, ReverseParseError> {
    let framework = detect::detect_framework(source_code);

    match framework {
        DetectedFramework::Express => {
            let mut warnings = Vec::new();

            let schemas = schema_extract::extract_schemas(source_code, &mut warnings);
            let (routes, handlers, middleware) =
                express::extract_express(source_code, &mut warnings);

            Ok(ReverseParseResult {
                framework,
                routes,
                schemas,
                middleware,
                handlers,
                warnings,
            })
        }
        DetectedFramework::Unknown => Err(ReverseParseError::UnsupportedFramework(
            file_name.to_string(),
        )),
        other => Err(ReverseParseError::UnsupportedFramework(format!(
            "{other:?} (from {file_name})"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_parse_express_basic() {
        let source = r#"
import express from "express";

const app = express();

app.get("/users", async (req, res) => {
    const users = await prisma.user.findMany();
    res.json(users);
});

app.listen(3000);
"#;

        let result = reverse_parse(source, "app.ts").unwrap();
        assert_eq!(result.framework, DetectedFramework::Express);
        assert!(!result.routes.is_empty());
        assert!(!result.handlers.is_empty());
    }

    #[test]
    fn test_reverse_parse_unknown_framework() {
        let source = r#"
console.log("hello world");
"#;
        let result = reverse_parse(source, "app.ts");
        assert!(result.is_err());
    }
}
