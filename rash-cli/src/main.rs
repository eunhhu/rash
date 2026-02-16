use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "rash", about = "Rash – visual server application builder")]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new Rash project
    Init {
        /// Project name
        name: String,
        /// Target directory (defaults to ./<name>)
        #[arg(short, long)]
        dir: Option<PathBuf>,
        /// Target language
        #[arg(short, long, default_value = "typescript")]
        language: String,
        /// Target framework
        #[arg(short, long, default_value = "express")]
        framework: String,
        /// Target runtime
        #[arg(short, long, default_value = "bun")]
        runtime: String,
    },
    /// Validate a Rash project
    Validate {
        /// Project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Check project and show summary info
    Check {
        /// Project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Generate code from a Rash project
    Codegen {
        /// Project directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Output directory (defaults to .rash/generated)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init {
            name,
            dir,
            language,
            framework,
            runtime,
        } => cmd_init(&name, dir.as_deref(), &language, &framework, &runtime),
        Command::Validate { path } => cmd_validate(&path),
        Command::Check { path } => cmd_check(&path),
        Command::Codegen { path, output } => cmd_codegen(&path, output.as_deref()),
    };

    match result {
        Ok(success) => {
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("{} {e:#}", "error:".red().bold());
            ExitCode::from(1)
        }
    }
}

fn cmd_init(
    name: &str,
    dir: Option<&Path>,
    language: &str,
    framework: &str,
    runtime: &str,
) -> Result<bool> {
    let project_dir = match dir {
        Some(d) => d.to_path_buf(),
        None => {
            validate_default_dir_name(name)?;
            PathBuf::from(name)
        }
    };

    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", project_dir.display());
    }

    std::fs::create_dir_all(&project_dir)
        .with_context(|| format!("Failed to create directory '{}'", project_dir.display()))?;

    // Create subdirectories
    for sub in &["routes", "schemas", "models", "middleware", "handlers"] {
        std::fs::create_dir_all(project_dir.join(sub))?;
    }

    // Write rash.config.json
    let config = serde_json::json!({
        "$schema": "https://rash.dev/schemas/config.json",
        "version": "1.0.0",
        "name": name,
        "target": {
            "language": language,
            "framework": framework,
            "runtime": runtime
        },
        "server": {
            "port": 3000,
            "host": "0.0.0.0"
        }
    });

    let config_str = serde_json::to_string_pretty(&config)?;
    std::fs::write(project_dir.join("rash.config.json"), &config_str)?;

    // Write a health check route
    let health_route = serde_json::json!({
        "path": "/health",
        "methods": {
            "GET": {
                "operationId": "healthCheck",
                "handler": { "ref": "health.check" },
                "response": {
                    "200": {
                        "description": "OK"
                    }
                }
            }
        }
    });
    std::fs::write(
        project_dir.join("routes/health.route.json"),
        serde_json::to_string_pretty(&health_route)?,
    )?;

    // Write health handler
    let health_handler = serde_json::json!({
        "name": "health.check",
        "async": false,
        "body": [
            {
                "type": "ReturnStatement",
                "tier": 0,
                "value": {
                    "type": "HttpRespond",
                    "tier": 1,
                    "status": 200,
                    "body": {
                        "type": "ObjectExpr",
                        "tier": 0,
                        "properties": {
                            "status": { "type": "Literal", "tier": 0, "value": "ok" },
                            "timestamp": {
                                "type": "CallExpr",
                                "tier": 0,
                                "callee": {
                                    "type": "MemberExpr",
                                    "tier": 0,
                                    "object": { "type": "Identifier", "tier": 0, "name": "Date" },
                                    "property": "now"
                                },
                                "args": []
                            }
                        }
                    }
                }
            }
        ]
    });
    std::fs::write(
        project_dir.join("handlers/health.handler.json"),
        serde_json::to_string_pretty(&health_handler)?,
    )?;

    println!(
        "{} Created project '{}' at {}",
        "✓".green().bold(),
        name.bold(),
        project_dir.display()
    );
    println!("  {} rash.config.json", "→".dimmed());
    println!("  {} routes/health.route.json", "→".dimmed());
    println!("  {} handlers/health.handler.json", "→".dimmed());
    println!();
    println!("Next steps:");
    println!("  {} {}", "cd".dimmed(), project_dir.display());
    println!("  {} validate", "rash".dimmed());

    Ok(true)
}

fn validate_default_dir_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Project name must not be empty");
    }

    // Prevent common path traversal / accidental nested directory creation.
    // Only apply this when name is used as the default directory.
    if name.contains('/') || name.contains('\\') {
        anyhow::bail!("Project name must not contain path separators");
    }

    let mut components = Path::new(name).components();
    let Some(first) = components.next() else {
        anyhow::bail!("Project name must not be empty");
    };

    if components.next().is_some() {
        anyhow::bail!("Project name must be a single path component");
    }

    match first {
        std::path::Component::Normal(_) => Ok(()),
        _ => anyhow::bail!("Project name must be a normal directory name"),
    }
}

fn cmd_validate(path: &Path) -> Result<bool> {
    println!(
        "{} {}",
        "Validating".bold(),
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
    );

    let (project, load_report) = rash_spec::loader::load_project(path)
        .with_context(|| format!("Failed to load project at '{}'", path.display()))?;

    // Run validation
    let validation_report = rash_valid::validator::validate(&project);

    // Merge reports
    let total_errors: Vec<_> = load_report
        .errors
        .iter()
        .chain(validation_report.errors.iter())
        .collect();

    let error_count = total_errors
        .iter()
        .filter(|e| e.severity == rash_spec::types::common::Severity::Error)
        .count();
    let warning_count = total_errors
        .iter()
        .filter(|e| e.severity == rash_spec::types::common::Severity::Warning)
        .count();

    // Print errors and warnings
    for entry in &total_errors {
        let severity_str = match entry.severity {
            rash_spec::types::common::Severity::Error => "error".red().bold(),
            rash_spec::types::common::Severity::Warning => "warning".yellow().bold(),
            rash_spec::types::common::Severity::Info => "info".blue().bold(),
        };

        println!(
            "  {} [{}] {} ({}:{})",
            severity_str,
            entry.code.dimmed(),
            entry.message,
            entry.file.dimmed(),
            entry.path.dimmed(),
        );

        if let Some(suggestion) = &entry.suggestion {
            println!("    {} {}", "hint:".cyan(), suggestion);
        }
    }

    // Summary
    println!();
    if error_count == 0 && warning_count == 0 {
        println!(
            "{} Project is valid ({} routes, {} schemas, {} models, {} handlers)",
            "✓".green().bold(),
            project.routes.len(),
            project.schemas.len(),
            project.models.len(),
            project.handlers.len(),
        );
        Ok(true)
    } else if error_count == 0 {
        println!(
            "{} Valid with {} warning(s)",
            "⚠".yellow().bold(),
            warning_count,
        );
        Ok(true)
    } else {
        println!(
            "{} {} error(s), {} warning(s)",
            "✗".red().bold(),
            error_count,
            warning_count,
        );
        Ok(false)
    }
}

fn cmd_check(path: &Path) -> Result<bool> {
    let (project, load_report) = rash_spec::loader::load_project(path)
        .with_context(|| format!("Failed to load project at '{}'", path.display()))?;

    println!("{}", "Project Info".bold().underline());
    println!("  Name:      {}", project.config.name);
    println!("  Version:   {}", project.config.version);

    let lang = serde_json::to_value(project.config.target.language)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    let fw = serde_json::to_value(project.config.target.framework)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    let rt = serde_json::to_value(project.config.target.runtime)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    println!("  Target:    {} + {} ({})", lang, fw, rt);
    println!(
        "  Server:    {}:{}",
        project.config.server.host, project.config.server.port
    );

    println!();
    println!("{}", "Spec Files".bold().underline());
    println!("  Routes:     {}", project.routes.len());
    println!("  Schemas:    {}", project.schemas.len());
    println!("  Models:     {}", project.models.len());
    println!("  Middleware:  {}", project.middleware.len());
    println!("  Handlers:   {}", project.handlers.len());

    // Try IR conversion
    println!();
    match rash_ir::convert::convert_project(&project) {
        Ok(ir) => {
            let handler_tiers: Vec<_> = ir
                .handlers
                .iter()
                .map(|h| format!("{}(tier:{})", h.name, h.max_tier as u8))
                .collect();

            if !handler_tiers.is_empty() {
                println!("{}", "Handlers".bold().underline());
                for desc in &handler_tiers {
                    println!("  {}", desc);
                }
            }

            println!();
            println!("{} IR conversion successful", "✓".green().bold());
        }
        Err(e) => {
            println!("{} IR conversion failed: {e}", "✗".red().bold());
        }
    }

    // Validation summary
    let validation_report = rash_valid::validator::validate(&project);
    let error_count = load_report
        .errors
        .iter()
        .chain(validation_report.errors.iter())
        .filter(|e| e.severity == rash_spec::types::common::Severity::Error)
        .count();

    if error_count > 0 {
        println!(
            "{} {} validation error(s) found",
            "✗".red().bold(),
            error_count
        );
        Ok(false)
    } else {
        println!("{} No validation errors", "✓".green().bold());
        Ok(true)
    }
}

fn cmd_codegen(path: &Path, output: Option<&Path>) -> Result<bool> {
    println!(
        "{} {}",
        "Generating code from".bold(),
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
    );

    // 1. Load & validate
    let (project, load_report) = rash_spec::loader::load_project(path)
        .with_context(|| format!("Failed to load project at '{}'", path.display()))?;

    let validation_report = rash_valid::validator::validate(&project);
    let error_count = load_report
        .errors
        .iter()
        .chain(validation_report.errors.iter())
        .filter(|e| e.severity == rash_spec::types::common::Severity::Error)
        .count();

    if error_count > 0 {
        eprintln!(
            "{} {} validation error(s) — fix them before generating code",
            "✗".red().bold(),
            error_count
        );
        return Ok(false);
    }

    // 2. Convert to IR
    let ir = rash_ir::convert::convert_project(&project)
        .with_context(|| "Failed to convert spec to IR")?;

    // 3. Create code generator
    let language = project.config.target.language;
    let framework = project.config.target.framework;

    let generator = rash_codegen::CodeGenerator::new(language, framework)
        .map_err(|e| anyhow::anyhow!("Codegen error: {}", e))?;

    // 4. Generate
    let generated = generator
        .generate(&ir)
        .map_err(|e| anyhow::anyhow!("Generation failed: {}", e))?;

    // 5. Write output
    let output_dir = match output {
        Some(d) => d.to_path_buf(),
        None => path.join(".rash/generated"),
    };

    generated
        .write_to_disk(&output_dir)
        .with_context(|| format!("Failed to write to '{}'", output_dir.display()))?;

    println!(
        "{} Generated {} file(s) → {}",
        "✓".green().bold(),
        generated.file_count(),
        output_dir.display()
    );

    for file_path in generated.files().keys() {
        println!("  {} {}", "→".dimmed(), file_path);
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_rejects_parent_dir_name() {
        let result = cmd_init("..", None, "typescript", "express", "bun");
        assert!(result.is_err());
    }

    #[test]
    fn init_creates_minimal_project() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("my-app");

        cmd_init(
            "my-app",
            Some(project_dir.as_path()),
            "typescript",
            "express",
            "bun",
        )
        .unwrap();

        assert!(project_dir.join("rash.config.json").exists());
        assert!(project_dir.join("routes/health.route.json").exists());
        assert!(project_dir.join("handlers/health.handler.json").exists());
    }

    #[test]
    fn validate_minimal_fixture_ok() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("fixtures/minimal");

        let ok = cmd_validate(&fixture_path).unwrap();
        assert!(ok);
    }

    #[test]
    fn codegen_golden_fixture() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("fixtures/golden-user-crud");

        let tmp = TempDir::new().unwrap();
        let ok = cmd_codegen(&fixture_path, Some(tmp.path())).unwrap();
        assert!(ok);

        assert!(tmp.path().join("src/index.ts").exists());
        assert!(tmp.path().join("package.json").exists());
        assert!(tmp.path().join("tsconfig.json").exists());
    }

    #[test]
    fn codegen_minimal_fixture() {
        let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("fixtures/minimal");

        let tmp = TempDir::new().unwrap();
        let ok = cmd_codegen(&fixture_path, Some(tmp.path())).unwrap();
        assert!(ok);

        assert!(tmp.path().join("src/index.ts").exists());
    }
}
