use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Identifies a node in the spec dependency graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id")]
pub enum NodeId {
    Route(String),
    Schema(String),
    Model(String),
    Middleware(String),
    Handler(String),
    GeneratedFile(PathBuf),
}

/// Dependency graph tracking relationships between spec elements and generated files.
///
/// Edge semantics: if `A -> B` exists in `edges`, then a change to A requires B
/// to be regenerated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecDependencyGraph {
    /// Key: source node, Value: set of dependent nodes
    edges: HashMap<NodeId, HashSet<NodeId>>,
}

impl SpecDependencyGraph {
    /// Create an empty dependency graph.
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    /// Add a dependency edge: when `from` changes, `to` must be regenerated.
    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        self.edges.entry(from).or_default().insert(to);
    }

    /// Get direct dependents of a node.
    pub fn dependents(&self, node: &NodeId) -> Option<&HashSet<NodeId>> {
        self.edges.get(node)
    }

    /// Compute transitive closure of affected nodes from a set of changed nodes (BFS).
    pub fn affected_nodes(&self, changed: &[NodeId]) -> HashSet<NodeId> {
        let mut visited = HashSet::new();
        let mut queue: Vec<&NodeId> = changed.iter().collect();

        while let Some(node) = queue.pop() {
            if !visited.insert(node.clone()) {
                continue;
            }
            if let Some(deps) = self.edges.get(node) {
                for dep in deps {
                    if !visited.contains(dep) {
                        queue.push(dep);
                    }
                }
            }
        }

        visited
    }

    /// Return the total number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|s| s.len()).sum()
    }

    /// Return the total number of unique nodes in the graph.
    pub fn node_count(&self) -> usize {
        let mut nodes = HashSet::new();
        for (from, tos) in &self.edges {
            nodes.insert(from);
            for to in tos {
                nodes.insert(to);
            }
        }
        nodes.len()
    }
}

/// Plan describing which files need to be regenerated after spec changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangePlan {
    /// Spec elements that were changed or affected
    pub affected_specs: Vec<NodeId>,
    /// Generated files that need to be regenerated
    pub affected_files: Vec<PathBuf>,
    /// Whether a full regeneration is required (e.g., language/framework change)
    pub requires_full_regen: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = SpecDependencyGraph::new();
        assert_eq!(graph.edge_count(), 0);
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_add_edge_and_query() {
        let mut graph = SpecDependencyGraph::new();
        let schema = NodeId::Schema("UserResponse".to_string());
        let handler = NodeId::Handler("users.getUser".to_string());
        let file = NodeId::GeneratedFile(PathBuf::from("src/handlers/users.ts"));

        graph.add_edge(schema.clone(), handler.clone());
        graph.add_edge(handler.clone(), file.clone());

        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.node_count(), 3);

        let deps = graph.dependents(&schema).unwrap();
        assert!(deps.contains(&handler));
        assert!(!deps.contains(&file));
    }

    #[test]
    fn test_transitive_affected_nodes() {
        let mut graph = SpecDependencyGraph::new();

        let schema = NodeId::Schema("UserResponse".to_string());
        let handler = NodeId::Handler("users.getUser".to_string());
        let route = NodeId::Route("/v1/users/:id".to_string());
        let file1 = NodeId::GeneratedFile(PathBuf::from("src/handlers/users.ts"));
        let file2 = NodeId::GeneratedFile(PathBuf::from("src/routes/index.ts"));

        graph.add_edge(schema.clone(), handler.clone());
        graph.add_edge(handler.clone(), file1.clone());
        graph.add_edge(handler.clone(), route.clone());
        graph.add_edge(route.clone(), file2.clone());

        let affected = graph.affected_nodes(std::slice::from_ref(&schema));

        assert!(affected.contains(&schema));
        assert!(affected.contains(&handler));
        assert!(affected.contains(&file1));
        assert!(affected.contains(&route));
        assert!(affected.contains(&file2));
        assert_eq!(affected.len(), 5);
    }

    #[test]
    fn test_no_infinite_loop_on_cycle() {
        let mut graph = SpecDependencyGraph::new();
        let a = NodeId::Handler("a".to_string());
        let b = NodeId::Handler("b".to_string());

        graph.add_edge(a.clone(), b.clone());
        graph.add_edge(b.clone(), a.clone());

        let affected = graph.affected_nodes(std::slice::from_ref(&a));
        assert!(affected.contains(&a));
        assert!(affected.contains(&b));
        assert_eq!(affected.len(), 2);
    }

    #[test]
    fn test_node_id_serialization() {
        let node = NodeId::Route("/v1/users".to_string());
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["kind"], "Route");
        assert_eq!(json["id"], "/v1/users");

        let deserialized: NodeId = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node);
    }

    #[test]
    fn test_file_change_plan() {
        let plan = FileChangePlan {
            affected_specs: vec![
                NodeId::Schema("User".to_string()),
                NodeId::Handler("users.getUser".to_string()),
            ],
            affected_files: vec![
                PathBuf::from("src/schemas/user.ts"),
                PathBuf::from("src/handlers/users.ts"),
            ],
            requires_full_regen: false,
        };

        let json = serde_json::to_value(&plan).unwrap();
        let deserialized: FileChangePlan = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.affected_specs.len(), 2);
        assert_eq!(deserialized.affected_files.len(), 2);
        assert!(!deserialized.requires_full_regen);
    }
}
