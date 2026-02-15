use std::collections::HashMap;

use crate::types::error::{ErrorEntry, E_DUPLICATE_SYMBOL};

/// Symbol types in the spec
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Schema,
    Handler,
    Middleware,
    Model,
    Route,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Schema => write!(f, "schema"),
            SymbolKind::Handler => write!(f, "handler"),
            SymbolKind::Middleware => write!(f, "middleware"),
            SymbolKind::Model => write!(f, "model"),
            SymbolKind::Route => write!(f, "route"),
        }
    }
}

/// Entry in the symbol table
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    /// The canonical key
    pub canonical_key: String,
    /// Original name as written in the spec
    pub original_name: String,
    /// Kind of symbol
    pub kind: SymbolKind,
    /// File where this symbol is defined
    pub file: String,
    /// JSONPath within the file
    pub path: String,
}

/// Index of all symbols in a loaded project.
/// Uses canonical (lowercased) keys to detect duplicates and resolve references.
#[derive(Debug, Clone)]
pub struct SpecIndex {
    /// Main symbol table: canonical_key -> entries (multiple = duplicate)
    symbols: HashMap<String, Vec<SymbolEntry>>,
    /// Symbols grouped by kind for scoped lookup
    by_kind: HashMap<SymbolKind, HashMap<String, SymbolEntry>>,
}

impl SpecIndex {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            by_kind: HashMap::new(),
        }
    }

    /// Register a symbol. Returns an error if it's a duplicate within the same kind.
    pub fn register(
        &mut self,
        name: &str,
        kind: SymbolKind,
        file: &str,
        path: &str,
    ) -> Option<ErrorEntry> {
        let canonical_any = canonicalize_any(name);
        let canonical_kind = canonicalize_for_kind(name, kind);

        let entry = SymbolEntry {
            canonical_key: canonical_kind.clone(),
            original_name: name.to_string(),
            kind,
            file: file.to_string(),
            path: path.to_string(),
        };

        // Check for duplicate within same kind
        let kind_map = self.by_kind.entry(kind).or_default();
        if let Some(existing) = kind_map.get(&canonical_kind) {
            let err = ErrorEntry::error(
                E_DUPLICATE_SYMBOL,
                format!(
                    "Duplicate {kind} symbol '{name}' (also defined in {})",
                    existing.file
                ),
                file,
                path,
            )
            .with_suggestion(format!(
                "Rename one of the '{}' definitions to avoid conflict",
                name
            ));
            // Still register for tracking
            self.symbols.entry(canonical_any).or_default().push(entry);
            return Some(err);
        }

        kind_map.insert(canonical_kind, entry.clone());
        self.symbols.entry(canonical_any).or_default().push(entry);
        None
    }

    /// Look up a symbol by name and kind
    pub fn lookup(&self, name: &str, kind: SymbolKind) -> Option<&SymbolEntry> {
        let canonical_kind = canonicalize_for_kind(name, kind);
        self.by_kind.get(&kind)?.get(&canonical_kind)
    }

    /// Look up a symbol by name across all kinds
    pub fn lookup_any(&self, name: &str) -> Vec<&SymbolEntry> {
        let canonical_any = canonicalize_any(name);
        self.symbols
            .get(&canonical_any)
            .map(|entries| entries.iter().collect())
            .unwrap_or_default()
    }

    /// Get all symbols of a specific kind
    pub fn symbols_of_kind(&self, kind: SymbolKind) -> Vec<&SymbolEntry> {
        self.by_kind
            .get(&kind)
            .map(|m| m.values().collect())
            .unwrap_or_default()
    }

    /// Total number of registered symbols
    pub fn len(&self) -> usize {
        self.by_kind.values().map(|m| m.len()).sum()
    }

    /// Whether the index is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for SpecIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a SpecIndex from a loaded project
pub fn build_index(project: &crate::loader::LoadedProject) -> (SpecIndex, Vec<ErrorEntry>) {
    let mut index = SpecIndex::new();
    let mut errors = Vec::new();

    // Register schemas (each definition is a separate symbol)
    for (file, schema) in &project.schemas {
        for def_name in schema.definitions.keys() {
            if let Some(err) = index.register(
                def_name,
                SymbolKind::Schema,
                file,
                &format!("$.definitions.{def_name}"),
            ) {
                errors.push(err);
            }
        }
    }

    // Register handlers
    for (file, handler) in &project.handlers {
        if let Some(err) = index.register(&handler.name, SymbolKind::Handler, file, "$.name") {
            errors.push(err);
        }
    }

    // Register middleware
    for (file, mw) in &project.middleware {
        if let Some(err) = index.register(&mw.name, SymbolKind::Middleware, file, "$.name") {
            errors.push(err);
        }
    }

    // Register models
    for (file, model) in &project.models {
        if let Some(err) = index.register(&model.name, SymbolKind::Model, file, "$.name") {
            errors.push(err);
        }
    }

    // Register routes
    for (file, route) in &project.routes {
        if let Some(err) = index.register(&route.path, SymbolKind::Route, file, "$.path") {
            errors.push(err);
        }
    }

    (index, errors)
}

/// Normalize a name to its canonical form for comparison.
/// Route paths are not lowercased.
fn canonicalize_for_kind(name: &str, kind: SymbolKind) -> String {
    match kind {
        SymbolKind::Route => canonicalize_route(name),
        SymbolKind::Schema | SymbolKind::Handler | SymbolKind::Middleware | SymbolKind::Model => {
            canonicalize_ident(name)
        }
    }
}

fn canonicalize_any(name: &str) -> String {
    canonicalize_ident(name)
}

fn canonicalize_route(path: &str) -> String {
    path.trim().to_string()
}

fn canonicalize_ident(name: &str) -> String {
    let trimmed = name.trim().to_lowercase();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            '.' => out.push(ch),
            '-' | '_' | ' ' | '\t' | '\n' | '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup() {
        let mut index = SpecIndex::new();
        assert!(index
            .register(
                "UserResponse",
                SymbolKind::Schema,
                "user.schema.json",
                "$.definitions.UserResponse"
            )
            .is_none());

        let entry = index.lookup("UserResponse", SymbolKind::Schema).unwrap();
        assert_eq!(entry.original_name, "UserResponse");
        assert_eq!(entry.file, "user.schema.json");
    }

    #[test]
    fn test_canonical_key_case_insensitive() {
        let mut index = SpecIndex::new();
        index.register("UserResponse", SymbolKind::Schema, "a.json", "$");

        // Lookup with different case should work
        assert!(index.lookup("userresponse", SymbolKind::Schema).is_some());
        assert!(index.lookup("USERRESPONSE", SymbolKind::Schema).is_some());

        assert!(index.lookup("user_response", SymbolKind::Schema).is_some());
        assert!(index.lookup("user-response", SymbolKind::Schema).is_some());
    }

    #[test]
    fn test_duplicate_detection() {
        let mut index = SpecIndex::new();
        assert!(index
            .register("User", SymbolKind::Schema, "a.json", "$")
            .is_none());

        let err = index
            .register("User", SymbolKind::Schema, "b.json", "$")
            .unwrap();
        assert_eq!(err.code, E_DUPLICATE_SYMBOL);
        assert!(err.message.contains("Duplicate"));
    }

    #[test]
    fn test_same_name_different_kinds_ok() {
        let mut index = SpecIndex::new();
        assert!(index
            .register("User", SymbolKind::Schema, "schema.json", "$")
            .is_none());
        assert!(index
            .register("User", SymbolKind::Model, "model.json", "$")
            .is_none());

        // Both should be found
        assert!(index.lookup("User", SymbolKind::Schema).is_some());
        assert!(index.lookup("User", SymbolKind::Model).is_some());
    }

    #[test]
    fn test_lookup_any() {
        let mut index = SpecIndex::new();
        index.register("User", SymbolKind::Schema, "a.json", "$");
        index.register("User", SymbolKind::Model, "b.json", "$");

        let entries = index.lookup_any("User");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_symbols_of_kind() {
        let mut index = SpecIndex::new();
        index.register("auth", SymbolKind::Middleware, "a.json", "$");
        index.register("cors", SymbolKind::Middleware, "b.json", "$");
        index.register("User", SymbolKind::Schema, "c.json", "$");

        assert_eq!(index.symbols_of_kind(SymbolKind::Middleware).len(), 2);
        assert_eq!(index.symbols_of_kind(SymbolKind::Schema).len(), 1);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut index = SpecIndex::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);

        index.register("User", SymbolKind::Schema, "a.json", "$");
        assert!(!index.is_empty());
        assert_eq!(index.len(), 1);
    }
}
