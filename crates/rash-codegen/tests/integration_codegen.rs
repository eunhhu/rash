use std::path::PathBuf;

use rash_codegen::CodeGenerator;
use rash_spec::types::common::{Framework, Language};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
}

fn load_and_convert(fixture: &str) -> rash_ir::types::ProjectIR {
    let path = fixtures_dir().join(fixture);
    let (project, _report) = rash_spec::loader::load_project(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture '{}': {}", fixture, e));
    rash_ir::convert::convert_project(&project)
        .unwrap_or_else(|e| panic!("Failed to convert fixture '{}': {}", fixture, e))
}

#[test]
fn test_golden_typescript_express_codegen() {
    let ir = load_and_convert("golden-user-crud");

    let gen = CodeGenerator::new(Language::Typescript, Framework::Express).unwrap();
    let output = gen.generate(&ir).unwrap();

    // Should generate multiple files
    assert!(output.file_count() > 0, "Should generate at least one file");

    // Should have entrypoint
    assert!(
        output.files().contains_key("src/index.ts"),
        "Should have src/index.ts entrypoint"
    );

    // Should have routes
    assert!(
        output.files().contains_key("src/routes/index.ts"),
        "Should have src/routes/index.ts"
    );

    // Entrypoint should contain express setup
    let entrypoint = &output.files()["src/index.ts"];
    assert!(
        entrypoint.contains("express"),
        "Entrypoint should mention express"
    );
    assert!(
        entrypoint.contains("3000"),
        "Entrypoint should use port from config"
    );

    // Should have package.json
    assert!(
        output.files().contains_key("package.json"),
        "Should have package.json"
    );

    let package_json = &output.files()["package.json"];
    assert!(
        package_json.contains("golden-user-crud"),
        "package.json should have project name"
    );

    // Should have tsconfig.json
    assert!(
        output.files().contains_key("tsconfig.json"),
        "Should have tsconfig.json"
    );
}

#[test]
fn test_golden_codegen_writes_to_disk() {
    let ir = load_and_convert("golden-user-crud");

    let gen = CodeGenerator::new(Language::Typescript, Framework::Express).unwrap();
    let output = gen.generate(&ir).unwrap();

    let dir = tempfile::tempdir().unwrap();
    output.write_to_disk(dir.path()).unwrap();

    // Verify files exist on disk
    assert!(dir.path().join("src/index.ts").exists());
    assert!(dir.path().join("src/routes/index.ts").exists());
    assert!(dir.path().join("package.json").exists());
    assert!(dir.path().join("tsconfig.json").exists());
}

#[test]
fn test_minimal_fixture_codegen() {
    let ir = load_and_convert("minimal");

    let gen = CodeGenerator::new(Language::Typescript, Framework::Express).unwrap();
    let output = gen.generate(&ir).unwrap();

    assert!(output.file_count() > 0);
    assert!(output.files().contains_key("src/index.ts"));
    assert!(output.files().contains_key("src/routes/index.ts"));
}

#[test]
fn test_incompatible_language_framework() {
    // Express requires TypeScript, not Rust
    let result = CodeGenerator::new(Language::Rust, Framework::Express);
    assert!(result.is_err());

    // Actix requires Rust, not TypeScript
    let result = CodeGenerator::new(Language::Typescript, Framework::Actix);
    assert!(result.is_err());

    // FastAPI requires Python, not Go
    let result = CodeGenerator::new(Language::Go, Framework::FastAPI);
    assert!(result.is_err());

    // Gin requires Go, not Python
    let result = CodeGenerator::new(Language::Python, Framework::Gin);
    assert!(result.is_err());
}

#[test]
fn test_all_valid_language_framework_pairs() {
    // All compatible pairs should work
    assert!(CodeGenerator::new(Language::Typescript, Framework::Express).is_ok());
    assert!(CodeGenerator::new(Language::Rust, Framework::Actix).is_ok());
    assert!(CodeGenerator::new(Language::Python, Framework::FastAPI).is_ok());
    assert!(CodeGenerator::new(Language::Go, Framework::Gin).is_ok());
}

#[test]
fn test_empty_project_codegen() {
    let ir = rash_ir::types::ProjectIR {
        config: serde_json::json!({
            "name": "empty-project",
            "server": { "port": 3000 }
        }),
        routes: vec![],
        schemas: vec![],
        models: vec![],
        middleware: vec![],
        handlers: vec![],
    };

    let gen = CodeGenerator::new(Language::Typescript, Framework::Express).unwrap();
    let output = gen.generate(&ir).unwrap();

    // Even empty project should have entrypoint + routes + config files
    assert!(output.files().contains_key("src/index.ts"));
    assert!(output.files().contains_key("src/routes/index.ts"));
    assert!(output.files().contains_key("package.json"));
    assert!(output.files().contains_key("tsconfig.json"));
}
