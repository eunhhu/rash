import { Component, For } from "solid-js";
import type { ParamSpec } from "../../ipc/types";

interface ParamEditorProps {
  params: Record<string, ParamSpec>;
  onChange: (params: Record<string, ParamSpec>) => void;
}

export const ParamEditor: Component<ParamEditorProps> = (props) => {
  const entries = () => Object.entries(props.params);

  const updateParam = (name: string, field: keyof ParamSpec, value: string) => {
    const updated = { ...props.params };
    updated[name] = { ...updated[name], [field]: value || undefined };
    props.onChange(updated);
  };

  const renameParam = (oldName: string, newName: string) => {
    if (!newName || newName === oldName) return;
    const updated: Record<string, ParamSpec> = {};
    for (const [key, val] of Object.entries(props.params)) {
      updated[key === oldName ? newName : key] = val;
    }
    props.onChange(updated);
  };

  const addParam = () => {
    const name = `param${entries().length + 1}`;
    props.onChange({ ...props.params, [name]: { type: "string" } });
  };

  const removeParam = (name: string) => {
    const updated = { ...props.params };
    delete updated[name];
    props.onChange(updated);
  };

  return (
    <div class="param-editor">
      <div class="param-editor-header">
        <span class="param-editor-title">Path Parameters</span>
        <button class="btn btn-sm" onClick={addParam}>+ Add</button>
      </div>
      <table class="param-editor-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Format</th>
            <th>Description</th>
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
                    onBlur={(e) => renameParam(name, e.currentTarget.value)}
                  />
                </td>
                <td>
                  <select
                    value={spec.type}
                    onChange={(e) => updateParam(name, "type", e.currentTarget.value)}
                  >
                    <option value="string">string</option>
                    <option value="integer">integer</option>
                    <option value="number">number</option>
                    <option value="boolean">boolean</option>
                  </select>
                </td>
                <td>
                  <input
                    value={spec.format ?? ""}
                    placeholder="e.g. uuid"
                    onInput={(e) => updateParam(name, "format", e.currentTarget.value)}
                  />
                </td>
                <td>
                  <input
                    value={spec.description ?? ""}
                    placeholder="Description"
                    onInput={(e) => updateParam(name, "description", e.currentTarget.value)}
                  />
                </td>
                <td>
                  <button class="btn-icon" onClick={() => removeParam(name)}>{"\u00D7"}</button>
                </td>
              </tr>
            )}
          </For>
        </tbody>
      </table>

      <style>{`
        .param-editor {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .param-editor-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .param-editor-title {
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .param-editor-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 12px;
        }
        .param-editor-table th {
          text-align: left;
          padding: 4px 8px;
          color: var(--rash-text-muted);
          font-weight: 500;
          border-bottom: 1px solid var(--rash-border);
        }
        .param-editor-table td {
          padding: 4px 8px;
          border-bottom: 1px solid var(--rash-border);
        }
        .param-editor-table input,
        .param-editor-table select {
          width: 100%;
          padding: 3px 6px;
          font-size: 12px;
        }
      `}</style>
    </div>
  );
};
