import type {
  ProjectTree,
  TreeNode,
  RouteSpec,
  SchemaSpec,
  ModelSpec,
  MiddlewareSpec,
  HandlerSpec,
} from "./types";

// ---------------------------------------------------------------------------
// Default spec factories (mirrors src/utils/defaults.ts)
// ---------------------------------------------------------------------------

function defaultRoute(path: string): RouteSpec {
  return {
    path,
    methods: {
      GET: { handler: { ref: "" } },
    },
  };
}

function defaultSchema(name: string): SchemaSpec {
  return {
    name,
    definitions: {
      [name]: { type: "object", properties: {} },
    },
  };
}

function defaultModel(name: string): ModelSpec {
  return {
    name,
    tableName: name.toLowerCase() + "s",
    columns: {
      id: { type: "uuid", primaryKey: true },
      createdAt: { type: "timestamp", default: "now()" },
      updatedAt: { type: "timestamp", default: "now()", onUpdate: "now()" },
    },
  };
}

function defaultMiddleware(name: string): MiddlewareSpec {
  return {
    name,
    type: "request",
  };
}

function defaultHandler(name: string): HandlerSpec {
  return {
    name,
    async: true,
    body: [],
  };
}

// ---------------------------------------------------------------------------
// Mock project tree
// ---------------------------------------------------------------------------

function buildMockTree(): ProjectTree {
  const routeChildren: TreeNode[] = [
    { id: "r1", label: "/users", kind: "route", path: "routes/users.route.json", children: [] },
    { id: "r2", label: "/users/:id", kind: "route", path: "routes/users-id.route.json", children: [] },
    { id: "r3", label: "/posts", kind: "route", path: "routes/posts.route.json", children: [] },
  ];

  const schemaChildren: TreeNode[] = [
    { id: "s1", label: "User", kind: "schema", path: "schemas/User.schema.json", children: [] },
    { id: "s2", label: "Post", kind: "schema", path: "schemas/Post.schema.json", children: [] },
  ];

  const modelChildren: TreeNode[] = [
    { id: "m1", label: "User", kind: "model", path: "models/User.model.json", children: [] },
  ];

  const middlewareChildren: TreeNode[] = [
    { id: "mw1", label: "auth", kind: "middleware", path: "middleware/auth.middleware.json", children: [] },
  ];

  const handlerChildren: TreeNode[] = [
    { id: "h1", label: "getUsers", kind: "handler", path: "handlers/getUsers.handler.json", children: [] },
  ];

  return {
    name: "demo-api",
    path: "/mock/demo-api",
    config: { target: { language: "typescript", framework: "express" } },
    nodes: [
      { id: "section:routes", label: "Routes", kind: "section", children: routeChildren },
      { id: "section:schemas", label: "Schemas", kind: "section", children: schemaChildren },
      { id: "section:models", label: "Models", kind: "section", children: modelChildren },
      { id: "section:middleware", label: "Middleware", kind: "section", children: middlewareChildren },
      { id: "section:handlers", label: "Handlers", kind: "section", children: handlerChildren },
    ],
  };
}

// ---------------------------------------------------------------------------
// In-memory spec storage
// ---------------------------------------------------------------------------

const specStore = new Map<string, unknown>();

// Seed default specs
specStore.set("routes/users.route.json", defaultRoute("/users"));
specStore.set("routes/users-id.route.json", defaultRoute("/users/:id"));
specStore.set("routes/posts.route.json", defaultRoute("/posts"));
specStore.set("schemas/User.schema.json", defaultSchema("User"));
specStore.set("schemas/Post.schema.json", defaultSchema("Post"));
specStore.set("models/User.model.json", defaultModel("User"));
specStore.set("middleware/auth.middleware.json", defaultMiddleware("auth"));
specStore.set("handlers/getUsers.handler.json", defaultHandler("getUsers"));

let mockTree = buildMockTree();

// ---------------------------------------------------------------------------
// Tree rebuild helper — syncs tree nodes with specStore keys
// ---------------------------------------------------------------------------

function rebuildTree(): void {
  const sectionMap: Record<string, { prefix: string; kind: string; label: string }> = {
    "section:routes": { prefix: "routes/", kind: "route", label: "Routes" },
    "section:schemas": { prefix: "schemas/", kind: "schema", label: "Schemas" },
    "section:models": { prefix: "models/", kind: "model", label: "Models" },
    "section:middleware": { prefix: "middleware/", kind: "middleware", label: "Middleware" },
    "section:handlers": { prefix: "handlers/", kind: "handler", label: "Handlers" },
  };

  const nodes: TreeNode[] = [];
  for (const [sectionId, info] of Object.entries(sectionMap)) {
    const children: TreeNode[] = [];
    for (const key of specStore.keys()) {
      if (key.startsWith(info.prefix)) {
        const spec = specStore.get(key) as Record<string, unknown> | undefined;
        const name = (spec && (spec.name ?? spec.path)) as string | undefined;
        const label = name ?? key.replace(info.prefix, "").replace(/\.\w+\.json$/, "");
        children.push({
          id: key,
          label,
          kind: info.kind,
          path: key,
          children: [],
        });
      }
    }
    nodes.push({ id: sectionId, label: info.label, kind: "section", children });
  }
  mockTree = { ...mockTree, nodes };
}

// ---------------------------------------------------------------------------
// Mock CRUD functions
// ---------------------------------------------------------------------------

function mockReadSpec(filePath: string): unknown {
  const spec = specStore.get(filePath);
  if (!spec) throw new Error(`[mock] spec not found: ${filePath}`);
  return structuredClone(spec);
}

function mockWriteSpec(filePath: string, value: unknown): void {
  specStore.set(filePath, structuredClone(value));
  rebuildTree();
}

function mockDeleteSpec(filePath: string): void {
  specStore.delete(filePath);
  rebuildTree();
}

function mockMoveSpec(oldPath: string, newPath: string): void {
  const spec = specStore.get(oldPath);
  if (!spec) throw new Error(`[mock] spec not found: ${oldPath}`);
  specStore.delete(oldPath);
  specStore.set(newPath, spec);
  rebuildTree();
}

function mockGetProjectTree(): ProjectTree {
  return structuredClone(mockTree);
}

function mockOpenProject(): ProjectTree {
  return mockGetProjectTree();
}

function mockCreateProject(): ProjectTree {
  return mockGetProjectTree();
}

function mockCloseProject(): void {
  // no-op
}

// ---------------------------------------------------------------------------
// mockInvoke — routes IPC command names to mock functions
// ---------------------------------------------------------------------------

export async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  switch (cmd) {
    case "create_project":
      return mockCreateProject() as T;
    case "open_project":
      return mockOpenProject() as T;
    case "close_project":
      return mockCloseProject() as unknown as T;
    case "get_project_tree":
      return mockGetProjectTree() as T;

    case "read_route":
    case "read_schema":
    case "read_model":
    case "read_middleware":
    case "read_handler":
      return mockReadSpec(args?.filePath as string) as T;

    case "write_route":
    case "write_schema":
    case "write_model":
    case "write_middleware":
    case "write_handler":
      mockWriteSpec(args?.filePath as string, args?.value);
      return undefined as T;

    case "delete_route":
    case "delete_schema":
    case "delete_model":
    case "delete_middleware":
    case "delete_handler":
      mockDeleteSpec(args?.filePath as string);
      return undefined as T;

    case "move_route":
    case "move_schema":
    case "move_model":
    case "move_middleware":
    case "move_handler":
      mockMoveSpec(args?.oldFilePath as string, args?.newFilePath as string);
      return undefined as T;

    case "validate_project":
      return { ok: true, errors: [] } as T;
    case "detect_runtimes":
      return [] as T;
    case "get_server_status":
      return "stopped" as T;
    case "run_preflight":
      return { ok: true, checks: [] } as T;
    case "start_server":
      return 0 as T;
    case "stop_server":
    case "restart_server":
      return undefined as T;
    case "preview_code":
      return {} as T;
    case "generate_project":
      return { outputDir: "/mock/output", fileCount: 0 } as T;
    case "export_openapi":
      return "{}" as T;
    case "import_openapi":
    case "import_from_code":
      return { filesCreated: [], warnings: [] } as T;
    case "apply_hmu":
      return { id: "mock", status: "success", applied: [], failed: [], requiresRestart: false } as T;

    default:
      console.warn(`[mock] unhandled command: ${cmd}`);
      return undefined as T;
  }
}
