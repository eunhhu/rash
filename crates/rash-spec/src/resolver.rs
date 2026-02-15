use crate::index::{SpecIndex, SymbolEntry, SymbolKind};
use crate::types::error::E_REF_EXTERNAL_UNSUPPORTED;
use crate::types::error::{ErrorEntry, E_REF_AMBIGUOUS, E_REF_NOT_FOUND, E_REF_TYPE_MISMATCH};

/// Context in which a reference appears, used to determine expected symbol kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefContext {
    /// Reference in handler field → expects Handler
    Handler,
    /// Reference in schema/response field → expects Schema
    Schema,
    /// Reference in middleware field → expects Middleware
    Middleware,
    /// Reference in model field → expects Model
    Model,
}

impl RefContext {
    /// Convert to the expected SymbolKind
    pub fn expected_kind(&self) -> SymbolKind {
        match self {
            RefContext::Handler => SymbolKind::Handler,
            RefContext::Schema => SymbolKind::Schema,
            RefContext::Middleware => SymbolKind::Middleware,
            RefContext::Model => SymbolKind::Model,
        }
    }
}

/// Result of resolving a reference
#[derive(Debug, Clone)]
pub enum ResolveResult<'a> {
    /// Successfully resolved to a single symbol
    Found(&'a SymbolEntry),
    /// Reference not found
    NotFound,
    /// Multiple candidates (ambiguous)
    Ambiguous(Vec<&'a SymbolEntry>),
    /// Found but wrong type
    TypeMismatch {
        expected: SymbolKind,
        found: &'a SymbolEntry,
    },
    /// External reference (file#definition) — not resolved locally
    External { file: String, definition: String },
}

/// Deterministic reference resolver.
///
/// 5 rules:
/// 1. Type discrimination: determine target kind from field context
/// 2. Local first: search project's SpecIndex for the expected kind
/// 3. Canonical key: case/separator normalized lookup
/// 4. External separation: `file#definition` parsed separately
/// 5. Ambiguity prohibition: 2+ candidates → error (no auto-selection)
pub struct Resolver<'a> {
    index: &'a SpecIndex,
}

impl<'a> Resolver<'a> {
    pub fn new(index: &'a SpecIndex) -> Self {
        Self { index }
    }

    /// Resolve a reference string in a given context.
    ///
    /// For handler refs like "users.getUser", the full string is used as the name.
    /// For external refs like "common.schema#Pagination", splits on '#'.
    pub fn resolve(&self, ref_str: &str, context: RefContext) -> ResolveResult<'a> {
        // Rule 4: External reference separation
        if let Some((file, definition)) = ref_str.split_once('#') {
            return ResolveResult::External {
                file: file.to_string(),
                definition: definition.to_string(),
            };
        }

        let expected_kind = context.expected_kind();

        // Rule 1 + 2 + 3: Look up by expected kind using canonical key
        if let Some(entry) = self.index.lookup(ref_str, expected_kind) {
            return ResolveResult::Found(entry);
        }

        // Rule 5: Check if the name exists under a different kind
        let all_matches = self.index.lookup_any(ref_str);
        if all_matches.is_empty() {
            return ResolveResult::NotFound;
        }

        // Found under different kind(s)
        if all_matches.len() == 1 {
            return ResolveResult::TypeMismatch {
                expected: expected_kind,
                found: all_matches[0],
            };
        }

        // Multiple matches across kinds — ambiguous
        ResolveResult::Ambiguous(all_matches)
    }

    /// Resolve a reference and convert to an ErrorEntry if failed.
    pub fn resolve_or_error(
        &self,
        ref_str: &str,
        context: RefContext,
        file: &str,
        path: &str,
    ) -> Result<&'a SymbolEntry, ErrorEntry> {
        match self.resolve(ref_str, context) {
            ResolveResult::Found(entry) => Ok(entry),
            ResolveResult::NotFound => Err(ErrorEntry::error(
                E_REF_NOT_FOUND,
                format!(
                    "Referenced {} '{}' was not found",
                    context.expected_kind(),
                    ref_str
                ),
                file,
                path,
            )
            .with_suggestion(format!(
                "Create {} '{}' or fix the ref name",
                context.expected_kind(),
                ref_str
            ))),
            ResolveResult::TypeMismatch { expected, found } => Err(ErrorEntry::error(
                E_REF_TYPE_MISMATCH,
                format!(
                    "Reference '{}' expected {} but found {} (defined in {})",
                    ref_str, expected, found.kind, found.file
                ),
                file,
                path,
            )
            .with_suggestion(format!(
                "Check that '{}' is a {} definition, not a {}",
                ref_str, expected, found.kind
            ))),
            ResolveResult::Ambiguous(entries) => {
                let locations: Vec<String> = entries
                    .iter()
                    .map(|e| format!("{} in {}", e.kind, e.file))
                    .collect();
                Err(ErrorEntry::error(
                    E_REF_AMBIGUOUS,
                    format!(
                        "Reference '{}' is ambiguous: found as {}",
                        ref_str,
                        locations.join(", ")
                    ),
                    file,
                    path,
                )
                .with_suggestion("Use a more specific reference or rename conflicting definitions"))
            }
            ResolveResult::External { .. } => {
                Err(ErrorEntry::error(
                    E_REF_EXTERNAL_UNSUPPORTED,
                    format!(
                        "External reference '{}' cannot be resolved locally",
                        ref_str
                    ),
                    file,
                    path,
                )
                .with_suggestion(
                    "Phase 1 does not support external refs. Inline the definition or rename it into this project.",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SpecIndex;

    fn setup_index() -> SpecIndex {
        let mut index = SpecIndex::new();
        index.register(
            "UserResponse",
            SymbolKind::Schema,
            "schemas/user.schema.json",
            "$.definitions.UserResponse",
        );
        index.register(
            "CreateUserBody",
            SymbolKind::Schema,
            "schemas/user.schema.json",
            "$.definitions.CreateUserBody",
        );
        index.register(
            "getUser",
            SymbolKind::Handler,
            "handlers/users.handler.json",
            "$.name",
        );
        index.register(
            "auth",
            SymbolKind::Middleware,
            "middleware/auth.middleware.json",
            "$.name",
        );
        index.register(
            "User",
            SymbolKind::Model,
            "models/user.model.json",
            "$.name",
        );
        index
    }

    #[test]
    fn test_resolve_schema_ref() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        match resolver.resolve("UserResponse", RefContext::Schema) {
            ResolveResult::Found(entry) => {
                assert_eq!(entry.original_name, "UserResponse");
                assert_eq!(entry.kind, SymbolKind::Schema);
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_handler_ref() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        match resolver.resolve("getUser", RefContext::Handler) {
            ResolveResult::Found(entry) => {
                assert_eq!(entry.original_name, "getUser");
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_not_found() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        match resolver.resolve("NonExistent", RefContext::Schema) {
            ResolveResult::NotFound => {}
            other => panic!("Expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_type_mismatch() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        // "auth" is a Middleware, not a Schema
        match resolver.resolve("auth", RefContext::Schema) {
            ResolveResult::TypeMismatch { expected, found } => {
                assert_eq!(expected, SymbolKind::Schema);
                assert_eq!(found.kind, SymbolKind::Middleware);
            }
            other => panic!("Expected TypeMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_external_ref() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        match resolver.resolve("common.schema#Pagination", RefContext::Schema) {
            ResolveResult::External { file, definition } => {
                assert_eq!(file, "common.schema");
                assert_eq!(definition, "Pagination");
            }
            other => panic!("Expected External, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_case_insensitive() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        // Should find "UserResponse" even with different case
        match resolver.resolve("userresponse", RefContext::Schema) {
            ResolveResult::Found(entry) => {
                assert_eq!(entry.original_name, "UserResponse");
            }
            other => panic!("Expected Found, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_or_error_not_found() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        let err = resolver
            .resolve_or_error(
                "NonExistent",
                RefContext::Schema,
                "routes/test.route.json",
                "$.methods.GET.response.200.schema.ref",
            )
            .unwrap_err();

        assert_eq!(err.code, E_REF_NOT_FOUND);
        assert!(err.suggestion.is_some());
    }

    #[test]
    fn test_resolve_or_error_type_mismatch() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        let err = resolver
            .resolve_or_error(
                "auth",
                RefContext::Schema,
                "routes/test.route.json",
                "$.methods.GET.response.200.schema.ref",
            )
            .unwrap_err();

        assert_eq!(err.code, E_REF_TYPE_MISMATCH);
    }

    #[test]
    fn test_determinism_100_runs() {
        let index = setup_index();
        let resolver = Resolver::new(&index);

        let mut results = Vec::new();
        for _ in 0..100 {
            let result = match resolver.resolve("UserResponse", RefContext::Schema) {
                ResolveResult::Found(entry) => entry.original_name.clone(),
                _ => "FAIL".to_string(),
            };
            results.push(result);
        }

        // All 100 results must be identical
        assert!(results.iter().all(|r| r == "UserResponse"));
    }
}
