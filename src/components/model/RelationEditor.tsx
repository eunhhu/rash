import { Component, For } from "solid-js";
import type { RelationSpec, RelationType } from "../../ipc/types";

interface RelationEditorProps {
  relations: Record<string, RelationSpec>;
  onChange: (relations: Record<string, RelationSpec>) => void;
}

const RELATION_TYPES: RelationType[] = ["hasOne", "hasMany", "belongsTo", "manyToMany"];

export const RelationEditor: Component<RelationEditorProps> = (props) => {
  const entries = () => Object.entries(props.relations);

  const updateRelation = (name: string, field: keyof RelationSpec, value: string) => {
    const updated = { ...props.relations };
    updated[name] = { ...updated[name], [field]: value };
    props.onChange(updated);
  };

  const renameRelation = (oldName: string, newName: string) => {
    if (!newName || newName === oldName) return;
    const result: Record<string, RelationSpec> = {};
    for (const [k, v] of Object.entries(props.relations)) {
      result[k === oldName ? newName : k] = v;
    }
    props.onChange(result);
  };

  const addRelation = () => {
    const name = `relation${entries().length + 1}`;
    props.onChange({
      ...props.relations,
      [name]: { type: "hasOne", target: "", foreignKey: "" },
    });
  };

  const removeRelation = (name: string) => {
    const updated = { ...props.relations };
    delete updated[name];
    props.onChange(updated);
  };

  return (
    <div class="relation-editor">
      <div class="relation-editor-header">
        <button class="btn btn-sm" onClick={addRelation}>+ Add Relation</button>
      </div>
      <table class="relation-editor-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Type</th>
            <th>Target Model</th>
            <th>Foreign Key</th>
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
                    onBlur={(e) => renameRelation(name, e.currentTarget.value)}
                  />
                </td>
                <td>
                  <select
                    value={spec.type}
                    onChange={(e) => updateRelation(name, "type", e.currentTarget.value)}
                  >
                    <For each={RELATION_TYPES}>{(t) => <option value={t}>{t}</option>}</For>
                  </select>
                </td>
                <td>
                  <input
                    value={spec.target}
                    placeholder="ModelName"
                    onInput={(e) => updateRelation(name, "target", e.currentTarget.value)}
                  />
                </td>
                <td>
                  <input
                    value={spec.foreignKey}
                    placeholder="foreign_key"
                    onInput={(e) => updateRelation(name, "foreignKey", e.currentTarget.value)}
                  />
                </td>
                <td>
                  <button class="btn-icon" onClick={() => removeRelation(name)}>{"\u00D7"}</button>
                </td>
              </tr>
            )}
          </For>
        </tbody>
      </table>

      <style>{`
        .relation-editor {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .relation-editor-header {
          display: flex;
          justify-content: flex-end;
        }
        .relation-editor-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 12px;
        }
        .relation-editor-table th {
          text-align: left;
          padding: 6px 8px;
          color: var(--rash-text-muted);
          font-weight: 500;
          border-bottom: 1px solid var(--rash-border);
          font-size: 11px;
        }
        .relation-editor-table td {
          padding: 4px 8px;
          border-bottom: 1px solid var(--rash-border);
        }
        .relation-editor-table input,
        .relation-editor-table select {
          width: 100%;
          padding: 3px 6px;
          font-size: 12px;
        }
      `}</style>
    </div>
  );
};
