import type { TreeNode, HttpMethod } from "../ipc/types";

export interface RouteTreeNode {
  segment: string;
  fullPath: string;
  specNode?: TreeNode;
  methods: HttpMethod[];
  children: RouteTreeNode[];
}

/**
 * Build a hierarchical tree from flat route nodes.
 * Each route has a path like "/users/:id/posts".
 * We split by "/" and build a trie.
 */
export function buildRouteTree(routeNodes: TreeNode[]): RouteTreeNode[] {
  const root: RouteTreeNode[] = [];

  for (const node of routeNodes) {
    const path = node.label.startsWith("/") ? node.label : `/${node.label}`;
    const segments = path.split("/").filter(Boolean);

    let currentLevel = root;
    let currentPath = "";

    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      currentPath += `/${seg}`;
      const isLast = i === segments.length - 1;

      let existing = currentLevel.find((n) => n.segment === seg);
      if (!existing) {
        existing = {
          segment: seg,
          fullPath: currentPath,
          methods: [],
          children: [],
        };
        currentLevel.push(existing);
      }

      if (isLast) {
        existing.specNode = node;
        existing.fullPath = path;
      }

      currentLevel = existing.children;
    }
  }

  return root;
}

/**
 * Extract methods from a route path label.
 * The caller should provide method data from the project tree or specs.
 */
export function getMethodsForRoute(_node: TreeNode): HttpMethod[] {
  return [];
}
