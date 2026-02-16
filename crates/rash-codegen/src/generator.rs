use std::collections::BTreeMap;
use std::path::Path;

use rash_spec::types::common::{Framework, Language};

use crate::adapters;
use crate::context::EmitContext;
use crate::emitters;
use crate::error::CodegenError;
use crate::traits::{FrameworkAdapter, LanguageEmitter};
use rash_ir::types::ProjectIR;

/// Normalize a dotted name to a valid identifier for the target language.
/// e.g., "health.check" → "healthCheck" (camelCase)
pub fn normalize_identifier(name: &str) -> String {
    if !name.contains('.') {
        return name.to_string();
    }
    let parts: Vec<&str> = name.split('.').collect();
    let mut result = parts[0].to_string();
    for part in &parts[1..] {
        if let Some(first) = part.chars().next() {
            result.push_str(&first.to_uppercase().to_string());
            result.push_str(&part[first.len_utf8()..]);
        }
    }
    result
}

/// Normalize a name to a safe file name (replace dots with underscores).
pub fn normalize_filename(name: &str) -> String {
    name.replace('.', "_")
}

/// A collection of generated files, keyed by relative path.
#[derive(Debug, Clone, Default)]
pub struct GeneratedProject {
    /// Files keyed by relative path (sorted for deterministic output)
    files: BTreeMap<String, String>,
}

impl GeneratedProject {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file to the generated project.
    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    /// Get all generated files.
    pub fn files(&self) -> &BTreeMap<String, String> {
        &self.files
    }

    /// Number of generated files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Write all generated files to the given output directory.
    pub fn write_to_disk(&self, output_dir: &Path) -> Result<(), std::io::Error> {
        for (rel_path, content) in &self.files {
            let full_path = output_dir.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, content)?;
        }
        Ok(())
    }
}

/// The main code generator that combines a language emitter
/// with a framework adapter to produce a complete project.
pub struct CodeGenerator {
    emitter: Box<dyn LanguageEmitter>,
    adapter: Box<dyn FrameworkAdapter>,
}

impl std::fmt::Debug for CodeGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeGenerator")
            .field("language", &self.emitter.language())
            .field("framework", &self.adapter.framework())
            .finish()
    }
}

impl CodeGenerator {
    /// Create a new code generator for the given language/framework combination.
    pub fn new(language: Language, framework: Framework) -> Result<Self, CodegenError> {
        let emitter = emitters::create_emitter(language)?;
        let adapter = adapters::create_adapter(framework)?;

        if adapter.compatible_language() != language {
            return Err(CodegenError::IncompatibleTarget { language, framework });
        }

        Ok(Self { emitter, adapter })
    }

    /// Generate a complete project from the given IR.
    pub fn generate(&self, project: &ProjectIR) -> Result<GeneratedProject, CodegenError> {
        let mut output = GeneratedProject::new();
        let ext = self.emitter.file_extension();

        // 1. Generate schemas (DTOs)
        for schema in &project.schemas {
            let mut ctx = self.new_context();
            let code = self.emitter.emit_schema(schema, &mut ctx);
            let imports = self.emitter.emit_imports(&mut ctx);
            let full = if imports.is_empty() {
                code
            } else {
                format!("{}\n\n{}", imports, code)
            };
            let path = format!("src/schemas/{}.{}", schema.name.to_lowercase(), ext);
            output.add_file(path, full);
        }

        // 2. Generate models (ORM)
        for model in &project.models {
            let mut ctx = self.new_context();
            let code = self.emitter.emit_model(model, &mut ctx);
            let imports = self.emitter.emit_imports(&mut ctx);
            let full = if imports.is_empty() {
                code
            } else {
                format!("{}\n\n{}", imports, code)
            };
            let path = format!("src/models/{}.{}", model.name.to_lowercase(), ext);
            output.add_file(path, full);
        }

        // 3. Generate middleware (normalize dotted names)
        for mw in &project.middleware {
            let mut ctx = self.new_context();
            let mut normalized = mw.clone();
            normalized.name = normalize_identifier(&mw.name);
            let code = self
                .adapter
                .emit_middleware_def(&normalized, self.emitter.as_ref(), &mut ctx);
            let imports = self.emitter.emit_imports(&mut ctx);
            let full = if imports.is_empty() {
                code
            } else {
                format!("{}\n\n{}", imports, code)
            };
            let filename = normalize_filename(&mw.name);
            let path = format!("src/middleware/{}.{}", filename, ext);
            output.add_file(path, full);
        }

        // 4. Generate handlers (normalize dotted names)
        for handler in &project.handlers {
            let mut ctx = self.new_context();
            let mut normalized = handler.clone();
            normalized.name = normalize_identifier(&handler.name);
            let code = self
                .adapter
                .emit_handler(&normalized, self.emitter.as_ref(), &mut ctx);
            let imports = self.emitter.emit_imports(&mut ctx);
            let full = if imports.is_empty() {
                code
            } else {
                format!("{}\n\n{}", imports, code)
            };
            let filename = normalize_filename(&handler.name);
            let path = format!("src/handlers/{}.{}", filename, ext);
            output.add_file(path, full);
        }

        // 5. Generate route registration
        {
            let mut ctx = self.new_context();
            let mut route_blocks = Vec::new();
            for route in &project.routes {
                // Normalize path params for the target framework
                let mut normalized_route = route.clone();
                normalized_route.path = self.adapter.normalize_path(&route.path);
                let block = self.adapter.emit_route_registration(
                    &normalized_route,
                    self.emitter.as_ref(),
                    &mut ctx,
                );
                route_blocks.push(block);
            }
            let imports = self.emitter.emit_imports(&mut ctx);
            let route_code = route_blocks.join("\n\n");
            let full = self.adapter.wrap_route_file(&imports, &route_code, &mut ctx);
            let path = format!("src/routes/index.{}", ext);
            output.add_file(path, full);
        }

        // 6. Generate entrypoint (with global middleware)
        {
            let mut ctx = self.new_context();
            let mut entry = self.adapter.emit_entrypoint(project, &mut ctx);

            // Extract global middleware refs from config
            let global_mw_refs: Vec<String> = project
                .config
                .get("middleware")
                .and_then(|m| m.get("global"))
                .and_then(|g| g.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            v.get("ref")
                                .and_then(|r| r.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default();

            if !global_mw_refs.is_empty() {
                let mw_lines = self.adapter.emit_global_middleware(&global_mw_refs, &mut ctx);
                // Insert global middleware imports and app.use() lines
                // after the app creation line (heuristic: insert before route registration)
                let mw_block = mw_lines.join("\n");
                // Generate import lines for each middleware
                let mw_imports: Vec<String> = global_mw_refs
                    .iter()
                    .map(|mw_ref| {
                        let mw_id = normalize_identifier(mw_ref);
                        let mw_file = normalize_filename(mw_ref);
                        format!(
                            "import {{ {} }} from \"./middleware/{}\";",
                            mw_id, mw_file
                        )
                    })
                    .collect();
                let import_block = mw_imports.join("\n");

                // Insert after the last import line in entrypoint
                if let Some(last_import_pos) = entry.rfind("\nimport ") {
                    let insert_pos = entry[last_import_pos + 1..]
                        .find('\n')
                        .map(|p| last_import_pos + 1 + p + 1)
                        .unwrap_or(entry.len());
                    entry.insert_str(insert_pos, &format!("{}\n", import_block));
                }

                // Insert middleware application before route registration
                if let Some(route_pos) = entry.find("registerRoutes")
                    .or_else(|| entry.find("routes::configure"))
                    .or_else(|| entry.find("include_router"))
                    .or_else(|| entry.find("registerRoutes(r)"))
                {
                    // Find the start of the line
                    let line_start = entry[..route_pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
                    entry.insert_str(line_start, &format!("{}\n\n", mw_block));
                }
            }

            let path = format!("src/index.{}", ext);
            output.add_file(path, entry);
        }

        // 7. Generate project config files
        for (path, content) in self.adapter.emit_project_config(project) {
            output.add_file(path, content);
        }

        Ok(output)
    }

    fn new_context(&self) -> EmitContext {
        EmitContext::new(self.emitter.indent_style())
    }

    /// Access the language emitter.
    pub fn emitter(&self) -> &dyn LanguageEmitter {
        self.emitter.as_ref()
    }

    /// Access the framework adapter.
    pub fn adapter(&self) -> &dyn FrameworkAdapter {
        self.adapter.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_project_basics() {
        let mut proj = GeneratedProject::new();
        proj.add_file("src/index.ts", "console.log('hello');");
        proj.add_file("package.json", "{}");
        assert_eq!(proj.file_count(), 2);
        assert!(proj.files().contains_key("src/index.ts"));
    }

    #[test]
    fn test_generated_project_write_to_disk() {
        let mut proj = GeneratedProject::new();
        proj.add_file("src/index.ts", "console.log('hello');");
        proj.add_file("package.json", "{}");

        let dir = tempfile::tempdir().unwrap();
        proj.write_to_disk(dir.path()).unwrap();

        assert!(dir.path().join("src/index.ts").exists());
        assert!(dir.path().join("package.json").exists());

        let content = std::fs::read_to_string(dir.path().join("src/index.ts")).unwrap();
        assert_eq!(content, "console.log('hello');");
    }

    #[test]
    fn test_incompatible_target() {
        // Express requires TypeScript, not Rust
        let result = CodeGenerator::new(Language::Rust, Framework::Express);
        assert!(result.is_err());
        match result.unwrap_err() {
            CodegenError::IncompatibleTarget { language, framework } => {
                assert_eq!(language, Language::Rust);
                assert_eq!(framework, Framework::Express);
            }
            e => panic!("Expected IncompatibleTarget, got: {:?}", e),
        }
    }

    #[test]
    fn test_valid_target_creation() {
        let gen = CodeGenerator::new(Language::Typescript, Framework::Express);
        assert!(gen.is_ok());
    }
}
