use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use rash_ir::dep_graph::{FileChangePlan, NodeId, SpecDependencyGraph};
use rash_ir::types::ProjectIR;

use crate::hmu_types::{HmuAction, HmuModule};

/// Type of spec change detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecChange {
    RouteModified(String),
    SchemaModified(String),
    HandlerModified(String),
    MiddlewareModified(String),
    ConfigModified,
}

impl SpecChange {
    /// Convert to the corresponding dependency graph NodeId.
    pub fn to_node_id(&self) -> Option<NodeId> {
        match self {
            SpecChange::RouteModified(path) => Some(NodeId::Route(path.clone())),
            SpecChange::SchemaModified(name) => Some(NodeId::Schema(name.clone())),
            SpecChange::HandlerModified(name) => Some(NodeId::Handler(name.clone())),
            SpecChange::MiddlewareModified(name) => Some(NodeId::Middleware(name.clone())),
            SpecChange::ConfigModified => None,
        }
    }
}

/// A single file change produced by incremental codegen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub action: FileChangeAction,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_hash: Option<String>,
    pub new_hash: String,
}

/// Action to perform on a generated file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeAction {
    Create,
    Update,
    Delete,
}

/// Cache of previously generated file content hashes.
#[derive(Debug, Clone, Default)]
pub struct CodegenCache {
    hashes: BTreeMap<String, String>,
}

impl CodegenCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update cache with new file hashes.
    pub fn update(&mut self, changes: &[FileChange]) {
        for change in changes {
            match change.action {
                FileChangeAction::Create | FileChangeAction::Update => {
                    self.hashes
                        .insert(change.path.clone(), change.new_hash.clone());
                }
                FileChangeAction::Delete => {
                    self.hashes.remove(&change.path);
                }
            }
        }
    }

    /// Get the cached hash for a file.
    pub fn get_hash(&self, path: &str) -> Option<&str> {
        self.hashes.get(path).map(|s| s.as_str())
    }

    /// Check if a file has changed based on content hash.
    pub fn has_changed(&self, path: &str, new_hash: &str) -> bool {
        match self.hashes.get(path) {
            Some(cached) => cached != new_hash,
            None => true,
        }
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

/// Compute SHA-256 hash of content, returning "sha256:{hex}".
pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}

/// Incremental code generator that only regenerates changed files.
pub struct IncrementalCodegen {
    cache: CodegenCache,
    dep_graph: SpecDependencyGraph,
}

impl IncrementalCodegen {
    pub fn new() -> Self {
        Self {
            cache: CodegenCache::new(),
            dep_graph: SpecDependencyGraph::new(),
        }
    }

    /// Access the internal cache (for inspection/testing).
    pub fn cache(&self) -> &CodegenCache {
        &self.cache
    }

    /// Access the internal dependency graph.
    pub fn dep_graph(&self) -> &SpecDependencyGraph {
        &self.dep_graph
    }

    /// Build dependency graph from a ProjectIR.
    ///
    /// Walks all routes, schemas, handlers, and middleware to create edges
    /// reflecting how changes propagate through the codebase.
    pub fn build_dep_graph(&mut self, project: &ProjectIR) {
        self.dep_graph = SpecDependencyGraph::new();

        // Schema -> generated schema file
        for schema in &project.schemas {
            let schema_node = NodeId::Schema(schema.name.clone());
            let file_node = NodeId::GeneratedFile(PathBuf::from(format!(
                "src/schemas/{}.ts",
                schema.name.to_lowercase()
            )));
            self.dep_graph.add_edge(schema_node, file_node);
        }

        // Handler -> generated handler file
        for handler in &project.handlers {
            let handler_node = NodeId::Handler(handler.name.clone());
            let filename = handler.name.replace('.', "_");
            let file_node = NodeId::GeneratedFile(PathBuf::from(format!(
                "src/handlers/{}.ts",
                filename
            )));
            self.dep_graph.add_edge(handler_node.clone(), file_node);
        }

        // Middleware -> generated middleware file
        for mw in &project.middleware {
            let mw_node = NodeId::Middleware(mw.name.clone());
            let filename = mw.name.replace('.', "_");
            let file_node = NodeId::GeneratedFile(PathBuf::from(format!(
                "src/middleware/{}.ts",
                filename
            )));
            self.dep_graph.add_edge(mw_node, file_node);
        }

        // Route -> generated routes index file
        let routes_file = NodeId::GeneratedFile(PathBuf::from("src/routes/index.ts"));
        for route in &project.routes {
            let route_node = NodeId::Route(route.path.clone());
            self.dep_graph
                .add_edge(route_node.clone(), routes_file.clone());

            // Route endpoints reference handlers and middleware
            for (_method, endpoint) in &route.methods {
                let handler_node = NodeId::Handler(endpoint.handler_ref.clone());
                self.dep_graph
                    .add_edge(handler_node, route_node.clone());

                for mw_ref in &endpoint.middleware {
                    let mw_node = NodeId::Middleware(mw_ref.clone());
                    self.dep_graph
                        .add_edge(mw_node, route_node.clone());
                }

                // If endpoint references schemas, add edges
                if let Some(ref body_schema) = endpoint.request.body_schema {
                    let schema_node = NodeId::Schema(body_schema.clone());
                    let handler_node = NodeId::Handler(endpoint.handler_ref.clone());
                    self.dep_graph.add_edge(schema_node, handler_node);
                }
                if let Some(ref query_schema) = endpoint.request.query_schema {
                    let schema_node = NodeId::Schema(query_schema.clone());
                    let handler_node = NodeId::Handler(endpoint.handler_ref.clone());
                    self.dep_graph.add_edge(schema_node, handler_node);
                }
                for (_status, resp) in &endpoint.response {
                    if let Some(ref schema_ref) = resp.schema_ref {
                        let schema_node = NodeId::Schema(schema_ref.clone());
                        let handler_node = NodeId::Handler(endpoint.handler_ref.clone());
                        self.dep_graph.add_edge(schema_node, handler_node);
                    }
                }
            }
        }

        // Middleware handler_ref -> middleware (handler changes affect middleware)
        for mw in &project.middleware {
            if let Some(ref handler_ref) = mw.handler_ref {
                let handler_node = NodeId::Handler(handler_ref.clone());
                let mw_node = NodeId::Middleware(mw.name.clone());
                self.dep_graph.add_edge(handler_node, mw_node);
            }
        }
    }

    /// Compute which files need regeneration based on spec changes.
    pub fn compute_change_plan(&self, changes: &[SpecChange]) -> FileChangePlan {
        let requires_full_regen = changes.iter().any(|c| matches!(c, SpecChange::ConfigModified));

        let node_ids: Vec<NodeId> = changes.iter().filter_map(|c| c.to_node_id()).collect();

        let affected = self.dep_graph.affected_nodes(&node_ids);

        let mut affected_specs = Vec::new();
        let mut affected_files = Vec::new();

        for node in &affected {
            match node {
                NodeId::GeneratedFile(path) => {
                    affected_files.push(path.clone());
                }
                _ => {
                    affected_specs.push(node.clone());
                }
            }
        }

        // Sort for deterministic output
        affected_specs.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        affected_files.sort();

        FileChangePlan {
            affected_specs,
            affected_files,
            requires_full_regen,
        }
    }

    /// Detect changes between old and new generated files.
    pub fn diff_files(
        &self,
        old_files: &BTreeMap<String, String>,
        new_files: &BTreeMap<String, String>,
    ) -> Vec<FileChange> {
        let mut changes = Vec::new();

        // Files in new but not in old -> Create
        // Files in both but content differs -> Update
        for (path, new_content) in new_files {
            let new_hash = compute_hash(new_content);
            match old_files.get(path) {
                None => {
                    changes.push(FileChange {
                        path: path.clone(),
                        action: FileChangeAction::Create,
                        content: new_content.clone(),
                        old_hash: None,
                        new_hash,
                    });
                }
                Some(old_content) => {
                    let old_hash = compute_hash(old_content);
                    if old_hash != new_hash {
                        changes.push(FileChange {
                            path: path.clone(),
                            action: FileChangeAction::Update,
                            content: new_content.clone(),
                            old_hash: Some(old_hash),
                            new_hash,
                        });
                    }
                }
            }
        }

        // Files in old but not in new -> Delete
        for (path, old_content) in old_files {
            if !new_files.contains_key(path) {
                changes.push(FileChange {
                    path: path.clone(),
                    action: FileChangeAction::Delete,
                    content: String::new(),
                    old_hash: Some(compute_hash(old_content)),
                    new_hash: compute_hash(""),
                });
            }
        }

        changes
    }

    /// Convert FileChanges to HMU modules for sending to the server.
    pub fn to_hmu_modules(changes: &[FileChange]) -> Vec<HmuModule> {
        changes
            .iter()
            .map(|change| HmuModule {
                path: change.path.clone(),
                action: match change.action {
                    FileChangeAction::Create => HmuAction::Add,
                    FileChangeAction::Update => HmuAction::Replace,
                    FileChangeAction::Delete => HmuAction::Remove,
                },
                content: change.content.clone(),
                hash: change.new_hash.clone(),
            })
            .collect()
    }

    /// Update the internal cache after successful code generation.
    pub fn update_cache(&mut self, changes: &[FileChange]) {
        self.cache.update(changes);
    }
}

impl Default for IncrementalCodegen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // ── CodegenCache tests ──────────────────────────────────────────

    #[test]
    fn cache_starts_empty() {
        let cache = CodegenCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.get_hash("any").is_none());
    }

    #[test]
    fn cache_update_tracks_creates_and_updates() {
        let mut cache = CodegenCache::new();
        let changes = vec![
            FileChange {
                path: "src/index.ts".into(),
                action: FileChangeAction::Create,
                content: "console.log('hello');".into(),
                old_hash: None,
                new_hash: "sha256:abc123".into(),
            },
            FileChange {
                path: "src/app.ts".into(),
                action: FileChangeAction::Update,
                content: "updated".into(),
                old_hash: Some("sha256:old".into()),
                new_hash: "sha256:new456".into(),
            },
        ];

        cache.update(&changes);

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get_hash("src/index.ts"), Some("sha256:abc123"));
        assert_eq!(cache.get_hash("src/app.ts"), Some("sha256:new456"));
    }

    #[test]
    fn cache_update_removes_deleted_files() {
        let mut cache = CodegenCache::new();

        // First create
        cache.update(&[FileChange {
            path: "src/old.ts".into(),
            action: FileChangeAction::Create,
            content: "old".into(),
            old_hash: None,
            new_hash: "sha256:old".into(),
        }]);
        assert_eq!(cache.len(), 1);

        // Then delete
        cache.update(&[FileChange {
            path: "src/old.ts".into(),
            action: FileChangeAction::Delete,
            content: String::new(),
            old_hash: Some("sha256:old".into()),
            new_hash: "sha256:empty".into(),
        }]);
        assert_eq!(cache.len(), 0);
        assert!(cache.get_hash("src/old.ts").is_none());
    }

    #[test]
    fn cache_has_changed_detects_difference() {
        let mut cache = CodegenCache::new();
        cache.update(&[FileChange {
            path: "a.ts".into(),
            action: FileChangeAction::Create,
            content: "x".into(),
            old_hash: None,
            new_hash: "sha256:aaa".into(),
        }]);

        assert!(!cache.has_changed("a.ts", "sha256:aaa"));
        assert!(cache.has_changed("a.ts", "sha256:bbb"));
        // Unknown file is always "changed"
        assert!(cache.has_changed("unknown.ts", "sha256:aaa"));
    }

    // ── compute_hash tests ──────────────────────────────────────────

    #[test]
    fn compute_hash_deterministic() {
        let h1 = compute_hash("hello world");
        let h2 = compute_hash("hello world");
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn compute_hash_different_for_different_content() {
        let h1 = compute_hash("hello");
        let h2 = compute_hash("world");
        assert_ne!(h1, h2);
    }

    // ── diff_files tests ────────────────────────────────────────────

    #[test]
    fn diff_files_detects_create() {
        let codegen = IncrementalCodegen::new();
        let old = BTreeMap::new();
        let mut new = BTreeMap::new();
        new.insert("src/index.ts".into(), "console.log('hi');".into());

        let changes = codegen.diff_files(&old, &new);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "src/index.ts");
        assert_eq!(changes[0].action, FileChangeAction::Create);
        assert!(changes[0].old_hash.is_none());
        assert!(changes[0].new_hash.starts_with("sha256:"));
    }

    #[test]
    fn diff_files_detects_update() {
        let codegen = IncrementalCodegen::new();
        let mut old = BTreeMap::new();
        old.insert("src/index.ts".into(), "old content".to_string());
        let mut new = BTreeMap::new();
        new.insert("src/index.ts".into(), "new content".to_string());

        let changes = codegen.diff_files(&old, &new);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action, FileChangeAction::Update);
        assert!(changes[0].old_hash.is_some());
        assert_ne!(changes[0].old_hash.as_ref().unwrap(), &changes[0].new_hash);
    }

    #[test]
    fn diff_files_detects_delete() {
        let codegen = IncrementalCodegen::new();
        let mut old = BTreeMap::new();
        old.insert("src/removed.ts".into(), "gone".to_string());
        let new = BTreeMap::new();

        let changes = codegen.diff_files(&old, &new);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "src/removed.ts");
        assert_eq!(changes[0].action, FileChangeAction::Delete);
        assert!(changes[0].old_hash.is_some());
    }

    #[test]
    fn diff_files_ignores_unchanged() {
        let codegen = IncrementalCodegen::new();
        let mut old = BTreeMap::new();
        old.insert("src/same.ts".into(), "unchanged content".to_string());
        let mut new = BTreeMap::new();
        new.insert("src/same.ts".into(), "unchanged content".to_string());

        let changes = codegen.diff_files(&old, &new);
        assert!(changes.is_empty());
    }

    #[test]
    fn diff_files_mixed_operations() {
        let codegen = IncrementalCodegen::new();
        let mut old = BTreeMap::new();
        old.insert("keep.ts".into(), "same".to_string());
        old.insert("update.ts".into(), "old".to_string());
        old.insert("delete.ts".into(), "remove me".to_string());

        let mut new = BTreeMap::new();
        new.insert("keep.ts".into(), "same".to_string());
        new.insert("update.ts".into(), "new".to_string());
        new.insert("create.ts".into(), "fresh".to_string());

        let changes = codegen.diff_files(&old, &new);

        assert_eq!(changes.len(), 3);

        let create = changes.iter().find(|c| c.path == "create.ts").unwrap();
        assert_eq!(create.action, FileChangeAction::Create);

        let update = changes.iter().find(|c| c.path == "update.ts").unwrap();
        assert_eq!(update.action, FileChangeAction::Update);

        let delete = changes.iter().find(|c| c.path == "delete.ts").unwrap();
        assert_eq!(delete.action, FileChangeAction::Delete);
    }

    // ── to_hmu_modules tests ────────────────────────────────────────

    #[test]
    fn to_hmu_modules_maps_actions_correctly() {
        let changes = vec![
            FileChange {
                path: "src/new.ts".into(),
                action: FileChangeAction::Create,
                content: "new file".into(),
                old_hash: None,
                new_hash: "sha256:aaa".into(),
            },
            FileChange {
                path: "src/updated.ts".into(),
                action: FileChangeAction::Update,
                content: "updated".into(),
                old_hash: Some("sha256:old".into()),
                new_hash: "sha256:bbb".into(),
            },
            FileChange {
                path: "src/removed.ts".into(),
                action: FileChangeAction::Delete,
                content: String::new(),
                old_hash: Some("sha256:ccc".into()),
                new_hash: "sha256:empty".into(),
            },
        ];

        let modules = IncrementalCodegen::to_hmu_modules(&changes);

        assert_eq!(modules.len(), 3);
        assert_eq!(modules[0].action, HmuAction::Add);
        assert_eq!(modules[0].path, "src/new.ts");
        assert_eq!(modules[0].content, "new file");
        assert_eq!(modules[0].hash, "sha256:aaa");

        assert_eq!(modules[1].action, HmuAction::Replace);
        assert_eq!(modules[2].action, HmuAction::Remove);
    }

    #[test]
    fn to_hmu_modules_empty_input() {
        let modules = IncrementalCodegen::to_hmu_modules(&[]);
        assert!(modules.is_empty());
    }

    // ── SpecChange -> NodeId tests ──────────────────────────────────

    #[test]
    fn spec_change_to_node_id() {
        assert_eq!(
            SpecChange::RouteModified("/users".into()).to_node_id(),
            Some(NodeId::Route("/users".into()))
        );
        assert_eq!(
            SpecChange::SchemaModified("User".into()).to_node_id(),
            Some(NodeId::Schema("User".into()))
        );
        assert_eq!(
            SpecChange::HandlerModified("getUser".into()).to_node_id(),
            Some(NodeId::Handler("getUser".into()))
        );
        assert_eq!(
            SpecChange::MiddlewareModified("auth".into()).to_node_id(),
            Some(NodeId::Middleware("auth".into()))
        );
        assert_eq!(SpecChange::ConfigModified.to_node_id(), None);
    }

    // ── compute_change_plan tests ───────────────────────────────────

    #[test]
    fn compute_change_plan_config_triggers_full_regen() {
        let codegen = IncrementalCodegen::new();
        let plan = codegen.compute_change_plan(&[SpecChange::ConfigModified]);
        assert!(plan.requires_full_regen);
    }

    #[test]
    fn compute_change_plan_non_config_is_partial() {
        let codegen = IncrementalCodegen::new();
        let plan =
            codegen.compute_change_plan(&[SpecChange::HandlerModified("getUser".into())]);
        assert!(!plan.requires_full_regen);
    }

    #[test]
    fn compute_change_plan_with_dep_graph() {
        let mut codegen = IncrementalCodegen::new();

        // Build a small graph: Schema("User") -> Handler("getUser") -> GeneratedFile
        let schema = NodeId::Schema("User".into());
        let handler = NodeId::Handler("getUser".into());
        let file = NodeId::GeneratedFile(PathBuf::from("src/handlers/getUser.ts"));

        codegen.dep_graph = SpecDependencyGraph::new();
        codegen.dep_graph.add_edge(schema.clone(), handler.clone());
        codegen.dep_graph.add_edge(handler.clone(), file.clone());

        let plan = codegen.compute_change_plan(&[SpecChange::SchemaModified("User".into())]);

        assert!(!plan.requires_full_regen);
        assert!(plan
            .affected_files
            .contains(&PathBuf::from("src/handlers/getUser.ts")));
        // Schema and Handler should be in affected_specs
        assert!(plan.affected_specs.contains(&schema));
        assert!(plan.affected_specs.contains(&handler));
    }

    #[test]
    fn compute_change_plan_mixed_with_config() {
        let mut codegen = IncrementalCodegen::new();

        let handler = NodeId::Handler("getUser".into());
        let file = NodeId::GeneratedFile(PathBuf::from("src/handlers/getUser.ts"));
        codegen.dep_graph = SpecDependencyGraph::new();
        codegen.dep_graph.add_edge(handler, file);

        let plan = codegen.compute_change_plan(&[
            SpecChange::HandlerModified("getUser".into()),
            SpecChange::ConfigModified,
        ]);

        // ConfigModified forces full regen even with other changes
        assert!(plan.requires_full_regen);
        assert!(!plan.affected_files.is_empty());
    }

    // ── update_cache integration test ───────────────────────────────

    #[test]
    fn update_cache_round_trip() {
        let mut codegen = IncrementalCodegen::new();
        assert!(codegen.cache().is_empty());

        let changes = vec![
            FileChange {
                path: "a.ts".into(),
                action: FileChangeAction::Create,
                content: "a".into(),
                old_hash: None,
                new_hash: "sha256:aaa".into(),
            },
            FileChange {
                path: "b.ts".into(),
                action: FileChangeAction::Create,
                content: "b".into(),
                old_hash: None,
                new_hash: "sha256:bbb".into(),
            },
        ];

        codegen.update_cache(&changes);
        assert_eq!(codegen.cache().len(), 2);
        assert!(!codegen.cache().has_changed("a.ts", "sha256:aaa"));
        assert!(codegen.cache().has_changed("a.ts", "sha256:xxx"));

        // Update a.ts, delete b.ts
        codegen.update_cache(&[
            FileChange {
                path: "a.ts".into(),
                action: FileChangeAction::Update,
                content: "a2".into(),
                old_hash: Some("sha256:aaa".into()),
                new_hash: "sha256:aaa2".into(),
            },
            FileChange {
                path: "b.ts".into(),
                action: FileChangeAction::Delete,
                content: String::new(),
                old_hash: Some("sha256:bbb".into()),
                new_hash: "sha256:empty".into(),
            },
        ]);

        assert_eq!(codegen.cache().len(), 1);
        assert_eq!(codegen.cache().get_hash("a.ts"), Some("sha256:aaa2"));
        assert!(codegen.cache().get_hash("b.ts").is_none());
    }

    // ── FileChange serialization ────────────────────────────────────

    #[test]
    fn file_change_action_serialization() {
        assert_eq!(
            serde_json::to_value(FileChangeAction::Create).unwrap(),
            "create"
        );
        assert_eq!(
            serde_json::to_value(FileChangeAction::Update).unwrap(),
            "update"
        );
        assert_eq!(
            serde_json::to_value(FileChangeAction::Delete).unwrap(),
            "delete"
        );
    }

    #[test]
    fn file_change_roundtrip() {
        let change = FileChange {
            path: "src/index.ts".into(),
            action: FileChangeAction::Update,
            content: "new code".into(),
            old_hash: Some("sha256:old".into()),
            new_hash: "sha256:new".into(),
        };

        let json = serde_json::to_string(&change).unwrap();
        let deserialized: FileChange = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.path, "src/index.ts");
        assert_eq!(deserialized.action, FileChangeAction::Update);
        assert_eq!(deserialized.old_hash.as_deref(), Some("sha256:old"));
        assert_eq!(deserialized.new_hash, "sha256:new");
    }

    // ── build_dep_graph tests ───────────────────────────────────────

    #[test]
    fn build_dep_graph_empty_project() {
        let mut codegen = IncrementalCodegen::new();
        let project = ProjectIR {
            config: serde_json::json!({}),
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        };

        codegen.build_dep_graph(&project);
        assert_eq!(codegen.dep_graph().edge_count(), 0);
    }

    #[test]
    fn build_dep_graph_creates_schema_edges() {
        use rash_ir::types::SchemaIR;

        let mut codegen = IncrementalCodegen::new();
        let project = ProjectIR {
            config: serde_json::json!({}),
            routes: vec![],
            schemas: vec![SchemaIR {
                name: "User".into(),
                definitions: Default::default(),
            }],
            models: vec![],
            middleware: vec![],
            handlers: vec![],
        };

        codegen.build_dep_graph(&project);

        // Schema("User") -> GeneratedFile("src/schemas/user.ts")
        let affected = codegen
            .dep_graph()
            .affected_nodes(&[NodeId::Schema("User".into())]);
        assert!(affected.contains(&NodeId::GeneratedFile(PathBuf::from(
            "src/schemas/user.ts"
        ))));
    }

    #[test]
    fn build_dep_graph_creates_handler_edges() {
        use rash_ir::types::HandlerIR;
        use rash_ir::expr::TypeIR;

        let mut codegen = IncrementalCodegen::new();
        let project = ProjectIR {
            config: serde_json::json!({}),
            routes: vec![],
            schemas: vec![],
            models: vec![],
            middleware: vec![],
            handlers: vec![HandlerIR {
                name: "users.getUser".into(),
                is_async: true,
                params: vec![],
                return_type: TypeIR::Void,
                body: vec![],
                max_tier: rash_spec::types::common::Tier::Universal,
                bridge_languages: Default::default(),
            }],
        };

        codegen.build_dep_graph(&project);

        let affected = codegen
            .dep_graph()
            .affected_nodes(&[NodeId::Handler("users.getUser".into())]);
        assert!(affected.contains(&NodeId::GeneratedFile(PathBuf::from(
            "src/handlers/users_getUser.ts"
        ))));
    }
}
