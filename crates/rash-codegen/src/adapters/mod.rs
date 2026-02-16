pub mod express;
pub mod actix;
pub mod fastapi;
pub mod gin;

use rash_spec::types::common::Framework;

use crate::error::CodegenError;
use crate::traits::FrameworkAdapter;

/// Create the appropriate framework adapter.
pub fn create_adapter(framework: Framework) -> Result<Box<dyn FrameworkAdapter>, CodegenError> {
    match framework {
        Framework::Express => Ok(Box::new(express::ExpressAdapter)),
        Framework::Actix => Ok(Box::new(actix::ActixAdapter)),
        Framework::FastAPI => Ok(Box::new(fastapi::FastAPIAdapter)),
        Framework::Gin => Ok(Box::new(gin::GinAdapter)),
        other => Err(CodegenError::UnsupportedFramework(other)),
    }
}

/// Convert `:param` path parameters to `{param}` format (Actix, FastAPI).
pub fn convert_colon_params_to_braces(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ':' {
            // Collect the parameter name
            let mut param = String::new();
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    param.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if param.is_empty() {
                result.push(':');
            } else {
                result.push('{');
                result.push_str(&param);
                result.push('}');
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Convert `[param]` path parameters (file-based routing convention) to `:param`.
/// e.g., `/api/v1/users/[id]` → `/api/v1/users/:id`
pub fn convert_bracket_params_to_colon(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            result.push(':');
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next();
                    break;
                }
                result.push(chars.next().unwrap());
            }
        } else {
            result.push(ch);
        }
    }
    result
}
