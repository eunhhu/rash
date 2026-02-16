import { Component, For, Show } from "solid-js";

interface DefinitionListProps {
  definitions: string[];
  selectedId: string | null;
  onSelect: (name: string) => void;
  onAdd: () => void;
  onRemove: (name: string) => void;
}

export const DefinitionList: Component<DefinitionListProps> = (props) => {
  return (
    <div class="definition-list">
      <div class="definition-list-header">
        <span class="definition-list-title">Definitions</span>
        <button class="btn btn-sm" onClick={props.onAdd}>+ Add</button>
      </div>
      <div class="definition-list-items">
        <Show when={props.definitions.length > 0} fallback={
          <div class="definition-list-empty">No definitions</div>
        }>
          <For each={props.definitions}>
            {(name) => (
              <div
                class="definition-list-item"
                classList={{ "definition-list-item-selected": props.selectedId === name }}
                onClick={() => props.onSelect(name)}
              >
                <span class="definition-list-item-name">{name}</span>
                <button
                  class="btn-icon definition-list-remove"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onRemove(name);
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
        .definition-list {
          display: flex;
          flex-direction: column;
          height: 100%;
          border-right: 1px solid var(--rash-border);
          min-width: 180px;
          max-width: 240px;
        }
        .definition-list-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 8px 12px;
          border-bottom: 1px solid var(--rash-border);
        }
        .definition-list-title {
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .definition-list-items {
          flex: 1;
          overflow-y: auto;
          padding: 4px;
        }
        .definition-list-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 6px 10px;
          cursor: pointer;
          border-radius: var(--rash-radius-sm);
          font-size: 13px;
          color: var(--rash-text-secondary);
          transition: background 0.1s ease;
        }
        .definition-list-item:hover {
          background: var(--rash-surface);
          color: var(--rash-text);
        }
        .definition-list-item-selected {
          background: var(--rash-surface) !important;
          color: var(--rash-accent) !important;
        }
        .definition-list-item-name {
          font-family: var(--rash-font);
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .definition-list-remove {
          opacity: 0;
          flex-shrink: 0;
        }
        .definition-list-item:hover .definition-list-remove {
          opacity: 1;
        }
        .definition-list-empty {
          padding: 16px;
          text-align: center;
          color: var(--rash-text-muted);
          font-size: 12px;
        }
      `}</style>
    </div>
  );
};
