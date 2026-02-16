import { Component, For } from "solid-js";
import type { ColumnSpec } from "../../ipc/types";

interface ColumnEditorProps {
  columns: Record<string, ColumnSpec>;
  onChange: (columns: Record<string, ColumnSpec>) => void;
}

const COLUMN_TYPES = [
  "uuid", "varchar(255)", "text", "integer", "float", "boolean",
  "timestamp", "enum", "json",
];

export const ColumnEditor: Component<ColumnEditorProps> = (props) => {
  const entries = () => Object.entries(props.columns);

  const updateColumn = (name: string, field: keyof ColumnSpec, value: unknown) => {
    const updated = { ...props.columns };
    updated[name] = { ...updated[name], [field]: value };
    props.onChange(updated);
  };

  const renameColumn = (oldName: string, newName: string) => {
    if (!newName || newName === oldName) return;
    const result: Record<string, ColumnSpec> = {};
    for (const [k, v] of Object.entries(props.columns)) {
      result[k === oldName ? newName : k] = v;
    }
    props.onChange(result);
  };

  const addColumn = () => {
    const name = `column${entries().length + 1}`;
    props.onChange({ ...props.columns, [name]: { type: "varchar(255)" } });
  };

  const removeColumn = (name: string) => {
    const updated = { ...props.columns };
    delete updated[name];
    props.onChange(updated);
  };

  return (
    <div class="column-editor">
      <div class="column-editor-header">
        <button class="btn btn-sm" onClick={addColumn}>+ Add Column</button>
      </div>
      <div class="column-editor-table-wrap">
        <table class="column-editor-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>PK</th>
              <th>Unique</th>
              <th>Nullable</th>
              <th>Index</th>
              <th>Default</th>
              <th>On Update</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <For each={entries()}>
              {([name, spec]) => (
                <tr>
                  <td>
                    <input
                      value={name}
                      onBlur={(e) => renameColumn(name, e.currentTarget.value)}
                    />
                  </td>
                  <td>
                    <select
                      value={spec.type}
                      onChange={(e) => updateColumn(name, "type", e.currentTarget.value)}
                    >
                      <For each={COLUMN_TYPES}>{(t) => <option value={t}>{t}</option>}</For>
                    </select>
                  </td>
                  <td>
                    <input
                      type="checkbox"
                      checked={spec.primaryKey ?? false}
                      onChange={(e) => updateColumn(name, "primaryKey", e.currentTarget.checked)}
                    />
                  </td>
                  <td>
                    <input
                      type="checkbox"
                      checked={spec.unique ?? false}
                      onChange={(e) => updateColumn(name, "unique", e.currentTarget.checked)}
                    />
                  </td>
                  <td>
                    <input
                      type="checkbox"
                      checked={spec.nullable ?? false}
                      onChange={(e) => updateColumn(name, "nullable", e.currentTarget.checked)}
                    />
                  </td>
                  <td>
                    <input
                      type="checkbox"
                      checked={spec.index ?? false}
                      onChange={(e) => updateColumn(name, "index", e.currentTarget.checked)}
                    />
                  </td>
                  <td>
                    <input
                      value={spec.default ?? ""}
                      placeholder="default"
                      onInput={(e) => updateColumn(name, "default", e.currentTarget.value || undefined)}
                    />
                  </td>
                  <td>
                    <input
                      value={spec.onUpdate ?? ""}
                      placeholder="on update"
                      onInput={(e) => updateColumn(name, "onUpdate", e.currentTarget.value || undefined)}
                    />
                  </td>
                  <td>
                    <button class="btn-icon" onClick={() => removeColumn(name)}>{"\u00D7"}</button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>

      <style>{`
        .column-editor {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .column-editor-header {
          display: flex;
          justify-content: flex-end;
        }
        .column-editor-table-wrap {
          overflow-x: auto;
        }
        .column-editor-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 12px;
          white-space: nowrap;
        }
        .column-editor-table th {
          text-align: left;
          padding: 6px 8px;
          color: var(--rash-text-muted);
          font-weight: 500;
          border-bottom: 1px solid var(--rash-border);
          font-size: 11px;
        }
        .column-editor-table td {
          padding: 4px 8px;
          border-bottom: 1px solid var(--rash-border);
        }
        .column-editor-table input[type="text"],
        .column-editor-table input:not([type]),
        .column-editor-table select {
          width: 100%;
          padding: 3px 6px;
          font-size: 12px;
          min-width: 80px;
        }
        .column-editor-table input[type="checkbox"] {
          width: auto;
          margin: 0 auto;
          display: block;
        }
      `}</style>
    </div>
  );
};
