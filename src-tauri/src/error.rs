use serde::Serialize;

/// Application error type for IPC commands
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No project is currently open")]
    NoProject,

    #[error("Failed to load project: {0}")]
    LoadError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid spec: {0}")]
    InvalidSpec(String),

    #[error("Codegen error: {0}")]
    CodegenError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InvalidSpec(err.to_string())
    }
}

impl From<rash_spec::loader::LoadError> for AppError {
    fn from(err: rash_spec::loader::LoadError) -> Self {
        AppError::LoadError(err.to_string())
    }
}

impl From<rash_codegen::CodegenError> for AppError {
    fn from(err: rash_codegen::CodegenError) -> Self {
        AppError::CodegenError(err.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
