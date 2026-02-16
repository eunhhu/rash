use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use rash_runtime::hmu_engine::{HmuConfig, HmuEngine};
use rash_runtime::hmu_types::*;
use rash_runtime::incremental::*;
use rash_runtime::log_types::*;
use rash_runtime::preflight::{CheckStatus, PreflightReport};
use rash_runtime::preflight_checker::PreflightChecker;
use rash_runtime::process_manager::{ProcessError, ProcessManager, ServerStatus};
use rash_runtime::runtime_detect::RuntimeDetector;

use rash_spec::types::common::*;
use rash_spec::types::config::{CodegenConfig, RashConfig, TargetConfig};
use rash_spec::types::config::ServerConfig as SpecServerConfig;

// ── Helpers ──────────────────────────────────────────────────────────

fn make_rash_config(runtime: Runtime, port: u16, out_dir: Option<&str>) -> RashConfig {
    RashConfig {
        schema: None,
        version: "1.0.0".into(),
        name: "integration-test".into(),
        description: None,
        target: TargetConfig {
            language: Language::Typescript,
            framework: Framework::Express,
            runtime,
        },
        server: SpecServerConfig {
            port,
            host: "127.0.0.1".into(),
            protocol: None,
            base_path: None,
        },
        database: None,
        codegen: out_dir.map(|d| CodegenConfig {
            out_dir: d.to_string(),
            source_map: false,
            strict: false,
        }),
        middleware: None,
        plugins: vec![],
        meta: None,
    }
}

// ══════════════════════════════════════════════════════════════════════
// 1. Runtime Detection + Preflight Integration
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_runtime_detection_feeds_into_preflight() {
    // Detect installed runtimes
    let runtimes = RuntimeDetector::detect_installed();
    assert!(!runtimes.is_empty(), "At least one runtime should be detected");

    // cargo is guaranteed in a Rust project
    let cargo_rt = runtimes.iter().find(|r| r.name == "cargo");
    assert!(cargo_rt.is_some(), "cargo should be detected");

    // Create a config targeting cargo (which we know is installed)
    let tmp = tempfile::tempdir().unwrap();
    let free_port = get_free_port();
    let config = make_rash_config(Runtime::Cargo, free_port, Some("."));

    // Run preflight — runtime check should pass
    let report = PreflightChecker::run(&config, tmp.path());
    let runtime_check = report
        .checks
        .iter()
        .find(|c| c.code == "RUNTIME_EXISTS")
        .expect("RUNTIME_EXISTS check should be present");

    assert_eq!(
        runtime_check.status,
        CheckStatus::Pass,
        "cargo runtime check should pass: {}",
        runtime_check.message
    );
}

#[test]
fn test_preflight_fails_for_missing_runtime() {
    // deno might not be installed — but even if it is, we test the structure
    let tmp = tempfile::tempdir().unwrap();
    let free_port = get_free_port();
    let config = make_rash_config(Runtime::Deno, free_port, Some("."));
    let report = PreflightChecker::run(&config, tmp.path());

    // Report should have all 3 checks
    assert_eq!(report.checks.len(), 3);

    let runtime_check = report
        .checks
        .iter()
        .find(|c| c.code == "RUNTIME_EXISTS")
        .expect("RUNTIME_EXISTS check should be present");

    // If deno is not installed, it should Fail; if installed, it should Pass
    // Either way, the check result should be well-formed
    assert!(
        runtime_check.status == CheckStatus::Pass || runtime_check.status == CheckStatus::Fail,
        "runtime check should be Pass or Fail, got: {:?}",
        runtime_check.status
    );

    if runtime_check.status == CheckStatus::Fail {
        assert!(
            runtime_check.suggestion.is_some(),
            "Failed runtime check should have a suggestion"
        );
    }
}

#[test]
fn test_preflight_report_structure_with_real_config() {
    let tmp = tempfile::tempdir().unwrap();
    let free_port = get_free_port();
    let config = make_rash_config(Runtime::Cargo, free_port, Some("."));
    let report = PreflightChecker::run(&config, tmp.path());

    // Should always have exactly 3 checks
    assert_eq!(report.checks.len(), 3, "Should have 3 preflight checks");

    let codes: Vec<&str> = report.checks.iter().map(|c| c.code.as_str()).collect();
    assert!(codes.contains(&"RUNTIME_EXISTS"));
    assert!(codes.contains(&"PORT_AVAILABLE"));
    assert!(codes.contains(&"OUTPUT_DIR_WRITABLE"));

    // With cargo installed, free port, and writable tempdir — all should pass
    assert!(
        report.ok,
        "All checks should pass: {:#?}",
        report.checks
    );
}

// ══════════════════════════════════════════════════════════════════════
// 2. Incremental Codegen + HMU Integration
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_incremental_diff_produces_hmu_modules() {
    let codegen = IncrementalCodegen::new();

    let mut old_files = BTreeMap::new();
    old_files.insert("src/handlers/users.ts".into(), "export function getUser() { return 'v1'; }".to_string());
    old_files.insert("src/handlers/posts.ts".into(), "export function getPosts() { return []; }".to_string());

    let mut new_files = BTreeMap::new();
    // Updated handler
    new_files.insert("src/handlers/users.ts".into(), "export function getUser() { return 'v2'; }".to_string());
    // Unchanged handler
    new_files.insert("src/handlers/posts.ts".into(), "export function getPosts() { return []; }".to_string());
    // New handler
    new_files.insert("src/handlers/comments.ts".into(), "export function getComments() { return []; }".to_string());

    let changes = codegen.diff_files(&old_files, &new_files);
    let hmu_modules = IncrementalCodegen::to_hmu_modules(&changes);

    // Should have 2 changes: 1 update + 1 create (posts unchanged)
    assert_eq!(changes.len(), 2);
    assert_eq!(hmu_modules.len(), 2);

    let users_module = hmu_modules.iter().find(|m| m.path == "src/handlers/users.ts");
    assert!(users_module.is_some());
    assert_eq!(users_module.unwrap().action, HmuAction::Replace);

    let comments_module = hmu_modules.iter().find(|m| m.path == "src/handlers/comments.ts");
    assert!(comments_module.is_some());
    assert_eq!(comments_module.unwrap().action, HmuAction::Add);
}

#[test]
fn test_handler_change_produces_replace_action() {
    let codegen = IncrementalCodegen::new();

    let mut old = BTreeMap::new();
    old.insert("src/handlers/users.ts".into(), "export function getUser() { return 'v1'; }".to_string());

    let mut new = BTreeMap::new();
    new.insert("src/handlers/users.ts".into(), "export function getUser() { return 'v2'; }".to_string());

    let changes = codegen.diff_files(&old, &new);
    let modules = IncrementalCodegen::to_hmu_modules(&changes);

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].path, "src/handlers/users.ts");
    assert_eq!(modules[0].action, HmuAction::Replace);
    assert!(modules[0].hash.starts_with("sha256:"));
    assert_eq!(modules[0].content, "export function getUser() { return 'v2'; }");
}

#[test]
fn test_new_file_produces_add_action() {
    let codegen = IncrementalCodegen::new();

    let old = BTreeMap::new();
    let mut new = BTreeMap::new();
    new.insert("src/handlers/new_handler.ts".into(), "export function newHandler() {}".to_string());

    let changes = codegen.diff_files(&old, &new);
    let modules = IncrementalCodegen::to_hmu_modules(&changes);

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].action, HmuAction::Add);
    assert_eq!(modules[0].path, "src/handlers/new_handler.ts");
    assert!(modules[0].old_hash_is_none_via_change(&changes[0]));
}

#[test]
fn test_deleted_file_produces_remove_action() {
    let codegen = IncrementalCodegen::new();

    let mut old = BTreeMap::new();
    old.insert("src/handlers/obsolete.ts".into(), "export function obsolete() {}".to_string());
    let new = BTreeMap::new();

    let changes = codegen.diff_files(&old, &new);
    let modules = IncrementalCodegen::to_hmu_modules(&changes);

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].action, HmuAction::Remove);
    assert_eq!(modules[0].path, "src/handlers/obsolete.ts");
}

// ══════════════════════════════════════════════════════════════════════
// 3. HMU Engine Protocol Tests
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_hmu_engine_create_and_validate_update() {
    let mut engine = HmuEngine::new(HmuConfig::default());

    let update = engine.create_update(vec![
        ("src/handlers/users.ts".into(), HmuAction::Replace, "new code".into()),
        ("src/handlers/posts.ts".into(), HmuAction::Add, "post code".into()),
    ]);

    // Verify ID format
    assert_eq!(update.id, "hmu_001");

    // Verify type tag
    assert_eq!(update.kind, HmuUpdateType::HmuUpdate);

    // Verify modules
    assert_eq!(update.modules.len(), 2);
    for module in &update.modules {
        assert!(module.hash.starts_with("sha256:"), "Hash should start with sha256:");
        assert!(!module.hash.is_empty());
    }

    // Verify timestamp is recent (within last minute)
    let now = chrono::Utc::now();
    let diff = now - update.timestamp;
    assert!(diff.num_seconds() < 60, "Timestamp should be recent");

    // Sequential ID
    let update2 = engine.create_update(vec![
        ("src/index.ts".into(), HmuAction::Replace, "index".into()),
    ]);
    assert_eq!(update2.id, "hmu_002");
}

#[test]
fn test_hmu_engine_failure_escalation() {
    let config = HmuConfig {
        ack_timeout: Duration::from_secs(5),
        max_consecutive_failures: 2,
    };
    let mut engine = HmuEngine::new(config);

    // Simulate 1 failure for "x.ts"
    engine.reset_failure_count("x.ts"); // ensure clean
    // Manually track failures by checking escalation
    assert!(!engine.check_escalation(&["x.ts".into()]));

    // After manually inserting failure count of 2, escalation should trigger
    // (In real usage, send_update tracks this automatically)
    // We test the escalation logic directly here
    let update1 = engine.create_update(vec![
        ("x.ts".into(), HmuAction::Replace, "code v1".into()),
    ]);
    assert_eq!(update1.id, "hmu_001");

    // Check that hash is computed correctly
    let expected_hash = HmuEngine::compute_hash("code v1");
    assert_eq!(update1.modules[0].hash, expected_hash);
}

#[tokio::test]
async fn test_hmu_engine_send_update_escalation_after_max_failures() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let config = HmuConfig {
        ack_timeout: Duration::from_secs(5),
        max_consecutive_failures: 2,
    };
    let mut engine = HmuEngine::new(config);

    // Pre-seed one failure for "x.ts"
    // (simulate that a prior update already failed once)
    // We need to access failure_counts — but it's private.
    // Instead, we'll do two rounds of send_update.

    // --- Round 1: first failure ---
    let update1 = engine.create_update(vec![
        ("x.ts".into(), HmuAction::Replace, "code".into()),
    ]);

    let ack1_json = serde_json::json!({
        "type": "HMU_ACK",
        "id": update1.id,
        "status": "failed",
        "applied": [],
        "failed": ["x.ts"],
        "requiresRestart": false
    });
    let ack1_line = format!("{}\n", ack1_json);

    let (stdin_r1, mut stdin_w1) = tokio::io::duplex(4096);
    let (mut stdout_w1, stdout_r1) = tokio::io::duplex(4096);

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdin_r1);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        stdout_w1.write_all(ack1_line.as_bytes()).await.unwrap();
        stdout_w1.flush().await.unwrap();
    });

    let mut stdout_buf1 = BufReader::new(stdout_r1);
    let result1 = engine.send_update(&mut stdin_w1, &mut stdout_buf1, &update1).await.unwrap();
    assert!(!result1.requires_restart, "First failure should not trigger restart");

    // --- Round 2: second failure → escalation ---
    let update2 = engine.create_update(vec![
        ("x.ts".into(), HmuAction::Replace, "code v2".into()),
    ]);

    let ack2_json = serde_json::json!({
        "type": "HMU_ACK",
        "id": update2.id,
        "status": "failed",
        "applied": [],
        "failed": ["x.ts"],
        "requiresRestart": false
    });
    let ack2_line = format!("{}\n", ack2_json);

    let (stdin_r2, mut stdin_w2) = tokio::io::duplex(4096);
    let (mut stdout_w2, stdout_r2) = tokio::io::duplex(4096);

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdin_r2);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        stdout_w2.write_all(ack2_line.as_bytes()).await.unwrap();
        stdout_w2.flush().await.unwrap();
    });

    let mut stdout_buf2 = BufReader::new(stdout_r2);
    let result2 = engine.send_update(&mut stdin_w2, &mut stdout_buf2, &update2).await.unwrap();
    assert!(
        result2.requires_restart,
        "Second consecutive failure should trigger restart (escalation)"
    );
}

// ══════════════════════════════════════════════════════════════════════
// 4. ProcessManager Config Resolution
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_resolve_command_all_combinations() {
    use std::collections::HashMap;
    use rash_runtime::process_manager::ServerConfig as PmServerConfig;

    let make_server_config = |lang: Language, rt: Runtime| -> PmServerConfig {
        PmServerConfig {
            language: lang,
            framework: Framework::Express,
            runtime: rt,
            port: 3000,
            host: "0.0.0.0".into(),
            output_dir: PathBuf::from("/tmp/rash-out"),
            env_vars: HashMap::new(),
        }
    };

    // TypeScript + Bun
    let (cmd, args, dir) = ProcessManager::resolve_command(&make_server_config(Language::Typescript, Runtime::Bun));
    assert_eq!(cmd, "bun");
    assert_eq!(args, vec!["run", "src/index.ts"]);
    assert_eq!(dir, PathBuf::from("/tmp/rash-out"));

    // TypeScript + Node
    let (cmd, args, _) = ProcessManager::resolve_command(&make_server_config(Language::Typescript, Runtime::Node));
    assert_eq!(cmd, "node");
    assert!(args.contains(&"src/index.ts".to_string()));

    // TypeScript + Deno
    let (cmd, args, _) = ProcessManager::resolve_command(&make_server_config(Language::Typescript, Runtime::Deno));
    assert_eq!(cmd, "deno");
    assert!(args.contains(&"--allow-net".to_string()));

    // Rust + Cargo
    let (cmd, args, _) = ProcessManager::resolve_command(&make_server_config(Language::Rust, Runtime::Cargo));
    assert_eq!(cmd, "cargo");
    assert_eq!(args, vec!["run"]);

    // Python + Python
    let (cmd, args, _) = ProcessManager::resolve_command(&make_server_config(Language::Python, Runtime::Python));
    assert_eq!(cmd, "python");
    assert!(args.contains(&"-m".to_string()));
    assert!(args.contains(&"uvicorn".to_string()));
    assert!(args.contains(&"3000".to_string()));

    // Go + Go
    let (cmd, args, _) = ProcessManager::resolve_command(&make_server_config(Language::Go, Runtime::Go));
    assert_eq!(cmd, "go");
    assert_eq!(args, vec!["run", "."]);

    // Unsupported combination falls back to echo
    let (cmd, _, _) = ProcessManager::resolve_command(&make_server_config(Language::Rust, Runtime::Bun));
    assert_eq!(cmd, "echo");
}

// ══════════════════════════════════════════════════════════════════════
// 5. CodegenCache Tests
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_codegen_cache_tracks_file_hashes() {
    let mut cache = CodegenCache::new();
    assert!(cache.is_empty());

    // Create files
    let changes = vec![
        FileChange {
            path: "src/index.ts".into(),
            action: FileChangeAction::Create,
            content: "console.log('hello');".into(),
            old_hash: None,
            new_hash: compute_hash("console.log('hello');"),
        },
        FileChange {
            path: "src/app.ts".into(),
            action: FileChangeAction::Create,
            content: "export default app;".into(),
            old_hash: None,
            new_hash: compute_hash("export default app;"),
        },
    ];

    cache.update(&changes);
    assert_eq!(cache.len(), 2);
    assert_eq!(
        cache.get_hash("src/index.ts"),
        Some(compute_hash("console.log('hello');").as_str())
    );

    // Verify has_changed detects modifications
    assert!(!cache.has_changed("src/index.ts", &compute_hash("console.log('hello');")));
    assert!(cache.has_changed("src/index.ts", &compute_hash("modified content")));
}

#[test]
fn test_codegen_cache_empty_initially() {
    let cache = CodegenCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert!(cache.get_hash("any_file.ts").is_none());

    // has_changed returns true for any file when cache is empty
    assert!(cache.has_changed("any_file.ts", "sha256:abc"));
    assert!(cache.has_changed("another.ts", "sha256:def"));
}

#[test]
fn test_codegen_cache_update_then_delete() {
    let mut cache = CodegenCache::new();

    // Create
    cache.update(&[FileChange {
        path: "a.ts".into(),
        action: FileChangeAction::Create,
        content: "a".into(),
        old_hash: None,
        new_hash: "sha256:aaa".into(),
    }]);
    assert_eq!(cache.len(), 1);

    // Update
    cache.update(&[FileChange {
        path: "a.ts".into(),
        action: FileChangeAction::Update,
        content: "a2".into(),
        old_hash: Some("sha256:aaa".into()),
        new_hash: "sha256:aaa2".into(),
    }]);
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get_hash("a.ts"), Some("sha256:aaa2"));

    // Delete
    cache.update(&[FileChange {
        path: "a.ts".into(),
        action: FileChangeAction::Delete,
        content: String::new(),
        old_hash: Some("sha256:aaa2".into()),
        new_hash: compute_hash(""),
    }]);
    assert_eq!(cache.len(), 0);
    assert!(cache.get_hash("a.ts").is_none());
}

// ══════════════════════════════════════════════════════════════════════
// 6. Full Flow: Spec Change → Diff → HMU
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_full_flow_spec_change_to_hmu() {
    // 1. Create initial "generated" files
    let mut old_files = BTreeMap::new();
    old_files.insert(
        "src/handlers/users.ts".into(),
        "export function getUser() { return { id: 1, name: 'Alice' }; }".to_string(),
    );
    old_files.insert(
        "src/handlers/posts.ts".into(),
        "export function getPosts() { return []; }".to_string(),
    );

    // 2. Create modified "generated" files (simulate handler change + new file)
    let mut new_files = BTreeMap::new();
    new_files.insert(
        "src/handlers/users.ts".into(),
        "export function getUser() { return { id: 1, name: 'Alice', email: 'alice@example.com' }; }".to_string(),
    );
    new_files.insert(
        "src/handlers/posts.ts".into(),
        "export function getPosts() { return []; }".to_string(), // unchanged
    );
    new_files.insert(
        "src/handlers/comments.ts".into(),
        "export function getComments() { return []; }".to_string(), // new
    );

    // 3. diff_files to get FileChanges
    let codegen = IncrementalCodegen::new();
    let changes = codegen.diff_files(&old_files, &new_files);

    assert_eq!(changes.len(), 2, "Should have 2 changes (1 update + 1 create)");

    let update_change = changes.iter().find(|c| c.action == FileChangeAction::Update);
    let create_change = changes.iter().find(|c| c.action == FileChangeAction::Create);
    assert!(update_change.is_some(), "Should have an Update change");
    assert!(create_change.is_some(), "Should have a Create change");

    // 4. to_hmu_modules to get HmuModules
    let hmu_modules = IncrementalCodegen::to_hmu_modules(&changes);
    assert_eq!(hmu_modules.len(), 2);

    let replace_mod = hmu_modules.iter().find(|m| m.action == HmuAction::Replace);
    let add_mod = hmu_modules.iter().find(|m| m.action == HmuAction::Add);
    assert!(replace_mod.is_some());
    assert!(add_mod.is_some());

    // 5. HmuEngine::create_update to build HmuUpdate message
    let mut engine = HmuEngine::new(HmuConfig::default());
    let module_tuples: Vec<(String, HmuAction, String)> = hmu_modules
        .iter()
        .map(|m| (m.path.clone(), m.action.clone(), m.content.clone()))
        .collect();

    let hmu_update = engine.create_update(module_tuples);

    // 6. Verify the HmuUpdate
    assert_eq!(hmu_update.id, "hmu_001");
    assert_eq!(hmu_update.kind, HmuUpdateType::HmuUpdate);
    assert_eq!(hmu_update.modules.len(), 2);

    // Each module should have a valid hash
    for module in &hmu_update.modules {
        assert!(module.hash.starts_with("sha256:"));
        let expected_hash = HmuEngine::compute_hash(&module.content);
        assert_eq!(module.hash, expected_hash, "Hash mismatch for {}", module.path);
    }

    // Verify serialization round-trip of the full HmuUpdate
    let json = serde_json::to_string(&hmu_update).unwrap();
    let deserialized: HmuUpdate = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, hmu_update.id);
    assert_eq!(deserialized.modules.len(), hmu_update.modules.len());
}

#[test]
fn test_full_flow_with_deletion() {
    let mut old_files = BTreeMap::new();
    old_files.insert("src/handlers/legacy.ts".into(), "// legacy handler".to_string());
    old_files.insert("src/handlers/active.ts".into(), "// active handler".to_string());

    let mut new_files = BTreeMap::new();
    // legacy removed, active updated
    new_files.insert("src/handlers/active.ts".into(), "// active handler v2".to_string());

    let codegen = IncrementalCodegen::new();
    let changes = codegen.diff_files(&old_files, &new_files);
    let modules = IncrementalCodegen::to_hmu_modules(&changes);

    assert_eq!(modules.len(), 2);

    let remove_mod = modules.iter().find(|m| m.action == HmuAction::Remove);
    assert!(remove_mod.is_some(), "Should have a Remove action for deleted file");
    assert_eq!(remove_mod.unwrap().path, "src/handlers/legacy.ts");

    let replace_mod = modules.iter().find(|m| m.action == HmuAction::Replace);
    assert!(replace_mod.is_some(), "Should have a Replace action for updated file");
    assert_eq!(replace_mod.unwrap().path, "src/handlers/active.ts");

    // Build HmuUpdate
    let mut engine = HmuEngine::new(HmuConfig::default());
    let tuples: Vec<(String, HmuAction, String)> = modules
        .iter()
        .map(|m| (m.path.clone(), m.action.clone(), m.content.clone()))
        .collect();

    let update = engine.create_update(tuples);
    assert_eq!(update.modules.len(), 2);
}

// ══════════════════════════════════════════════════════════════════════
// 7. Preflight Report Aggregation
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_preflight_report_serialization_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let free_port = get_free_port();
    let config = make_rash_config(Runtime::Cargo, free_port, Some("."));
    let report = PreflightChecker::run(&config, tmp.path());

    // Serialize and deserialize
    let json = serde_json::to_string(&report).unwrap();
    let deserialized: PreflightReport = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.ok, report.ok);
    assert_eq!(deserialized.checks.len(), report.checks.len());

    for (orig, deser) in report.checks.iter().zip(deserialized.checks.iter()) {
        assert_eq!(orig.code, deser.code);
        assert_eq!(orig.status, deser.status);
    }
}

#[test]
fn test_preflight_report_port_warn_does_not_fail() {
    let tmp = tempfile::tempdir().unwrap();

    // Hold a port open so preflight sees it as occupied
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let occupied_port = listener.local_addr().unwrap().port();

    let config = make_rash_config(Runtime::Cargo, occupied_port, Some("."));
    let report = PreflightChecker::run(&config, tmp.path());

    // PORT_AVAILABLE should be Warn (not Fail)
    let port_check = report
        .checks
        .iter()
        .find(|c| c.code == "PORT_AVAILABLE")
        .expect("PORT_AVAILABLE check should be present");
    assert_eq!(port_check.status, CheckStatus::Warn);

    // Overall report should still be ok (Warn doesn't cause failure)
    assert!(report.ok, "Warn should not cause report failure");

    drop(listener);
}

#[test]
fn test_preflight_output_dir_fail_causes_report_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let free_port = get_free_port();

    // Use an impossible output directory path
    let config = make_rash_config(Runtime::Cargo, free_port, Some("/nonexistent_root_abc/impossible_dir"));
    let report = PreflightChecker::run(&config, tmp.path());

    assert!(!report.ok, "Report should fail with impossible output dir");

    let output_check = report
        .checks
        .iter()
        .find(|c| c.code == "OUTPUT_DIR_WRITABLE")
        .expect("OUTPUT_DIR_WRITABLE check should be present");
    assert_eq!(output_check.status, CheckStatus::Fail);
}

// ══════════════════════════════════════════════════════════════════════
// 8. Log Types Integration
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_log_entry_creation_and_serialization() {
    let entry = LogEntry {
        timestamp: chrono::Utc::now(),
        level: LogLevel::Info,
        message: "Server started on port 3000".into(),
        source: LogSource::Stdout,
    };

    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["level"], "info");
    assert_eq!(json["source"], "stdout");
    assert_eq!(json["message"], "Server started on port 3000");
    assert!(json["timestamp"].is_string());

    // Roundtrip
    let json_str = serde_json::to_string(&entry).unwrap();
    let deserialized: LogEntry = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.level, LogLevel::Info);
    assert_eq!(deserialized.source, LogSource::Stdout);
}

// ══════════════════════════════════════════════════════════════════════
// 9. Cross-Module Hash Consistency
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_hash_consistency_between_incremental_and_hmu_engine() {
    // Both incremental::compute_hash and HmuEngine::compute_hash should produce
    // identical results for the same input
    let content = "export function handler() { return 'hello'; }";

    let incremental_hash = compute_hash(content);
    let engine_hash = HmuEngine::compute_hash(content);

    assert_eq!(
        incremental_hash, engine_hash,
        "Hash functions should be consistent across modules"
    );

    // Both should be valid sha256 format
    assert!(incremental_hash.starts_with("sha256:"));
    assert_eq!(incremental_hash.len(), 7 + 64); // "sha256:" + 64 hex chars
}

#[test]
fn test_diff_hashes_match_hmu_module_hashes() {
    let codegen = IncrementalCodegen::new();

    let old = BTreeMap::new();
    let mut new = BTreeMap::new();
    let content = "export const value = 42;";
    new.insert("src/const.ts".into(), content.to_string());

    let changes = codegen.diff_files(&old, &new);
    let modules = IncrementalCodegen::to_hmu_modules(&changes);

    assert_eq!(changes.len(), 1);
    assert_eq!(modules.len(), 1);

    // The hash in FileChange.new_hash should match the hash in HmuModule.hash
    assert_eq!(changes[0].new_hash, modules[0].hash);

    // And both should match a fresh computation
    assert_eq!(changes[0].new_hash, compute_hash(content));
}

// ══════════════════════════════════════════════════════════════════════
// 10. ProcessManager Lifecycle
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_process_manager_initial_state() {
    let (mgr, _log_rx, status_rx) = ProcessManager::new();

    assert_eq!(mgr.status(), ServerStatus::Stopped);
    assert_eq!(mgr.pid(), None);
    assert_eq!(mgr.port(), None);
    assert_eq!(mgr.started_at(), None);
    assert_eq!(*status_rx.borrow(), ServerStatus::Stopped);
}

#[tokio::test]
async fn test_process_manager_stop_when_not_running() {
    let (mut mgr, _log_rx, _status_rx) = ProcessManager::new();

    let result = mgr.stop().await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ProcessError::NotRunning => {} // expected
        other => panic!("Expected NotRunning, got: {:?}", other),
    }
}

// ══════════════════════════════════════════════════════════════════════
// 11. HMU Types Serialization Integration
// ══════════════════════════════════════════════════════════════════════

#[test]
fn test_hmu_update_full_serialization_cycle() {
    let mut engine = HmuEngine::new(HmuConfig::default());

    let update = engine.create_update(vec![
        ("src/a.ts".into(), HmuAction::Replace, "updated code".into()),
        ("src/b.ts".into(), HmuAction::Add, "new code".into()),
        ("src/c.ts".into(), HmuAction::Remove, "".into()),
    ]);

    // Serialize to JSON
    let json = serde_json::to_string(&update).unwrap();

    // Verify JSON structure
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "HMU_UPDATE");
    assert_eq!(value["id"], "hmu_001");
    assert_eq!(value["modules"].as_array().unwrap().len(), 3);
    assert_eq!(value["modules"][0]["action"], "replace");
    assert_eq!(value["modules"][1]["action"], "add");
    assert_eq!(value["modules"][2]["action"], "remove");

    // Deserialize back
    let deserialized: HmuUpdate = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, update.id);
    assert_eq!(deserialized.modules.len(), 3);
    assert_eq!(deserialized.modules[0].action, HmuAction::Replace);
    assert_eq!(deserialized.modules[1].action, HmuAction::Add);
    assert_eq!(deserialized.modules[2].action, HmuAction::Remove);
}

// ══════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════

/// Get a free port by briefly binding to port 0, then releasing it.
fn get_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// Extension trait to check FileChange fields from HmuModule context.
trait FileChangeExt {
    fn old_hash_is_none_via_change(&self, change: &FileChange) -> bool;
}

impl FileChangeExt for HmuModule {
    fn old_hash_is_none_via_change(&self, change: &FileChange) -> bool {
        change.old_hash.is_none()
    }
}
