import type { AstNode, Tier } from "../ipc/types";

let nextId = 1;
function uid(): string {
  return `node_${Date.now()}_${nextId++}`;
}

// ---------------------------------------------------------------------------
// Generic factory
// ---------------------------------------------------------------------------

export function createNode(type: string, tier: Tier, extra: Record<string, unknown> = {}): AstNode {
  return { id: uid(), type, tier, ...extra };
}

// ---------------------------------------------------------------------------
// Statement factories
// ---------------------------------------------------------------------------

export function createLetStatement(name = "", value?: AstNode): AstNode {
  return createNode("LetStatement", 0, { name, value: value ?? null, mutable: false });
}

export function createIfStatement(condition?: AstNode, body: AstNode[] = [], elseBody: AstNode[] = []): AstNode {
  return createNode("IfStatement", 0, { condition: condition ?? null, body, elseBody });
}

export function createForStatement(variable = "item", iterable?: AstNode, body: AstNode[] = []): AstNode {
  return createNode("ForStatement", 0, { variable, iterable: iterable ?? null, body });
}

export function createWhileStatement(condition?: AstNode, body: AstNode[] = []): AstNode {
  return createNode("WhileStatement", 0, { condition: condition ?? null, body });
}

export function createReturnStatement(value?: AstNode): AstNode {
  return createNode("ReturnStatement", 0, { value: value ?? null });
}

export function createTryCatchStatement(
  tryBody: AstNode[] = [],
  catchParam = "err",
  catchBody: AstNode[] = [],
): AstNode {
  return createNode("TryCatchStatement", 0, { tryBody, catchParam, catchBody });
}

export function createThrowStatement(value?: AstNode): AstNode {
  return createNode("ThrowStatement", 0, { value: value ?? null });
}

export function createMatchStatement(subject?: AstNode, arms: { pattern: string; body: AstNode[] }[] = []): AstNode {
  return createNode("MatchStatement", 0, { subject: subject ?? null, arms });
}

// ---------------------------------------------------------------------------
// Expression factories
// ---------------------------------------------------------------------------

export function createLiteral(value: unknown = ""): AstNode {
  return createNode("Literal", 0, { value });
}

export function createIdentifier(name = ""): AstNode {
  return createNode("Identifier", 0, { name });
}

export function createBinaryExpression(operator = "+", left?: AstNode, right?: AstNode): AstNode {
  return createNode("BinaryExpression", 0, { operator, left: left ?? null, right: right ?? null });
}

export function createCallExpression(callee?: AstNode, args: AstNode[] = []): AstNode {
  return createNode("CallExpression", 0, { callee: callee ?? null, arguments: args });
}

export function createMemberExpression(object?: AstNode, property = ""): AstNode {
  return createNode("MemberExpression", 0, { object: object ?? null, property });
}

export function createObjectExpression(properties: { key: string; value: AstNode }[] = []): AstNode {
  return createNode("ObjectExpression", 0, { properties });
}

export function createArrayExpression(elements: AstNode[] = []): AstNode {
  return createNode("ArrayExpression", 0, { elements });
}

export function createTemplateLiteral(parts: (string | AstNode)[] = []): AstNode {
  return createNode("TemplateLiteral", 0, { parts });
}

// ---------------------------------------------------------------------------
// Domain (Tier 1) factories
// ---------------------------------------------------------------------------

export function createDbQuery(model = "", operation = "findMany", where?: AstNode): AstNode {
  return createNode("DbQuery", 1, { model, operation, where: where ?? null });
}

export function createDbMutate(model = "", operation = "create", data?: AstNode): AstNode {
  return createNode("DbMutate", 1, { model, operation, data: data ?? null });
}

export function createHttpRespond(status = 200, body?: AstNode, headers?: AstNode): AstNode {
  return createNode("HttpRespond", 1, { status, body: body ?? null, headers: headers ?? null });
}

export function createCtxGet(key = ""): AstNode {
  return createNode("CtxGet", 1, { key });
}

export function createValidate(schema = "", value?: AstNode): AstNode {
  return createNode("Validate", 1, { schema, value: value ?? null });
}

export function createHashPassword(value?: AstNode): AstNode {
  return createNode("HashPassword", 1, { value: value ?? null });
}

export function createSignToken(payload?: AstNode, secret?: AstNode): AstNode {
  return createNode("SignToken", 1, { payload: payload ?? null, secret: secret ?? null });
}

// ---------------------------------------------------------------------------
// Bridge (Tier 3)
// ---------------------------------------------------------------------------

export function createNativeBridge(language = "typescript", module = "", method = "", args: AstNode[] = []): AstNode {
  return createNode("NativeBridge", 3, { language, module, method, arguments: args });
}

// ---------------------------------------------------------------------------
// Tree utilities
// ---------------------------------------------------------------------------

export function getNodeTier(node: AstNode): Tier {
  return node.tier;
}

/**
 * Remove a node by id from an array of nodes (searches recursively).
 * Returns a new array (immutable).
 */
export function removeNode(nodes: AstNode[], nodeId: string): AstNode[] {
  return nodes
    .filter((n) => (n as Record<string, unknown>).id !== nodeId)
    .map((n) => removeNodeFromChildren(n, nodeId));
}

function removeNodeFromChildren(node: AstNode, nodeId: string): AstNode {
  const result: Record<string, unknown> = { ...node };
  for (const key of Object.keys(result)) {
    const val = result[key];
    if (Array.isArray(val) && val.length > 0 && val[0] && typeof val[0] === "object" && "type" in val[0]) {
      result[key] = removeNode(val as AstNode[], nodeId);
    } else if (val && typeof val === "object" && "type" in (val as Record<string, unknown>)) {
      if ((val as Record<string, unknown>).id === nodeId) {
        result[key] = null;
      }
    }
  }
  return result as AstNode;
}

/**
 * Insert a node into a parent's child array at position `index`.
 * `parentId` is the id of the parent node; the child array is searched by
 * looking at known container keys (body, tryBody, catchBody, elements, etc.).
 */
export function insertNode(
  nodes: AstNode[],
  parentId: string | null,
  node: AstNode,
  index?: number,
): AstNode[] {
  // Top-level insert
  if (parentId === null) {
    const idx = index ?? nodes.length;
    const copy = [...nodes];
    copy.splice(idx, 0, node);
    return copy;
  }

  return nodes.map((n) => {
    if ((n as Record<string, unknown>).id === parentId) {
      return insertIntoFirstChildArray(n, node, index);
    }
    return insertNodeRecursive(n, parentId, node, index);
  });
}

const CONTAINER_KEYS = ["body", "tryBody", "catchBody", "elseBody", "elements", "arguments", "properties", "arms"];

function insertIntoFirstChildArray(parent: AstNode, node: AstNode, index?: number): AstNode {
  const result: Record<string, unknown> = { ...parent };
  for (const key of CONTAINER_KEYS) {
    if (Array.isArray(result[key])) {
      const arr = [...(result[key] as AstNode[])];
      const idx = index ?? arr.length;
      arr.splice(idx, 0, node);
      result[key] = arr;
      return result as AstNode;
    }
  }
  return result as AstNode;
}

function insertNodeRecursive(node: AstNode, parentId: string, child: AstNode, index?: number): AstNode {
  const result: Record<string, unknown> = { ...node };
  for (const key of Object.keys(result)) {
    const val = result[key];
    if (Array.isArray(val) && val.length > 0 && val[0] && typeof val[0] === "object" && "type" in val[0]) {
      result[key] = insertNode(val as AstNode[], parentId, child, index);
    }
  }
  return result as AstNode;
}

/**
 * Update a single property on a node found by its `id`.
 */
export function updateNodeProperty(nodes: AstNode[], nodeId: string, key: string, value: unknown): AstNode[] {
  return nodes.map((n) => {
    if ((n as Record<string, unknown>).id === nodeId) {
      return { ...n, [key]: value };
    }
    const result: Record<string, unknown> = { ...n };
    for (const k of Object.keys(result)) {
      const v = result[k];
      if (Array.isArray(v) && v.length > 0 && v[0] && typeof v[0] === "object" && "type" in v[0]) {
        result[k] = updateNodeProperty(v as AstNode[], nodeId, key, value);
      }
    }
    return result as AstNode;
  });
}
