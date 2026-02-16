use indexmap::IndexMap;
use regex::Regex;

use rash_spec::types::schema::SchemaSpec;

/// Extract schemas from TypeScript source code.
///
/// Recognizes:
/// - `const XxxSchema = z.object({ ... })`  (Zod schemas)
/// - `interface Xxx { ... }`                 (TypeScript interfaces)
/// - `type Xxx = { ... }`                    (TypeScript type aliases)
pub fn extract_schemas(source: &str, warnings: &mut Vec<String>) -> Vec<SchemaSpec> {
    let mut schemas = Vec::new();

    extract_zod_schemas(source, &mut schemas, warnings);
    extract_ts_interfaces(source, &mut schemas, warnings);
    extract_ts_type_aliases(source, &mut schemas, warnings);

    schemas
}

/// Extract Zod schemas: `const XxxSchema = z.object({ ... })`
fn extract_zod_schemas(
    source: &str,
    schemas: &mut Vec<SchemaSpec>,
    _warnings: &mut Vec<String>,
) {
    let zod_re = Regex::new(
        r"(?m)const\s+(\w+)\s*=\s*z\.object\s*\(\s*\{",
    )
    .unwrap();

    for cap in zod_re.captures_iter(source) {
        let name = &cap[1];
        // Strip trailing "Schema" suffix if present for the definition name
        let def_name = name.strip_suffix("Schema").unwrap_or(name);

        // Find the body of z.object({ ... })
        let start = cap.get(0).unwrap().end() - 1; // position of opening `{`
        if let Some(body) = extract_brace_block(source, start) {
            let properties = parse_zod_properties(&body);

            let mut definitions = IndexMap::new();
            let mut def = serde_json::Map::new();
            def.insert("type".to_string(), serde_json::json!("object"));
            if !properties.is_empty() {
                def.insert(
                    "properties".to_string(),
                    serde_json::Value::Object(properties),
                );
            }
            definitions.insert(
                def_name.to_string(),
                serde_json::Value::Object(def),
            );

            schemas.push(SchemaSpec {
                schema: None,
                name: def_name.to_string(),
                description: None,
                definitions,
                meta: None,
            });
        }
    }
}

/// Parse Zod property declarations inside z.object body.
///
/// Handles: `fieldName: z.string()`, `z.number()`, `z.boolean()`,
/// `.email()`, `.min(N)`, `.max(N)`, `.optional()`
fn parse_zod_properties(body: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut props = serde_json::Map::new();

    let prop_re = Regex::new(r"(\w+)\s*:\s*(z\.\w+\([^)]*\)(?:\.\w+\([^)]*\))*)").unwrap();
    let min_re = Regex::new(r"\.min\((\d+)\)").unwrap();
    let max_re = Regex::new(r"\.max\((\d+)\)").unwrap();

    for cap in prop_re.captures_iter(body) {
        let field_name = &cap[1];
        let zod_chain = &cap[2];

        let mut prop = serde_json::Map::new();

        // Base type
        if zod_chain.contains("z.string") {
            prop.insert("type".to_string(), serde_json::json!("string"));
        } else if zod_chain.contains("z.number") {
            prop.insert("type".to_string(), serde_json::json!("number"));
        } else if zod_chain.contains("z.boolean") {
            prop.insert("type".to_string(), serde_json::json!("boolean"));
        } else if zod_chain.contains("z.array") {
            prop.insert("type".to_string(), serde_json::json!("array"));
        } else {
            prop.insert("type".to_string(), serde_json::json!("string"));
        }

        // Format: .email()
        if zod_chain.contains(".email()") {
            prop.insert("format".to_string(), serde_json::json!("email"));
        }
        if zod_chain.contains(".uuid()") {
            prop.insert("format".to_string(), serde_json::json!("uuid"));
        }
        if zod_chain.contains(".url()") {
            prop.insert("format".to_string(), serde_json::json!("uri"));
        }

        // Constraints: .min(N), .max(N)
        if let Some(min_cap) = min_re.captures(zod_chain) {
            let n: u64 = min_cap[1].parse().unwrap_or(0);
            if prop.get("type").and_then(|v| v.as_str()) == Some("string") {
                prop.insert("minLength".to_string(), serde_json::json!(n));
            } else {
                prop.insert("minimum".to_string(), serde_json::json!(n));
            }
        }
        if let Some(max_cap) = max_re.captures(zod_chain) {
            let n: u64 = max_cap[1].parse().unwrap_or(0);
            if prop.get("type").and_then(|v| v.as_str()) == Some("string") {
                prop.insert("maxLength".to_string(), serde_json::json!(n));
            } else {
                prop.insert("maximum".to_string(), serde_json::json!(n));
            }
        }

        // Nullable: .optional() or .nullable()
        if zod_chain.contains(".optional()") || zod_chain.contains(".nullable()") {
            prop.insert("nullable".to_string(), serde_json::json!(true));
        }

        props.insert(
            field_name.to_string(),
            serde_json::Value::Object(prop),
        );
    }

    props
}

/// Extract TypeScript interfaces: `interface Xxx { ... }`
fn extract_ts_interfaces(
    source: &str,
    schemas: &mut Vec<SchemaSpec>,
    _warnings: &mut Vec<String>,
) {
    let iface_re = Regex::new(r"(?m)(?:export\s+)?interface\s+(\w+)\s*\{").unwrap();

    for cap in iface_re.captures_iter(source) {
        let name = &cap[1];
        let start = cap.get(0).unwrap().end() - 1;
        if let Some(body) = extract_brace_block(source, start) {
            let properties = parse_ts_properties(&body);

            let mut definitions = IndexMap::new();
            let mut def = serde_json::Map::new();
            def.insert("type".to_string(), serde_json::json!("object"));
            if !properties.is_empty() {
                def.insert(
                    "properties".to_string(),
                    serde_json::Value::Object(properties),
                );
            }
            definitions.insert(
                name.to_string(),
                serde_json::Value::Object(def),
            );

            schemas.push(SchemaSpec {
                schema: None,
                name: name.to_string(),
                description: None,
                definitions,
                meta: None,
            });
        }
    }
}

/// Extract TypeScript type aliases: `type Xxx = { ... }`
fn extract_ts_type_aliases(
    source: &str,
    schemas: &mut Vec<SchemaSpec>,
    _warnings: &mut Vec<String>,
) {
    let type_re = Regex::new(r"(?m)(?:export\s+)?type\s+(\w+)\s*=\s*\{").unwrap();

    for cap in type_re.captures_iter(source) {
        let name = &cap[1];
        let start = cap.get(0).unwrap().end() - 1;
        if let Some(body) = extract_brace_block(source, start) {
            let properties = parse_ts_properties(&body);

            let mut definitions = IndexMap::new();
            let mut def = serde_json::Map::new();
            def.insert("type".to_string(), serde_json::json!("object"));
            if !properties.is_empty() {
                def.insert(
                    "properties".to_string(),
                    serde_json::Value::Object(properties),
                );
            }
            definitions.insert(
                name.to_string(),
                serde_json::Value::Object(def),
            );

            schemas.push(SchemaSpec {
                schema: None,
                name: name.to_string(),
                description: None,
                definitions,
                meta: None,
            });
        }
    }
}

/// Parse TS interface/type properties: `fieldName: type;` or `fieldName?: type;`
fn parse_ts_properties(body: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut props = serde_json::Map::new();

    let prop_re = Regex::new(r"(\w+)(\?)?:\s*(\w+)").unwrap();

    for cap in prop_re.captures_iter(body) {
        let field = &cap[1];
        let optional = cap.get(2).is_some();
        let ts_type = &cap[3];

        let mut prop = serde_json::Map::new();
        let json_type = ts_type_to_json_type(ts_type);
        prop.insert("type".to_string(), serde_json::json!(json_type));
        if optional {
            prop.insert("nullable".to_string(), serde_json::json!(true));
        }

        props.insert(field.to_string(), serde_json::Value::Object(prop));
    }

    props
}

/// Map TypeScript type names to JSON Schema types.
fn ts_type_to_json_type(ts_type: &str) -> &str {
    match ts_type {
        "string" => "string",
        "number" => "number",
        "boolean" => "boolean",
        "any" => "object",
        _ => "string",
    }
}

/// Extract a brace-delimited block from source starting at position `start`.
///
/// `start` must point to the opening `{`. Returns the content between the braces.
pub(crate) fn extract_brace_block(source: &str, start: usize) -> Option<String> {
    let bytes = source.as_bytes();
    if start >= bytes.len() || bytes[start] != b'{' {
        return None;
    }

    let mut depth = 0i32;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(source[start + 1..i].to_string());
                }
            }
            b'"' | b'\'' | b'`' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1; // skip escaped char
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_zod_schema() {
        let source = r#"
const UserSchema = z.object({
    email: z.string().email(),
    age: z.number().min(0).max(150),
    name: z.string().min(1).max(100),
    nickname: z.string().optional(),
});
"#;
        let mut warnings = Vec::new();
        let schemas = extract_schemas(source, &mut warnings);
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "User");

        let def = &schemas[0].definitions["User"];
        let props = def["properties"].as_object().unwrap();
        assert_eq!(props["email"]["type"], "string");
        assert_eq!(props["email"]["format"], "email");
        assert_eq!(props["age"]["type"], "number");
        assert_eq!(props["age"]["minimum"], 0);
        assert_eq!(props["age"]["maximum"], 150);
        assert_eq!(props["name"]["minLength"], 1);
        assert_eq!(props["name"]["maxLength"], 100);
        assert_eq!(props["nickname"]["nullable"], true);
    }

    #[test]
    fn test_extract_ts_interface() {
        let source = r#"
interface User {
    id: string;
    email: string;
    age?: number;
}
"#;
        let mut warnings = Vec::new();
        let schemas = extract_schemas(source, &mut warnings);
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "User");

        let def = &schemas[0].definitions["User"];
        let props = def["properties"].as_object().unwrap();
        assert_eq!(props["id"]["type"], "string");
        assert_eq!(props["age"]["nullable"], true);
    }

    #[test]
    fn test_extract_ts_type_alias() {
        let source = r#"
type CreateUserInput = {
    email: string;
    password: string;
};
"#;
        let mut warnings = Vec::new();
        let schemas = extract_schemas(source, &mut warnings);
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "CreateUserInput");
    }

    #[test]
    fn test_extract_brace_block_simple() {
        let s = "{ a: 1, b: 2 }";
        assert_eq!(extract_brace_block(s, 0).unwrap(), " a: 1, b: 2 ");
    }

    #[test]
    fn test_extract_brace_block_nested() {
        let s = "{ a: { b: 1 }, c: 2 }";
        assert_eq!(extract_brace_block(s, 0).unwrap(), " a: { b: 1 }, c: 2 ");
    }

    #[test]
    fn test_zod_uuid_format() {
        let source = r#"
const IdSchema = z.object({
    id: z.string().uuid(),
});
"#;
        let mut warnings = Vec::new();
        let schemas = extract_schemas(source, &mut warnings);
        let def = &schemas[0].definitions["Id"];
        let props = def["properties"].as_object().unwrap();
        assert_eq!(props["id"]["format"], "uuid");
    }
}
