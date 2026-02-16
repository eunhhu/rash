import { Component, For } from "solid-js";
import type { IndexSpec } from "../../ipc/types";

interface IndexEditorProps {
  indexes: IndexSpec[];
  columnNames: string[];
  onChange: (indexes: IndexSpec[]) => void;
}

export const IndexEditor: Component<IndexEditorProps> = (props) => {
  const addIndex = () => {
    props.onChange([...props.indexes, { columns: [], unique: false }]);
  };

  const removeIndex = (i: number) => {
    const updated = [...props.indexes];
    updated.splice(i, 1);
    props.onChange(updated);
  };

  const updateIndex = (i: number, field: keyof IndexSpec, value: unknown) => {
    const updated = [...props.indexes];
    updated[i] = { ...updated[i], [field]: value };
    props.onChange(updated);
  };

  const toggleColumn = (indexIdx: number, colName: string) => {
    const idx = props.indexes[indexIdx];
    const cols = [...idx.columns];
    const pos = cols.indexOf(colName);
    if (pos >= 0) {
      cols.splice(pos, 1);
    } else {
      cols.push(colName);
    }
    updateIndex(indexIdx, "columns", cols);
  };

  return (
    <div class="index-editor">
      <div class="index-editor-header">
        <button class="btn btn-sm" onClick={addIndex}>+ Add Index</button>
      </div>

      <div class="index-editor-list">
        <For each={props.indexes}>
          {(idx, i) => (
            <div class="index-editor-item">
              <div class="index-editor-item-header">
                <span class="index-editor-item-label">Index {i() + 1}</span>
                <label class="index-editor-unique-label">
                  <input
                    type="checkbox"
                    checked={idx.unique ?? false}
                    onChange={(e) => updateIndex(i(), "unique", e.currentTarget.checked)}
                  />
                  Unique
                </label>
                <button class="btn-icon" onClick={() => removeIndex(i())}>{"\u00D7"}</button>
              </div>

              <div class="index-editor-columns">
                <label class="index-editor-sublabel">Columns</label>
                <div class="index-editor-column-chips">
                  <For each={props.columnNames}>
                    {(col) => (
                      <button
                        class="index-editor-chip"
                        classList={{ "index-editor-chip-active": idx.columns.includes(col) }}
                        onClick={() => toggleColumn(i(), col)}
                      >
                        {col}
                      </button>
                    )}
                  </For>
                </div>
              </div>

              <div class="index-editor-where">
                <label class="index-editor-sublabel">Where clause</label>
                <input
                  value={idx.where ?? ""}
                  placeholder="Optional WHERE condition"
                  onInput={(e) => updateIndex(i(), "where", e.currentTarget.value || undefined)}
                />
              </div>
            </div>
          )}
        </For>
      </div>

      <style>{`
        .index-editor {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .index-editor-header {
          display: flex;
          justify-content: flex-end;
        }
        .index-editor-list {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .index-editor-item {
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
          padding: 10px;
          background: var(--rash-bg-secondary);
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .index-editor-item-header {
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .index-editor-item-label {
          flex: 1;
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .index-editor-unique-label {
          display: flex;
          align-items: center;
          gap: 4px;
          font-size: 11px;
          color: var(--rash-text-muted);
        }
        .index-editor-unique-label input[type="checkbox"] {
          width: auto;
        }
        .index-editor-sublabel {
          font-size: 11px;
          color: var(--rash-text-muted);
        }
        .index-editor-columns {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .index-editor-column-chips {
          display: flex;
          flex-wrap: wrap;
          gap: 4px;
        }
        .index-editor-chip {
          padding: 2px 8px;
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
          background: transparent;
          color: var(--rash-text-secondary);
          font-size: 11px;
          font-family: var(--rash-font);
          cursor: pointer;
          transition: all 0.1s ease;
        }
        .index-editor-chip:hover {
          border-color: var(--rash-accent);
          color: var(--rash-text);
        }
        .index-editor-chip-active {
          background: var(--rash-accent) !important;
          color: var(--rash-bg) !important;
          border-color: var(--rash-accent) !important;
        }
        .index-editor-where {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .index-editor-where input {
          font-size: 12px;
          padding: 4px 8px;
          font-family: var(--rash-font);
        }
      `}</style>
    </div>
  );
};
