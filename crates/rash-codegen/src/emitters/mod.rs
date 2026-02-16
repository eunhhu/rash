pub mod typescript;
pub mod rust_lang;
pub mod python;
pub mod go_lang;

use rash_spec::types::common::Language;

use crate::error::CodegenError;
use crate::traits::LanguageEmitter;

/// Create the appropriate language emitter for the given language.
pub fn create_emitter(language: Language) -> Result<Box<dyn LanguageEmitter>, CodegenError> {
    match language {
        Language::Typescript => Ok(Box::new(typescript::TypeScriptEmitter)),
        Language::Rust => Ok(Box::new(rust_lang::RustEmitter)),
        Language::Python => Ok(Box::new(python::PythonEmitter)),
        Language::Go => Ok(Box::new(go_lang::GoEmitter)),
    }
}
