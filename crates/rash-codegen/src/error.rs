use rash_spec::types::common::{Framework, Language};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("incompatible target: {language:?} is not compatible with {framework:?}")]
    IncompatibleTarget {
        language: Language,
        framework: Framework,
    },

    #[error("unsupported language: {0:?}")]
    UnsupportedLanguage(Language),

    #[error("unsupported framework: {0:?}")]
    UnsupportedFramework(Framework),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("codegen error: {0}")]
    Other(String),
}
