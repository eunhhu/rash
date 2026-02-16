import { Component, For, Show, createSignal, onCleanup, onMount } from "solid-js";
import { FiPlus } from "solid-icons/fi";
import { useProjectStore } from "../../stores/projectStore";
import { useEditorStore } from "../../stores/editorStore";
import { useUiStore } from "../../stores/uiStore";
import { useSpecStore, type SpecKind } from "../../stores/specStore";
import { SpecIcon } from "../common/SpecIcon";
import { SearchInput } from "../common/SearchInput";
import { buildRouteTree, type RouteTreeNode } from "../../utils/routeTree";
import { setupTreeKeyboard, type TreeNode as KbTreeNode } from "../../utils/keyboard";
import { defaultRoute, defaultSchema, defaultModel, defaultMiddleware, defaultHandler } from "../../utils/defaults";
import type { TreeNode, HttpMethod } from "../../ipc/types";
import "./layout.css";

const SIDEBAR_TABS: { kind: string; label: string }[] = [
  { kind: "route", label: "Routes" },
  { kind: "schema", label: "Schemas" },
  { kind: "model", label: "Models" },
  { kind: "middleware", label: "Middleware" },
  { kind: "handler", label: "Handlers" },
];

const SECTION_IDS: Record<string, string> = {
  route: "section:routes",
  schema: "section:schemas",
  model: "section:models",
  middleware: "section:middleware",
  handler: "section:handlers",
};

const VALID_KINDS = new Set(["route", "schema", "model", "middleware", "handler"]);
function isSpecKind(kind: string): kind is SpecKind { return VALID_KINDS.has(kind); }

function getDefaultSpec(kind: SpecKind, name: string): unknown {
  switch (kind) {
    case "route": return defaultRoute(name.startsWith("/") ? name : `/${name}`);
    case "schema": return defaultSchema(name);
    case "model": return defaultModel(name);
    case "middleware": return defaultMiddleware(name);
    case "handler": return defaultHandler(name);
  }
}

function getCreatePlaceholder(kind: string): string {
  switch (kind) { case "route": return "/new-route"; default: return `new-${kind}`; }
}

function nameFromPath(filePath: string): string {
  const file = filePath.split("/").pop() ?? "";
  return file.replace(/\.[^.]+\.json$/, "");
}

// ── Inline Input ──

interface InlineInputProps {
  value: string;
  placeholder: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

const InlineInput: Component<InlineInputProps> = (props) => {
  let inputRef: HTMLInputElement | undefined;

  onMount(() => {
    inputRef?.focus();
    inputRef?.select();
  });

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      const val = inputRef?.value.trim() ?? "";
      if (val) props.onConfirm(val);
      else props.onCancel();
    } else if (e.key === "Escape") {
      e.preventDefault();
      props.onCancel();
    }
  };

  const handleBlur = () => {
    const val = inputRef?.value.trim() ?? "";
    if (val && val !== props.value) props.onConfirm(val);
    else props.onCancel();
  };

  return (
    <input
      ref={inputRef}
      class="carbon-input text-xs w-full mx-1 my-0.5"
      style={{ "padding": "2px 6px", "min-width": "0" }}
      value={props.value}
      placeholder={props.placeholder}
      onKeyDown={handleKeyDown}
      onBlur={handleBlur}
      onClick={(e) => e.stopPropagation()}
    />
  );
};

// ── IconRail ──

const IconRail: Component = () => {
  const ui = useUiStore();
  return (
    <div class="icon-rail">
      <For each={SIDEBAR_TABS}>
        {(tab) => (
          <button
            class={`icon-rail-btn ${ui.activeSidebarTab() === tab.kind ? "active" : ""}`}
            onClick={() => ui.setActiveSidebarTab(tab.kind)}
            title={tab.label}
          >
            <SpecIcon kind={tab.kind} size={18} />
          </button>
        )}
      </For>
    </div>
  );
};

// ── RouteTreeView ──

const RouteTreeView: Component<{
  nodes: RouteTreeNode[];
  depth?: number;
}> = (props) => {
  const editor = useEditorStore();
  const ui = useUiStore();
  const depth = () => props.depth ?? 0;

  return (
    <For each={props.nodes}>
      {(node) => {
        const [expanded, setExpanded] = createSignal(true);
        const hasChildren = () => node.children.length > 0;
        const isParam = () => node.segment.startsWith(":");
        const isActive = () => node.specNode ? editor.activeTabId() === node.specNode.id : false;

        return (
          <>
            <div
              class={`route-tree-node ${isActive() ? "active" : ""}`}
              style={{ "--depth-pad": `${8 + depth() * 16}px` }}
              onClick={() => {
                if (node.specNode) {
                  editor.openTab(node.specNode.id, node.specNode.label, node.specNode.kind, node.specNode.path ?? "");
                  ui.setSelectedNodeIds(new Set([node.specNode.id]));
                  ui.setFocusedNodeId(node.specNode.id);
                }
                if (hasChildren()) setExpanded(p => !p);
              }}
              onContextMenu={(e) => {
                if (node.specNode) {
                  e.preventDefault();
                }
              }}
            >
              <Show when={hasChildren()}>
                <span
                  class="text-rash-text-muted transition-transform"
                  style={{ "font-size": "10px", transform: expanded() ? "none" : "rotate(-90deg)" }}
                >
                  &#9660;
                </span>
              </Show>
              <Show when={!hasChildren()}>
                <span style={{ width: "10px" }} />
              </Show>
              <span class={`segment ${isParam() ? "param" : ""}`}>
                /{node.segment}
              </span>
              <Show when={node.methods.length > 0}>
                <div class="route-tree-methods">
                  <For each={node.methods}>
                    {(m) => (
                      <span class={`method-badge-sm ${m.toLowerCase()}`}>
                        {m.substring(0, 1)}
                      </span>
                    )}
                  </For>
                </div>
              </Show>
            </div>
            <Show when={hasChildren() && expanded()}>
              <RouteTreeView nodes={node.children} depth={depth() + 1} />
            </Show>
          </>
        );
      }}
    </For>
  );
};

// ── ListPanel ──

const ListPanel: Component = () => {
  const project = useProjectStore();
  const editor = useEditorStore();
  const ui = useUiStore();
  const spec = useSpecStore();
  let panelRef: HTMLDivElement | undefined;

  const activeKind = () => ui.activeSidebarTab();
  const activeLabel = () => SIDEBAR_TABS.find(t => t.kind === activeKind())?.label ?? "";

  const nodesForKind = (): TreeNode[] => {
    const tree = project.project();
    if (!tree) return [];
    const sectionId = SECTION_IDS[activeKind()];
    const section = tree.nodes.find((n) => n.id === sectionId);
    const nodes = section?.children ?? [];

    const query = ui.searchQuery().toLowerCase();
    if (!query) return nodes;

    const filterNodes = (items: TreeNode[]): TreeNode[] => {
      return items.reduce<TreeNode[]>((acc, node) => {
        const matchesSelf = node.label.toLowerCase().includes(query);
        const filteredChildren = filterNodes(node.children);
        if (matchesSelf || filteredChildren.length > 0) {
          acc.push({ ...node, children: matchesSelf ? node.children : filteredChildren });
        }
        return acc;
      }, []);
    };

    return filterNodes(nodes);
  };

  const routeTree = () => {
    if (activeKind() !== "route") return [];
    return buildRouteTree(nodesForKind());
  };

  // Find node by id across current section
  const findNode = (id: string): TreeNode | null => {
    const nodes = nodesForKind();
    for (const node of nodes) {
      if (node.id === id) return node;
      for (const child of node.children) {
        if (child.id === id) return child;
      }
    }
    return null;
  };

  // Also search all sections for keyboard handler
  const findNodeGlobal = (id: string): TreeNode | null => {
    const tree = project.project();
    if (!tree) return null;
    for (const section of tree.nodes) {
      for (const node of section.children) {
        if (node.id === id) return node;
        for (const child of node.children) {
          if (child.id === id) return child;
        }
      }
    }
    return null;
  };

  // Inline create
  const handleCreateConfirm = async (value: string) => {
    ui.clearEditing();
    const kind = activeKind();
    if (!isSpecKind(kind)) return;

    if (kind === "route") {
      const routePath = value.startsWith("/") ? value : `/${value}`;
      const specName = routePath.replace(/^\//, "").replace(/\//g, "_").replace(/:/g, "") || "new-route";
      const data = defaultRoute(routePath);
      await spec.createSpec(kind, specName, data);
    } else {
      const data = getDefaultSpec(kind, value);
      await spec.createSpec(kind, value, data);
    }
  };

  const handleRenameConfirm = async (_nodeId: string, oldPath: string, newName: string) => {
    ui.clearEditing();
    const kind = activeKind();
    if (!isSpecKind(kind)) return;
    await spec.renameSpec(kind, oldPath, newName);
  };

  const handlePaste = async (targetKind: SpecKind) => {
    const clip = ui.clipboard();
    if (!clip || clip.kind !== targetKind) return;
    for (const srcPath of clip.paths) {
      const srcName = nameFromPath(srcPath);
      if (clip.op === "copy") {
        await spec.duplicateSpec(targetKind, srcPath, `${srcName}-copy`);
      }
    }
    if (clip.op === "cut") ui.setClipboard(null);
  };

  const confirmDelete = (node: TreeNode) => {
    if (!isSpecKind(node.kind)) return;
    ui.setConfirmDialog({
      title: `Delete ${node.label}`,
      message: `Are you sure you want to delete "${node.label}"? This action cannot be undone.`,
      confirmLabel: "Delete",
      danger: true,
      onConfirm: () => {
        spec.deleteSpec(node.kind as SpecKind, node.path ?? "", node.label);
      },
    });
  };

  const handleContextMenu = (e: MouseEvent, node: TreeNode) => {
    e.preventDefault();
    const kind = node.kind;
    if (!isSpecKind(kind)) return;

    if (!ui.selectedNodeIds().has(node.id)) {
      ui.setSelectedNodeIds(new Set([node.id]));
    }
    ui.setFocusedNodeId(node.id);

    ui.setContextMenu({
      x: e.clientX,
      y: e.clientY,
      items: [
        {
          label: "Open",
          shortcut: "Enter",
          action: () => editor.openTab(node.id, node.label, node.kind, node.path ?? ""),
        },
        { label: "", separator: true, action: () => {} },
        {
          label: "Rename",
          shortcut: "F2",
          action: () => ui.startRename(node.id, nameFromPath(node.path ?? node.label)),
        },
        {
          label: "Duplicate",
          action: () => {
            const srcName = nameFromPath(node.path ?? node.label);
            spec.duplicateSpec(kind, node.path ?? "", `${srcName}-copy`);
          },
        },
        { label: "", separator: true, action: () => {} },
        {
          label: "Cut",
          shortcut: "\u2318X",
          action: () => {
            ui.setClipboard({
              ids: [node.id],
              paths: [node.path ?? ""],
              kind,
              op: "cut",
            });
          },
        },
        {
          label: "Copy",
          shortcut: "\u2318C",
          action: () => {
            ui.setClipboard({
              ids: [node.id],
              paths: [node.path ?? ""],
              kind,
              op: "copy",
            });
          },
        },
        ...(ui.clipboard() && ui.clipboard()!.kind === kind
          ? [{
              label: "Paste",
              shortcut: "\u2318V",
              action: () => handlePaste(kind),
            }]
          : []),
        { label: "", separator: true, action: () => {} },
        {
          label: "Delete",
          shortcut: "Del",
          danger: true,
          action: () => confirmDelete(node),
        },
      ],
    });
  };

  const handleItemClick = (node: TreeNode, e: MouseEvent) => {
    if (e.metaKey || e.ctrlKey) {
      ui.setSelectedNodeIds((prev) => {
        const next = new Set(prev);
        if (next.has(node.id)) next.delete(node.id);
        else next.add(node.id);
        return next;
      });
    } else if (e.shiftKey) {
      const nodes = nodesForKind();
      const focused = ui.focusedNodeId();
      if (focused) {
        const ids = nodes.map((n) => n.id);
        const start = ids.indexOf(focused);
        const end = ids.indexOf(node.id);
        if (start !== -1 && end !== -1) {
          const [from, to] = start < end ? [start, end] : [end, start];
          ui.setSelectedNodeIds(new Set(ids.slice(from, to + 1)));
        }
      }
    } else {
      ui.setSelectedNodeIds(new Set([node.id]));
      editor.openTab(node.id, node.label, node.kind, node.path ?? "");
    }
    ui.setFocusedNodeId(node.id);
  };

  // Slow double-click rename detection
  let lastClickId: string | null = null;
  let lastClickTime = 0;

  const handleSlowDoubleClick = (node: TreeNode) => {
    const now = Date.now();
    if (lastClickId === node.id && now - lastClickTime > 300 && now - lastClickTime < 1000) {
      ui.startRename(node.id, nameFromPath(node.path ?? node.label));
    }
    lastClickId = node.id;
    lastClickTime = now;
  };

  // Keyboard handler
  onMount(() => {
    const allVisibleNodes = (): KbTreeNode[] => {
      const nodes = nodesForKind();
      return nodes.map((n) => ({
        id: n.id,
        kind: n.kind,
        path: n.path,
        children: n.children.map((c) => ({ id: c.id, kind: c.kind, path: c.path, children: [] })),
      }));
    };

    const treeHandler = setupTreeKeyboard({
      getFocusedId: () => ui.focusedNodeId(),
      getVisibleNodes: allVisibleNodes,
      getCollapsed: () => new Set<string>(),
      onNavigate: (id) => {
        ui.setFocusedNodeId(id);
        ui.setSelectedNodeIds(new Set([id]));
      },
      onOpen: (id) => {
        const node = findNodeGlobal(id);
        if (node) editor.openTab(node.id, node.label, node.kind, node.path ?? "");
      },
      onDelete: (ids) => {
        const node = findNodeGlobal(ids[0]);
        if (node && isSpecKind(node.kind)) {
          confirmDelete(node);
        }
      },
      onRename: (id) => {
        const node = findNodeGlobal(id);
        if (node) ui.startRename(id, nameFromPath(node.path ?? node.label));
      },
      onToggleCollapse: () => {},
      onCut: () => {
        const selected = ui.selectedNodeIds();
        if (selected.size === 0) return;
        const firstNode = findNodeGlobal([...selected][0]);
        if (!firstNode || !isSpecKind(firstNode.kind)) return;
        const paths = [...selected].map((id) => findNodeGlobal(id)?.path ?? "").filter(Boolean);
        ui.setClipboard({ ids: [...selected], paths, kind: firstNode.kind, op: "cut" });
      },
      onCopy: () => {
        const selected = ui.selectedNodeIds();
        if (selected.size === 0) return;
        const firstNode = findNodeGlobal([...selected][0]);
        if (!firstNode || !isSpecKind(firstNode.kind)) return;
        const paths = [...selected].map((id) => findNodeGlobal(id)?.path ?? "").filter(Boolean);
        ui.setClipboard({ ids: [...selected], paths, kind: firstNode.kind, op: "copy" });
      },
      onPaste: async () => {
        const clip = ui.clipboard();
        if (!clip) return;
        const kind = clip.kind as SpecKind;
        for (const srcPath of clip.paths) {
          const srcName = nameFromPath(srcPath);
          if (clip.op === "copy") {
            await spec.duplicateSpec(kind, srcPath, `${srcName}-copy`);
          }
        }
        if (clip.op === "cut") ui.setClipboard(null);
      },
      getSelectedIds: () => ui.selectedNodeIds(),
    });

    const handler = (e: KeyboardEvent) => {
      if (ui.editingMode()) return;
      if (panelRef?.contains(document.activeElement) || document.activeElement === document.body) {
        treeHandler(e);
      }
    };

    window.addEventListener("keydown", handler);
    onCleanup(() => window.removeEventListener("keydown", handler));
  });

  return (
    <div ref={panelRef} class="list-panel" tabIndex={0}>
      <div class="list-panel-header">
        <h3>{activeLabel()}</h3>
        <button
          class="carbon-btn-icon p-0.5"
          onClick={() => ui.startCreate(activeKind())}
          title={`New ${activeLabel().toLowerCase()}`}
        >
          <FiPlus size={14} />
        </button>
      </div>
      <div class="px-2 py-1.5">
        <SearchInput
          value={ui.searchQuery()}
          onSearch={ui.setSearchQuery}
          placeholder={`Search ${activeLabel().toLowerCase()}...`}
        />
      </div>
      <div class="list-panel-body">
        {/* Route: hierarchical tree */}
        <Show when={activeKind() === "route"}>
          <Show when={routeTree().length > 0} fallback={
            <div class="list-panel-empty">No routes</div>
          }>
            <RouteTreeView nodes={routeTree()} />
          </Show>
        </Show>

        {/* Other kinds: flat list */}
        <Show when={activeKind() !== "route"}>
          <Show when={nodesForKind().length > 0 || ui.creatingInSection() === activeKind()} fallback={
            <Show when={ui.creatingInSection() !== activeKind()}>
              <div class="list-panel-empty">No {activeLabel().toLowerCase()}</div>
            </Show>
          }>
            <For each={nodesForKind()}>
              {(node) => (
                <Show
                  when={ui.editingNodeId() === node.id && ui.editingMode() === "rename"}
                  fallback={
                    <div
                      class={`list-panel-item ${editor.activeTabId() === node.id ? "active" : ""} ${ui.selectedNodeIds().has(node.id) ? "selected" : ""}`}
                      onClick={(e) => {
                        handleItemClick(node, e);
                        handleSlowDoubleClick(node);
                      }}
                      onContextMenu={(e) => handleContextMenu(e, node)}
                    >
                      <SpecIcon kind={node.kind} size={14} class="flex-shrink-0" />
                      <span class="overflow-hidden text-ellipsis whitespace-nowrap">{node.label}</span>
                    </div>
                  }
                >
                  <div class="flex items-center gap-1.5 py-0.5 px-3">
                    <SpecIcon kind={node.kind} size={14} class="flex-shrink-0" />
                    <InlineInput
                      value={ui.editingNodeValue()}
                      placeholder={node.label}
                      onConfirm={(val) => handleRenameConfirm(node.id, node.path ?? "", val)}
                      onCancel={() => ui.clearEditing()}
                    />
                  </div>
                </Show>
              )}
            </For>
          </Show>
        </Show>

        {/* Inline create input */}
        <Show when={ui.creatingInSection() === activeKind() && ui.editingMode() === "create"}>
          <div class="flex items-center gap-1.5 py-0.5 px-3">
            <SpecIcon kind={activeKind()} size={14} class="flex-shrink-0 text-rash-text-muted" />
            <InlineInput
              value=""
              placeholder={getCreatePlaceholder(activeKind())}
              onConfirm={handleCreateConfirm}
              onCancel={() => ui.clearEditing()}
            />
          </div>
        </Show>
      </div>
    </div>
  );
};

// ── Main Sidebar Export ──

export const Sidebar: Component = () => {
  return (
    <>
      <IconRail />
      <ListPanel />
    </>
  );
};
