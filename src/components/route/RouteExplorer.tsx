import { Component, For, Show, createMemo } from "solid-js";
import { useProjectStore } from "../../stores/projectStore";
import type { TreeNode, HttpMethod } from "../../ipc/types";
import { Badge } from "../common/Badge";

interface RouteExplorerProps {
  onSelectRoute: (path: string, filePath: string) => void;
}

interface RouteItem {
  id: string;
  path: string;
  filePath: string;
  methods: HttpMethod[];
  children: RouteItem[];
}

/** Build a flat route list from the project tree nodes of kind "route". */
function collectRoutes(nodes: TreeNode[]): RouteItem[] {
  const items: RouteItem[] = [];
  for (const node of nodes) {
    if (node.kind === "route") {
      items.push({
        id: node.id,
        path: node.label,
        filePath: node.path ?? "",
        methods: extractMethods(node),
        children: collectRoutes(node.children),
      });
    } else if (node.children.length > 0) {
      items.push(...collectRoutes(node.children));
    }
  }
  return items;
}

function extractMethods(node: TreeNode): HttpMethod[] {
  const methods: HttpMethod[] = [];
  for (const child of node.children) {
    if (child.kind === "method") {
      methods.push(child.label as HttpMethod);
    }
  }
  return methods;
}

const METHOD_ORDER: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

const RouteRow: Component<{ route: RouteItem; onSelect: (path: string, filePath: string) => void }> = (props) => {
  const sorted = createMemo(() =>
    [...props.route.methods].sort((a, b) => METHOD_ORDER.indexOf(a) - METHOD_ORDER.indexOf(b)),
  );

  return (
    <div class="route-explorer-item" onClick={() => props.onSelect(props.route.path, props.route.filePath)}>
      <div class="route-explorer-methods">
        <For each={sorted()}>
          {(method) => <Badge variant="method" value={method} />}
        </For>
      </div>
      <span class="route-explorer-path">{props.route.path}</span>
    </div>
  );
};

export const RouteExplorer: Component<RouteExplorerProps> = (props) => {
  const { project } = useProjectStore();

  const routes = createMemo(() => {
    const tree = project();
    if (!tree) return [];
    return collectRoutes(tree.nodes);
  });

  return (
    <div class="route-explorer">
      <div class="route-explorer-header">
        <span>Routes</span>
      </div>
      <div class="route-explorer-list">
        <Show when={routes().length > 0} fallback={<div class="route-explorer-empty">No routes defined</div>}>
          <For each={routes()}>
            {(route) => <RouteRow route={route} onSelect={props.onSelectRoute} />}
          </For>
        </Show>
      </div>

      <style>{`
        .route-explorer {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .route-explorer-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 8px 12px;
          font-size: 11px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.5px;
          color: var(--rash-text-muted);
          border-bottom: 1px solid var(--rash-border);
        }
        .route-explorer-list {
          flex: 1;
          overflow-y: auto;
          padding: 4px;
        }
        .route-explorer-item {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 6px 8px;
          cursor: pointer;
          border-radius: var(--rash-radius-sm);
          transition: background 0.1s ease;
        }
        .route-explorer-item:hover {
          background: var(--rash-surface);
        }
        .route-explorer-methods {
          display: flex;
          gap: 4px;
          flex-shrink: 0;
        }
        .route-explorer-path {
          font-family: var(--rash-font);
          font-size: 12px;
          color: var(--rash-text);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .route-explorer-empty {
          padding: 16px;
          text-align: center;
          color: var(--rash-text-muted);
          font-size: 12px;
        }
      `}</style>
    </div>
  );
};
