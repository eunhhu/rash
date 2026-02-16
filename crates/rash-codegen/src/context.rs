use indexmap::IndexSet;

/// Tracks state during code emission: indentation, collected imports, etc.
#[derive(Debug, Clone)]
pub struct EmitContext {
    /// Current indentation level
    indent_level: usize,
    /// Characters per indent (e.g., 2 spaces)
    indent_width: usize,
    /// Whether to use tabs
    use_tabs: bool,
    /// Collected import paths (deduped, insertion-ordered)
    imports: IndexSet<ImportIR>,
}

/// Represents a single import statement to be collected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportIR {
    /// What to import (e.g., "express", "{ Request, Response }")
    pub names: String,
    /// Module path (e.g., "express", "./routes")
    pub from: String,
}

/// Indentation style configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    Spaces(usize),
    Tabs,
}

impl EmitContext {
    pub fn new(style: IndentStyle) -> Self {
        let (use_tabs, indent_width) = match style {
            IndentStyle::Spaces(n) => (false, n),
            IndentStyle::Tabs => (true, 1),
        };
        Self {
            indent_level: 0,
            indent_width,
            use_tabs,
            imports: IndexSet::new(),
        }
    }

    /// Get the current indentation string.
    pub fn indent(&self) -> String {
        let unit = if self.use_tabs { "\t" } else { " " };
        unit.repeat(self.indent_level * self.indent_width)
    }

    /// Increase indentation by one level.
    pub fn push_indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation by one level.
    pub fn pop_indent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    /// Add an import to the collection (deduped).
    pub fn add_import(&mut self, names: impl Into<String>, from: impl Into<String>) {
        self.imports.insert(ImportIR {
            names: names.into(),
            from: from.into(),
        });
    }

    /// Get all collected imports.
    pub fn imports(&self) -> &IndexSet<ImportIR> {
        &self.imports
    }

    /// Drain and return all collected imports.
    pub fn take_imports(&mut self) -> IndexSet<ImportIR> {
        std::mem::take(&mut self.imports)
    }

    /// Current indent level.
    pub fn indent_level(&self) -> usize {
        self.indent_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent_spaces() {
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        assert_eq!(ctx.indent(), "");
        ctx.push_indent();
        assert_eq!(ctx.indent(), "  ");
        ctx.push_indent();
        assert_eq!(ctx.indent(), "    ");
        ctx.pop_indent();
        assert_eq!(ctx.indent(), "  ");
    }

    #[test]
    fn test_indent_tabs() {
        let mut ctx = EmitContext::new(IndentStyle::Tabs);
        ctx.push_indent();
        assert_eq!(ctx.indent(), "\t");
        ctx.push_indent();
        assert_eq!(ctx.indent(), "\t\t");
    }

    #[test]
    fn test_imports_dedup() {
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        ctx.add_import("express", "express");
        ctx.add_import("express", "express");
        ctx.add_import("{ z }", "zod");
        assert_eq!(ctx.imports().len(), 2);
    }

    #[test]
    fn test_take_imports() {
        let mut ctx = EmitContext::new(IndentStyle::Spaces(2));
        ctx.add_import("express", "express");
        let imports = ctx.take_imports();
        assert_eq!(imports.len(), 1);
        assert!(ctx.imports().is_empty());
    }
}
