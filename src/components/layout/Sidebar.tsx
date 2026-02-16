import { Component, For, Show, createSignal } from "solid-js";
import { useProjectStore } from "../../stores/projectStore";
import { useEditorStore } from "../../stores/editorStore";
import type { TreeNode } from "../../ipc/types";
import "./layout.css";

interface SidebarSectionProps {
  title: string;
  kind: string;
  nodes: TreeNode[];
}

const kindIcons: Record<string, string> = {
  route: "/",
  schema: "{}",
  model: "M",
  middleware: "~",
  handler: "f",
};

const SidebarSection: Component<SidebarSectionProps> = (props) => {
  const [collapsed, setCollapsed] = createSignal(false);
  const { openTab } = useEditorStore();
  const { activeTabId } = useEditorStore();

  const handleItemClick = (node: TreeNode) => {
    openTab(node.id, node.label, node.kind, node.path ?? "");
  };

  return (
    <div class="sidebar-section">
      <div
        class="sidebar-section-header"
        onClick={() => setCollapsed((prev) => !prev)}
      >
        <span>{props.title}</span>
        <span class={`chevron ${collapsed() ? "collapsed" : ""}`}>
          &#9660;
        </span>
      </div>
      <Show when={!collapsed()}>
        <div class="sidebar-section-body">
          <Show
            when={props.nodes.length > 0}
            fallback={<div class="sidebar-empty">No {props.title.toLowerCase()}</div>}
          >
            <For each={props.nodes}>
              {(node) => (
                <TreeItem
                  node={node}
                  activeId={activeTabId()}
                  onSelect={handleItemClick}
                  depth={0}
                />
              )}
            </For>
          </Show>
        </div>
      </Show>
    </div>
  );
};

interface TreeItemProps {
  node: TreeNode;
  activeId: string | null;
  onSelect: (node: TreeNode) => void;
  depth: number;
}

const TreeItem: Component<TreeItemProps> = (props) => {
  const [expanded, setExpanded] = createSignal(true);
  const hasChildren = () => props.node.children && props.node.children.length > 0;
  const icon = () => kindIcons[props.node.kind] ?? "#";

  return (
    <>
      <div
        class={`sidebar-item ${props.activeId === props.node.id ? "active" : ""}`}
        style={{ "padding-left": `${20 + props.depth * 12}px` }}
        onClick={(e) => {
          e.stopPropagation();
          if (hasChildren()) {
            setExpanded((prev) => !prev);
          }
          props.onSelect(props.node);
        }}
      >
        <span class="item-icon">{icon()}</span>
        <span class="item-label">{props.node.label}</span>
      </div>
      <Show when={hasChildren() && expanded()}>
        <For each={props.node.children}>
          {(child) => (
            <TreeItem
              node={child}
              activeId={props.activeId}
              onSelect={props.onSelect}
              depth={props.depth + 1}
            />
          )}
        </For>
      </Show>
    </>
  );
};

export const Sidebar: Component = () => {
  const { project } = useProjectStore();

  const nodesByKind = (kind: string): TreeNode[] => {
    const tree = project();
    if (!tree) return [];
    return tree.nodes.filter((n) => n.kind === kind);
  };

  return (
    <div class="sidebar">
      <SidebarSection title="Routes" kind="route" nodes={nodesByKind("route")} />
      <SidebarSection title="Schemas" kind="schema" nodes={nodesByKind("schema")} />
      <SidebarSection title="Models" kind="model" nodes={nodesByKind("model")} />
      <SidebarSection title="Middleware" kind="middleware" nodes={nodesByKind("middleware")} />
      <SidebarSection title="Handlers" kind="handler" nodes={nodesByKind("handler")} />
    </div>
  );
};
