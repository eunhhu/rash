import type {
  RouteSpec,
  SchemaSpec,
  ModelSpec,
  MiddlewareSpec,
  HandlerSpec,
  MiddlewareType,
} from "../ipc/types";

export function defaultRoute(path: string): RouteSpec {
  return {
    path,
    methods: {
      GET: {
        handler: { ref: "" },
      },
    },
  };
}

export function defaultSchema(name: string): SchemaSpec {
  return {
    name,
    definitions: {
      [name]: { type: "object", properties: {} },
    },
  };
}

export function defaultModel(name: string): ModelSpec {
  const tableName = name.toLowerCase() + "s";
  return {
    name,
    tableName,
    columns: {
      id: {
        type: "uuid",
        primaryKey: true,
      },
      createdAt: {
        type: "timestamp",
        default: "now()",
      },
      updatedAt: {
        type: "timestamp",
        default: "now()",
        onUpdate: "now()",
      },
    },
  };
}

export function defaultMiddleware(
  name: string,
  type: MiddlewareType = "request",
): MiddlewareSpec {
  return {
    name,
    type,
  };
}

export function defaultHandler(name: string): HandlerSpec {
  return {
    name,
    async: true,
    body: [],
  };
}
