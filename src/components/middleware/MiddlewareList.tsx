import { Component, For, Show } from "solid-js";
import type { MiddlewareSpec } from "../../ipc/types";
import { Badge } from "../common/Badge";

interface MiddlewareListProps {
  items: { name: string; spec: MiddlewareSpec; filePath: string }[];
  selectedName: string | null;
  onSelect: (name: string, filePath: string) => void;
  onAdd: () => void;
  onDelete: (name: string) => void;
}

export const MiddlewareList: Component<MiddlewareListProps> = (props) => {
  return (
    <div class="middleware-list">
      <div class="middleware-list-header">
        <span class="middleware-list-title">Middleware</span>
        <button class="btn btn-sm" onClick={props.onAdd}>+ Add</button>
      </div>
      <div class="middleware-list-items">
        <Show when={props.items.length > 0} fallback={
          <div class="middleware-list-empty">No middleware defined</div>
        }>
          <For each={props.items}>
            {(item) => (
              <div
                class="middleware-list-item"
                classList={{ "middleware-list-item-selected": props.selectedName === item.name }}
                onClick={() => props.onSelect(item.name, item.filePath)}
              >
                <div class="middleware-list-item-info">
                  <span class="middleware-list-item-name">{item.name}</span>
                  <span class="middleware-list-item-type">{item.spec.type}</span>
                </div>
                <button
                  class="btn-icon middleware-list-delete"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onDelete(item.name);
                  }}
                >
                  {"\u00D7"}
                </button>
              </div>
            )}
          </For>
        </Show>
      </div>

      <style>{`
        .middleware-list {
          display: flex;
          flex-direction: column;
          height: 100%;
          border-right: 1px solid var(--rash-border);
          min-width: 200px;
          max-width: 260px;
        }
        .middleware-list-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 8px 12px;
          border-bottom: 1px solid var(--rash-border);
        }
        .middleware-list-title {
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .middleware-list-items {
          flex: 1;
          overflow-y: auto;
          padding: 4px;
        }
        .middleware-list-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 8px 10px;
          cursor: pointer;
          border-radius: var(--rash-radius-sm);
          transition: background 0.1s ease;
        }
        .middleware-list-item:hover {
          background: var(--rash-surface);
        }
        .middleware-list-item-selected {
          background: var(--rash-surface) !important;
        }
        .middleware-list-item-info {
          display: flex;
          flex-direction: column;
          gap: 2px;
          overflow: hidden;
        }
        .middleware-list-item-name {
          font-size: 13px;
          font-family: var(--rash-font);
          color: var(--rash-text);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .middleware-list-item-selected .middleware-list-item-name {
          color: var(--rash-accent);
        }
        .middleware-list-item-type {
          font-size: 11px;
          color: var(--rash-text-muted);
        }
        .middleware-list-delete {
          opacity: 0;
          flex-shrink: 0;
        }
        .middleware-list-item:hover .middleware-list-delete {
          opacity: 1;
        }
        .middleware-list-empty {
          padding: 16px;
          text-align: center;
          color: var(--rash-text-muted);
          font-size: 12px;
        }
      `}</style>
    </div>
  );
};
