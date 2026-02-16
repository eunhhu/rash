import { Component, For, Show, createSignal, JSX } from "solid-js";

export interface TreeItem {
  id: string;
  label: string;
  icon?: string;
  children?: TreeItem[];
}

interface TreeViewProps {
  items: TreeItem[];
  onSelect: (id: string) => void;
  selectedId?: string;
}

interface TreeNodeProps {
  item: TreeItem;
  depth: number;
  onSelect: (id: string) => void;
  selectedId?: string;
}

const TreeNode: Component<TreeNodeProps> = (props) => {
  const [expanded, setExpanded] = createSignal(true);
  const hasChildren = () => (props.item.children?.length ?? 0) > 0;

  const handleClick: JSX.EventHandler<HTMLDivElement, MouseEvent> = (e) => {
    e.stopPropagation();
    if (hasChildren()) {
      setExpanded((prev) => !prev);
    }
    props.onSelect(props.item.id);
  };

  return (
    <div class="tree-node">
      <div
        class="tree-node-row"
        classList={{ "tree-node-selected": props.selectedId === props.item.id }}
        style={{ "padding-left": `${props.depth * 16 + 8}px` }}
        onClick={handleClick}
      >
        <Show when={hasChildren()}>
          <span class="tree-node-chevron" classList={{ "tree-node-chevron-open": expanded() }}>
            {"\u25B6"}
          </span>
        </Show>
        <Show when={!hasChildren()}>
          <span class="tree-node-spacer" />
        </Show>
        <Show when={props.item.icon}>
          <span class="tree-node-icon">{props.item.icon}</span>
        </Show>
        <span class="tree-node-label">{props.item.label}</span>
      </div>
      <Show when={hasChildren() && expanded()}>
        <div class="tree-node-children">
          <For each={props.item.children}>
            {(child) => (
              <TreeNode
                item={child}
                depth={props.depth + 1}
                onSelect={props.onSelect}
                selectedId={props.selectedId}
              />
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export const TreeView: Component<TreeViewProps> = (props) => {
  return (
    <div class="tree-view">
      <For each={props.items}>
        {(item) => (
          <TreeNode item={item} depth={0} onSelect={props.onSelect} selectedId={props.selectedId} />
        )}
      </For>

      <style>{`
        .tree-view {
          font-family: var(--rash-font-ui);
          font-size: var(--rash-font-size);
          user-select: none;
          overflow-y: auto;
        }
        .tree-node-row {
          display: flex;
          align-items: center;
          gap: 4px;
          padding: 4px 8px;
          cursor: pointer;
          border-radius: var(--rash-radius-sm);
          color: var(--rash-text-secondary);
          transition: background 0.1s ease;
        }
        .tree-node-row:hover {
          background: var(--rash-surface);
          color: var(--rash-text);
        }
        .tree-node-selected {
          background: var(--rash-surface) !important;
          color: var(--rash-accent) !important;
        }
        .tree-node-chevron {
          font-size: 8px;
          width: 14px;
          text-align: center;
          transition: transform 0.15s ease;
          color: var(--rash-text-muted);
        }
        .tree-node-chevron-open {
          transform: rotate(90deg);
        }
        .tree-node-spacer {
          width: 14px;
        }
        .tree-node-icon {
          font-size: 14px;
          width: 18px;
          text-align: center;
        }
        .tree-node-label {
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
      `}</style>
    </div>
  );
};
