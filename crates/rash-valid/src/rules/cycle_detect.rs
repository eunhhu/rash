use std::collections::{HashMap, HashSet};

use rash_spec::index::SpecIndex;
use rash_spec::loader::LoadedProject;
use rash_spec::types::error::{ErrorEntry, ValidationReport, E_REF_CYCLE};

/// Detect circular references between schemas using DFS.
pub fn check(project: &LoadedProject, _index: &SpecIndex, report: &mut ValidationReport) {
    // Build adjacency list for schema references
    let mut graph: HashMap<String, Vec<(String, String)>> = HashMap::new(); // name -> [(target, file)]

    for (file, schema) in &project.schemas {
        for (def_name, def_value) in &schema.definitions {
            let refs = extract_refs_from_value(def_value);
            for ref_target in refs {
                graph
                    .entry(def_name.clone())
                    .or_default()
                    .push((ref_target, file.clone()));
            }
        }
    }

    // DFS cycle detection
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();

    for node in graph.keys() {
        if !visited.contains(node) {
            let mut path = Vec::new();
            if let Some(cycle) =
                dfs_find_cycle(node, &graph, &mut visited, &mut in_stack, &mut path)
            {
                let cycle_str = cycle.join(" -> ");
                // Find the file for the first node in the cycle
                let file = graph
                    .get(&cycle[0])
                    .and_then(|edges| edges.first())
                    .map(|(_, f)| f.as_str())
                    .unwrap_or("unknown");

                report.push(
                    ErrorEntry::error(
                        E_REF_CYCLE,
                        format!("Circular reference detected: {}", cycle_str),
                        file,
                        &format!("$.definitions.{}", cycle[0]),
                    )
                    .with_suggestion(
                        "Break the circular reference by removing or reorganizing one of the references",
                    ),
                );
            }
        }
    }
}

/// DFS to find cycles, returns the cycle path if found
fn dfs_find_cycle(
    node: &str,
    graph: &HashMap<String, Vec<(String, String)>>,
    visited: &mut HashSet<String>,
    in_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    in_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(edges) = graph.get(node) {
        for (target, _) in edges {
            if !visited.contains(target) {
                if let Some(cycle) = dfs_find_cycle(target, graph, visited, in_stack, path) {
                    return Some(cycle);
                }
            } else if in_stack.contains(target) {
                // Found a cycle: extract the cycle from path
                let start_idx = path.iter().position(|n| n == target).unwrap_or(0);
                let mut cycle: Vec<String> = path[start_idx..].to_vec();
                cycle.push(target.clone()); // Close the cycle
                return Some(cycle);
            }
        }
    }

    in_stack.remove(node);
    path.pop();
    None
}

/// Extract $ref values from a JSON Schema value
fn extract_refs_from_value(value: &serde_json::Value) -> Vec<String> {
    let mut refs = Vec::new();
    collect_refs(value, &mut refs);
    refs
}

fn collect_refs(value: &serde_json::Value, refs: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            // Check both "$ref" (JSON Schema style) and "ref" (Rash spec style)
            for key in &["$ref", "ref"] {
                if let Some(serde_json::Value::String(r)) = map.get(*key) {
                    if let Some(name) = extract_ref_target(r) {
                        refs.push(name);
                    }
                }
            }
            for v in map.values() {
                collect_refs(v, refs);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_refs(v, refs);
            }
        }
        _ => {}
    }
}

/// Extract the target definition name from various ref formats:
/// - `"#/definitions/Name"` → `"Name"`
/// - `"other.schema#Name"` → `"Name"` (cross-file ref)
/// - `"Name"` → `"Name"` (simple ref, only if it looks like a type name)
fn extract_ref_target(r: &str) -> Option<String> {
    // JSON Schema internal ref: #/definitions/Name
    if let Some(name) = r.strip_prefix("#/definitions/") {
        return Some(name.to_string());
    }
    // Cross-file ref: file.schema#DefinitionName
    if let Some((_file, name)) = r.split_once('#') {
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    // Simple ref: a type-like name (starts with uppercase, no slashes or special chars)
    // e.g., "UserResponse", "ErrorType"
    if !r.is_empty()
        && r.chars().next().is_some_and(|c| c.is_uppercase())
        && !r.contains('/')
        && !r.contains('.')
    {
        return Some(r.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_refs_from_json_schema() {
        let value = serde_json::json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "array",
                    "items": { "$ref": "#/definitions/UserResponse" }
                },
                "error": { "$ref": "#/definitions/ErrorResponse" }
            }
        });

        let refs = extract_refs_from_value(&value);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"UserResponse".to_string()));
        assert!(refs.contains(&"ErrorResponse".to_string()));
    }

    #[test]
    fn test_extract_ref_target_simple_name() {
        // Simple ref: uppercase type-like names should be detected
        assert_eq!(extract_ref_target("UserResponse"), Some("UserResponse".to_string()));
        assert_eq!(extract_ref_target("ErrorType"), Some("ErrorType".to_string()));
        // Lowercase names should NOT be treated as type refs
        assert_eq!(extract_ref_target("somefield"), None);
        // Already handled formats
        assert_eq!(extract_ref_target("#/definitions/Foo"), Some("Foo".to_string()));
        assert_eq!(extract_ref_target("other.schema#Bar"), Some("Bar".to_string()));
    }

    #[test]
    fn test_extract_refs_simple_ref_in_schema() {
        let value = serde_json::json!({
            "type": "object",
            "properties": {
                "author": { "$ref": "Author" },
                "tags": {
                    "type": "array",
                    "items": { "$ref": "Tag" }
                }
            }
        });

        let refs = extract_refs_from_value(&value);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"Author".to_string()));
        assert!(refs.contains(&"Tag".to_string()));
    }

    #[test]
    fn test_dfs_no_cycle() {
        let mut graph = HashMap::new();
        graph.insert(
            "A".to_string(),
            vec![("B".to_string(), "a.json".to_string())],
        );
        graph.insert(
            "B".to_string(),
            vec![("C".to_string(), "b.json".to_string())],
        );

        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut path = Vec::new();

        let result = dfs_find_cycle("A", &graph, &mut visited, &mut in_stack, &mut path);
        assert!(result.is_none());
    }

    #[test]
    fn test_dfs_with_cycle() {
        let mut graph = HashMap::new();
        graph.insert(
            "A".to_string(),
            vec![("B".to_string(), "a.json".to_string())],
        );
        graph.insert(
            "B".to_string(),
            vec![("A".to_string(), "b.json".to_string())],
        );

        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut path = Vec::new();

        let result = dfs_find_cycle("A", &graph, &mut visited, &mut in_stack, &mut path);
        assert!(result.is_some());
        let cycle = result.unwrap();
        assert!(cycle.contains(&"A".to_string()));
        assert!(cycle.contains(&"B".to_string()));
    }
}
