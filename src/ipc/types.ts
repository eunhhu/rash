// ---------------------------------------------------------------------------
// Enums — mirror Rust serde rename conventions
// ---------------------------------------------------------------------------

export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

export type Language = "typescript" | "rust" | "python" | "go";

export type Framework =
  | "express"
  | "fastify"
  | "hono"
  | "elysia"
  | "nestjs"
  | "actix"
  | "axum"
  | "rocket"
  | "fastapi"
  | "django"
  | "flask"
  | "gin"
  | "echo"
  | "fiber";

export type Runtime = "bun" | "node" | "deno" | "cargo" | "python" | "go";

export type DatabaseType = "postgresql" | "mysql" | "sqlite" | "mongodb";

export type Orm = "prisma" | "typeorm" | "seaorm" | "sqlalchemy" | "django-orm" | "gorm";

export type Protocol = "http" | "https";

export type Severity = "error" | "warning" | "info";

export type RelationType = "hasOne" | "hasMany" | "belongsTo" | "manyToMany";

export type MiddlewareType = "request" | "response" | "error" | "composed";

/** AST node portability tier (0-3) */
export type Tier = 0 | 1 | 2 | 3;

// ---------------------------------------------------------------------------
// Common primitives
// ---------------------------------------------------------------------------

/** Reference to another spec element */
export interface Ref {
  ref: string;
  config?: unknown;
}

/** Type reference — simple string or a $ref */
export type TypeRef =
  | string
  | Ref
  | { ref: string; nullable?: boolean };

/** Metadata attached to spec files */
export interface Meta {
  createdAt?: string;
  updatedAt?: string;
  rashVersion?: string;
  lastMigratedFrom?: string;
}

// ---------------------------------------------------------------------------
// Project tree (returned by open_project / get_project_tree)
// ---------------------------------------------------------------------------

export interface ProjectTree {
  name: string;
  path: string;
  config: unknown;
  nodes: TreeNode[];
}

export interface TreeNode {
  id: string;
  label: string;
  kind: string;
  path?: string;
  children: TreeNode[];
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

export interface ValidationReport {
  ok: boolean;
  errors: ErrorEntry[];
}

export interface ErrorEntry {
  code: string;
  severity: Severity;
  message: string;
  file: string;
  path: string;
  suggestion?: string;
}

// ---------------------------------------------------------------------------
// Route spec
// ---------------------------------------------------------------------------

export interface RouteSpec {
  $schema?: string;
  path: string;
  description?: string;
  params?: Record<string, ParamSpec>;
  methods: Partial<Record<HttpMethod, EndpointSpec>>;
  tags?: string[];
  meta?: Meta;
}

export interface ParamSpec {
  type: string;
  format?: string;
  description?: string;
}

export interface EndpointSpec {
  operationId?: string;
  summary?: string;
  handler: Ref;
  middleware?: Ref[];
  request?: RequestSpec;
  response?: Record<string, ResponseSpec>;
}

export interface RequestSpec {
  query?: Ref;
  body?: RequestBodySpec;
  headers?: Record<string, unknown>;
}

export interface RequestBodySpec {
  ref: string;
  contentType?: string;
}

export interface ResponseSpec {
  description?: string;
  schema?: Ref;
}

// ---------------------------------------------------------------------------
// Schema spec
// ---------------------------------------------------------------------------

export interface SchemaSpec {
  $schema?: string;
  name: string;
  description?: string;
  definitions: Record<string, unknown>;
  meta?: Meta;
}

// ---------------------------------------------------------------------------
// Model spec
// ---------------------------------------------------------------------------

export interface ModelSpec {
  $schema?: string;
  name: string;
  description?: string;
  tableName?: string;
  columns: Record<string, ColumnSpec>;
  relations?: Record<string, RelationSpec>;
  indexes?: IndexSpec[];
  hooks?: Record<string, Ref>;
  meta?: Meta;
}

export interface ColumnSpec {
  type: string;
  primaryKey?: boolean;
  unique?: boolean;
  nullable?: boolean;
  index?: boolean;
  default?: string;
  onUpdate?: string;
  values?: string[];
}

export interface RelationSpec {
  type: RelationType;
  target: string;
  foreignKey: string;
}

export interface IndexSpec {
  columns: string[];
  unique?: boolean;
  where?: string;
}

// ---------------------------------------------------------------------------
// Middleware spec
// ---------------------------------------------------------------------------

export interface MiddlewareSpec {
  $schema?: string;
  name: string;
  description?: string;
  type: MiddlewareType;
  config?: unknown;
  handler?: Ref;
  provides?: Record<string, unknown>;
  errors?: Record<string, MiddlewareError>;
  compose?: Ref[];
  shortCircuit?: boolean;
  meta?: Meta;
}

export interface MiddlewareError {
  status: number;
  message: string;
}

// ---------------------------------------------------------------------------
// Handler spec
// ---------------------------------------------------------------------------

export interface HandlerSpec {
  $schema?: string;
  name: string;
  description?: string;
  async?: boolean;
  params?: Record<string, HandlerParam>;
  returnType?: TypeRef;
  body: AstNode[];
  meta?: HandlerMeta;
}

export interface HandlerParam {
  type: string;
  description?: string;
}

export interface HandlerMeta {
  maxTier?: number;
  languages?: Language[];
  bridges?: string[];
  createdAt?: string;
  updatedAt?: string;
  rashVersion?: string;
  lastMigratedFrom?: string;
}

// ---------------------------------------------------------------------------
// AST nodes (simplified discriminated union)
// ---------------------------------------------------------------------------

/**
 * Simplified AST node representation for the frontend.
 *
 * The Rust side uses a tagged enum (`#[serde(tag = "type")]`) producing
 * objects like `{ type: "LetStatement", tier: 0, name: "x", value: {...} }`.
 * We model this as a base interface plus a string-literal `type` discriminator.
 * Fields vary per node type; the frontend treats them loosely via `AstNode`.
 */
export interface AstNode {
  type: string;
  tier: Tier;
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// IPC command argument types
// ---------------------------------------------------------------------------

export interface CreateProjectArgs {
  name: string;
  path: string;
  language: string;
  framework: string;
  runtime: string;
}

export interface PreviewCodeArgs {
  language: Language;
  framework: Framework;
}

// ---------------------------------------------------------------------------
// OpenAPI import/export
// ---------------------------------------------------------------------------

export interface ImportResult {
  filesCreated: string[];
  warnings: string[];
}
