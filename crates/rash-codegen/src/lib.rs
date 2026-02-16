pub mod context;
pub mod error;
pub mod generator;
pub mod traits;

// Language emitters
pub mod emitters;

// Framework adapters
pub mod adapters;

// Re-exports
pub use context::EmitContext;
pub use error::CodegenError;
pub use generator::{CodeGenerator, GeneratedProject};
pub use traits::{FrameworkAdapter, LanguageEmitter};
