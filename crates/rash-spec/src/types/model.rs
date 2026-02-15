use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::common::{Meta, Ref};

/// Model specification (*.model.json) — Database table/collection definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpec {
    /// JSON Schema reference
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Model name (e.g., "User")
    pub name: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Database table name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,

    /// Column definitions
    pub columns: IndexMap<String, ColumnSpec>,

    /// Relation definitions
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub relations: IndexMap<String, RelationSpec>,

    /// Index definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<IndexSpec>,

    /// Model hooks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<IndexMap<String, Ref>>,

    /// Metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

/// Column specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnSpec {
    /// Column type (e.g., "uuid", "varchar(255)", "timestamp")
    #[serde(rename = "type")]
    pub col_type: String,

    /// Whether this is a primary key
    #[serde(default, skip_serializing_if = "is_false")]
    pub primary_key: bool,

    /// Whether values must be unique
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique: bool,

    /// Whether NULL is allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,

    /// Whether to create an index
    #[serde(default, skip_serializing_if = "is_false")]
    pub index: bool,

    /// Default value expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// On-update expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<String>,

    /// Enum values (for "enum" type)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

fn is_false(v: &bool) -> bool {
    !(*v)
}

/// Relation specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationSpec {
    /// Relation type
    #[serde(rename = "type")]
    pub relation_type: RelationType,

    /// Target model name
    pub target: String,

    /// Foreign key column name
    pub foreign_key: String,
}

/// Types of model relations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationType {
    HasOne,
    HasMany,
    BelongsTo,
    ManyToMany,
}

/// Index specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSpec {
    /// Column names included in the index
    pub columns: Vec<String>,

    /// Whether this is a unique index
    #[serde(default, skip_serializing_if = "is_false")]
    pub unique: bool,

    /// Partial index WHERE clause
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_clause: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_deserialization_from_docs() {
        let json = serde_json::json!({
            "name": "User",
            "description": "사용자 테이블",
            "tableName": "users",
            "columns": {
                "id": {
                    "type": "uuid",
                    "primaryKey": true,
                    "default": "gen_random_uuid()"
                },
                "email": {
                    "type": "varchar(255)",
                    "unique": true,
                    "nullable": false,
                    "index": true
                },
                "role": {
                    "type": "enum",
                    "values": ["admin", "user", "moderator"],
                    "default": "user",
                    "nullable": false
                },
                "createdAt": {
                    "type": "timestamp",
                    "default": "now()",
                    "nullable": false
                }
            },
            "relations": {
                "posts": {
                    "type": "hasMany",
                    "target": "Post",
                    "foreignKey": "authorId"
                },
                "profile": {
                    "type": "hasOne",
                    "target": "Profile",
                    "foreignKey": "userId"
                }
            },
            "indexes": [
                { "columns": ["email"], "unique": true },
                { "columns": ["role", "createdAt"] },
                { "columns": ["deletedAt"], "where": "deletedAt IS NULL" }
            ]
        });

        let model: ModelSpec = serde_json::from_value(json).unwrap();
        assert_eq!(model.name, "User");
        assert_eq!(model.table_name.as_deref(), Some("users"));
        assert_eq!(model.columns.len(), 4);
        assert!(model.columns["id"].primary_key);
        assert!(model.columns["email"].unique);
        assert_eq!(model.relations.len(), 2);
        assert_eq!(
            model.relations["posts"].relation_type,
            RelationType::HasMany
        );
        assert_eq!(model.indexes.len(), 3);
        assert!(model.indexes[0].unique);
        assert_eq!(
            model.indexes[2].where_clause.as_deref(),
            Some("deletedAt IS NULL")
        );
    }
}
