use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// OpenAPI 3.1 Document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenApiDocument {
    pub openapi: String,
    pub info: InfoObject,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<ServerObject>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub paths: IndexMap<String, PathItemObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<ComponentsObject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagObject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<IndexMap<String, Vec<String>>>,
}

/// Info Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoObject {
    pub title: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Server Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerObject {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Path Item Object — maps HTTP methods to operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathItemObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<OperationObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OperationObject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterObject>,
}

/// Operation Object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBodyObject>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub responses: IndexMap<String, ResponseObject>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<IndexMap<String, Vec<String>>>,
}

/// Parameter Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterObject {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Request Body Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBodyObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    pub content: IndexMap<String, MediaTypeObject>,
}

/// Media Type Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaTypeObject {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Response Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseObject {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<IndexMap<String, MediaTypeObject>>,
}

/// Components Object
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentsObject {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub schemas: IndexMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub security_schemes: IndexMap<String, SecuritySchemeObject>,
}

/// Security Scheme Object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySchemeObject {
    #[serde(rename = "type")]
    pub scheme_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Tag Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagObject {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
