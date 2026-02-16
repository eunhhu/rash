import { Component, For, Show, createSignal } from "solid-js";

export interface FieldDef {
  type: string;
  description?: string;
  required?: boolean;
  default?: unknown;
  minimum?: number;
  maximum?: number;
  minLength?: number;
  maxLength?: number;
  format?: string;
  pattern?: string;
  enum?: string[];
  items?: FieldDef;
  properties?: Record<string, FieldDef>;
}

interface FieldEditorProps {
  field: FieldDef;
  name: string;
  onChange: (field: FieldDef) => void;
  onRemove?: () => void;
  onRename?: (newName: string) => void;
  depth?: number;
}

const TYPES = ["string", "number", "integer", "boolean", "object", "array", "enum"];

export const FieldEditor: Component<FieldEditorProps> = (props) => {
  const [expanded, setExpanded] = createSignal(true);
  const depth = () => props.depth ?? 0;

  const update = (key: keyof FieldDef, value: unknown) => {
    props.onChange({ ...props.field, [key]: value });
  };

  const handleTypeChange = (type: string) => {
    const base: FieldDef = { type, description: props.field.description, required: props.field.required };
    if (type === "object") {
      base.properties = props.field.properties ?? {};
    } else if (type === "array") {
      base.items = props.field.items ?? { type: "string" };
    } else if (type === "enum") {
      base.enum = props.field.enum ?? [];
    }
    props.onChange(base);
  };

  const addNestedField = () => {
    const existing = props.field.properties ?? {};
    const name = `field${Object.keys(existing).length + 1}`;
    update("properties", { ...existing, [name]: { type: "string" } });
  };

  const updateNestedField = (name: string, field: FieldDef) => {
    const updated = { ...(props.field.properties ?? {}), [name]: field };
    update("properties", updated);
  };

  const removeNestedField = (name: string) => {
    const updated = { ...(props.field.properties ?? {}) };
    delete updated[name];
    update("properties", updated);
  };

  const renameNestedField = (oldName: string, newName: string) => {
    if (!newName || newName === oldName) return;
    const result: Record<string, FieldDef> = {};
    for (const [k, v] of Object.entries(props.field.properties ?? {})) {
      result[k === oldName ? newName : k] = v;
    }
    update("properties", result);
  };

  const addEnumValue = () => {
    const values = [...(props.field.enum ?? []), ""];
    update("enum", values);
  };

  const updateEnumValue = (index: number, value: string) => {
    const values = [...(props.field.enum ?? [])];
    values[index] = value;
    update("enum", values);
  };

  const removeEnumValue = (index: number) => {
    const values = [...(props.field.enum ?? [])];
    values.splice(index, 1);
    update("enum", values);
  };

  return (
    <div class="field-editor" style={{ "margin-left": depth() > 0 ? "16px" : "0" }}>
      <div class="field-editor-row">
        <button class="field-editor-toggle" onClick={() => setExpanded((p) => !p)}>
          {expanded() ? "\u25BC" : "\u25B6"}
        </button>

        <input
          class="field-editor-name"
          value={props.name}
          onBlur={(e) => props.onRename?.(e.currentTarget.value)}
        />

        <select
          class="field-editor-type"
          value={props.field.type}
          onChange={(e) => handleTypeChange(e.currentTarget.value)}
        >
          <For each={TYPES}>{(t) => <option value={t}>{t}</option>}</For>
        </select>

        <label class="field-editor-required-label">
          <input
            type="checkbox"
            checked={props.field.required ?? false}
            onChange={(e) => update("required", e.currentTarget.checked)}
          />
          req
        </label>

        <Show when={props.onRemove}>
          <button class="btn-icon" onClick={() => props.onRemove!()}>{"\u00D7"}</button>
        </Show>
      </div>

      <Show when={expanded()}>
        <div class="field-editor-details">
          {/* Description */}
          <div class="field-editor-detail-row">
            <label>Description</label>
            <input
              value={props.field.description ?? ""}
              onInput={(e) => update("description", e.currentTarget.value)}
            />
          </div>

          {/* Default */}
          <div class="field-editor-detail-row">
            <label>Default</label>
            <input
              value={props.field.default != null ? String(props.field.default) : ""}
              onInput={(e) => update("default", e.currentTarget.value || undefined)}
            />
          </div>

          {/* Constraints for string */}
          <Show when={props.field.type === "string"}>
            <div class="field-editor-constraints">
              <div class="field-editor-detail-row">
                <label>Format</label>
                <input value={props.field.format ?? ""} onInput={(e) => update("format", e.currentTarget.value || undefined)} />
              </div>
              <div class="field-editor-detail-row">
                <label>Pattern</label>
                <input value={props.field.pattern ?? ""} onInput={(e) => update("pattern", e.currentTarget.value || undefined)} />
              </div>
              <div class="field-editor-number-row">
                <div class="field-editor-detail-row">
                  <label>Min Length</label>
                  <input type="number" value={props.field.minLength ?? ""} onInput={(e) => update("minLength", e.currentTarget.value ? Number(e.currentTarget.value) : undefined)} />
                </div>
                <div class="field-editor-detail-row">
                  <label>Max Length</label>
                  <input type="number" value={props.field.maxLength ?? ""} onInput={(e) => update("maxLength", e.currentTarget.value ? Number(e.currentTarget.value) : undefined)} />
                </div>
              </div>
            </div>
          </Show>

          {/* Constraints for number / integer */}
          <Show when={props.field.type === "number" || props.field.type === "integer"}>
            <div class="field-editor-number-row">
              <div class="field-editor-detail-row">
                <label>Min</label>
                <input type="number" value={props.field.minimum ?? ""} onInput={(e) => update("minimum", e.currentTarget.value ? Number(e.currentTarget.value) : undefined)} />
              </div>
              <div class="field-editor-detail-row">
                <label>Max</label>
                <input type="number" value={props.field.maximum ?? ""} onInput={(e) => update("maximum", e.currentTarget.value ? Number(e.currentTarget.value) : undefined)} />
              </div>
            </div>
          </Show>

          {/* Enum values */}
          <Show when={props.field.type === "enum"}>
            <div class="field-editor-enum">
              <div class="field-editor-enum-header">
                <label>Enum Values</label>
                <button class="btn btn-sm" onClick={addEnumValue}>+ Add</button>
              </div>
              <For each={props.field.enum ?? []}>
                {(val, i) => (
                  <div class="field-editor-enum-row">
                    <input
                      value={val}
                      onInput={(e) => updateEnumValue(i(), e.currentTarget.value)}
                    />
                    <button class="btn-icon" onClick={() => removeEnumValue(i())}>{"\u00D7"}</button>
                  </div>
                )}
              </For>
            </div>
          </Show>

          {/* Array items type */}
          <Show when={props.field.type === "array" && props.field.items}>
            <div class="field-editor-array-items">
              <label class="field-editor-sublabel">Array Items</label>
              <FieldEditor
                field={props.field.items!}
                name="items"
                depth={depth() + 1}
                onChange={(f) => update("items", f)}
              />
            </div>
          </Show>

          {/* Object properties (recursive) */}
          <Show when={props.field.type === "object"}>
            <div class="field-editor-nested">
              <div class="field-editor-enum-header">
                <label class="field-editor-sublabel">Properties</label>
                <button class="btn btn-sm" onClick={addNestedField}>+ Add</button>
              </div>
              <For each={Object.entries(props.field.properties ?? {})}>
                {([name, def]) => (
                  <FieldEditor
                    field={def}
                    name={name}
                    depth={depth() + 1}
                    onChange={(f) => updateNestedField(name, f)}
                    onRemove={() => removeNestedField(name)}
                    onRename={(n) => renameNestedField(name, n)}
                  />
                )}
              </For>
            </div>
          </Show>
        </div>
      </Show>

      <style>{`
        .field-editor {
          border-left: 2px solid var(--rash-border);
          padding: 4px 0 4px 8px;
          margin: 2px 0;
        }
        .field-editor-row {
          display: flex;
          align-items: center;
          gap: 6px;
        }
        .field-editor-toggle {
          background: none;
          border: none;
          color: var(--rash-text-muted);
          cursor: pointer;
          font-size: 8px;
          width: 14px;
          padding: 0;
        }
        .field-editor-name {
          width: 120px;
          font-family: var(--rash-font);
          font-size: 12px;
        }
        .field-editor-type {
          width: 100px;
          font-size: 12px;
        }
        .field-editor-required-label {
          display: flex;
          align-items: center;
          gap: 3px;
          font-size: 11px;
          color: var(--rash-text-muted);
          white-space: nowrap;
        }
        .field-editor-required-label input[type="checkbox"] {
          width: auto;
          padding: 0;
        }
        .field-editor-details {
          display: flex;
          flex-direction: column;
          gap: 4px;
          padding: 6px 0 2px 20px;
        }
        .field-editor-detail-row {
          display: flex;
          align-items: center;
          gap: 6px;
        }
        .field-editor-detail-row label {
          font-size: 11px;
          color: var(--rash-text-muted);
          min-width: 70px;
          white-space: nowrap;
        }
        .field-editor-detail-row input {
          flex: 1;
          font-size: 12px;
          padding: 3px 6px;
        }
        .field-editor-number-row {
          display: flex;
          gap: 8px;
        }
        .field-editor-number-row .field-editor-detail-row {
          flex: 1;
        }
        .field-editor-constraints {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .field-editor-enum {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .field-editor-enum-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .field-editor-enum-row {
          display: flex;
          gap: 4px;
          align-items: center;
        }
        .field-editor-enum-row input {
          flex: 1;
          font-size: 12px;
          padding: 3px 6px;
        }
        .field-editor-sublabel {
          font-size: 11px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .field-editor-nested,
        .field-editor-array-items {
          display: flex;
          flex-direction: column;
          gap: 4px;
          margin-top: 4px;
        }
      `}</style>
    </div>
  );
};
