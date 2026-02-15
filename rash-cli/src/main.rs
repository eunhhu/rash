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
    let project_dir = dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(name));

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
