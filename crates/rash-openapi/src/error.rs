use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenApiError {
    #[error("invalid route path `{0}`: {1}")]
    InvalidPath(String, String),

    #[error("missing schema reference `{0}`")]
    MissingSchema(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("unsupported OpenAPI version: {0}")]
    UnsupportedVersion(String),
}

#[derive(Debug, Error)]
pub enum ReverseParseError {
    #[error("unsupported framework in `{0}`")]
    UnsupportedFramework(String),

    #[error("parse error: {0}")]
    ParseError(String),
}
