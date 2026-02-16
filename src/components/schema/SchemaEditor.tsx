import { Component, Show, createSignal, createEffect, createMemo } from "solid-js";
import { writeSchema } from "../../ipc/commands";
import type { SchemaSpec } from "../../ipc/types";
import { DefinitionList } from "./DefinitionList";
import { FieldEditor, type FieldDef } from "./FieldEditor";

interface SchemaEditorProps {
  schema: SchemaSpec;
  filePath: string;
  onDirty?: (dirty: boolean) => void;
}

export const SchemaEditor: Component<SchemaEditorProps> = (props) => {
  const [draft, setDraft] = createSignal<SchemaSpec>(structuredClone(props.schema));
  const [selectedDef, setSelectedDef] = createSignal<string | null>(null);
  const [dirty, setDirty] = createSignal(false);
  const [saving, setSaving] = createSignal(false);

  createEffect(() => {
    setDraft(structuredClone(props.schema));
    setDirty(false);
    const names = Object.keys(props.schema.definitions);
    setSelectedDef(names[0] ?? null);
  });

  const defNames = createMemo(() => Object.keys(draft().definitions));

  const markDirty = () => {
    setDirty(true);
    props.onDirty?.(true);
  };

  const addDefinition = () => {
    const name = `NewType${defNames().length + 1}`;
    setDraft((prev) => ({
      ...prev,
      definitions: { ...prev.definitions, [name]: { type: "object", properties: {} } },
    }));
    setSelectedDef(name);
    markDirty();
  };

  const removeDefinition = (name: string) => {
    setDraft((prev) => {
      const updated = { ...prev.definitions };
      delete updated[name];
      return { ...prev, definitions: updated };
    });
    if (selectedDef() === name) {
      const remaining = defNames().filter((n) => n !== name);
      setSelectedDef(remaining[0] ?? null);
    }
    markDirty();
  };

  const updateDefinition = (name: string, field: FieldDef) => {
    setDraft((prev) => ({
      ...prev,
      definitions: { ...prev.definitions, [name]: field },
    }));
    markDirty();
  };

  const selectedField = () => {
    const name = selectedDef();
    if (!name) return null;
    return (draft().definitions[name] as FieldDef) ?? null;
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await writeSchema(props.filePath, draft());
      setDirty(false);
      props.onDirty?.(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="schema-editor">
      <div class="schema-editor-header">
        <span class="schema-editor-name">{draft().name}</span>
        <button
          class="btn btn-primary btn-sm"
          disabled={!dirty() || saving()}
          onClick={handleSave}
        >
          {saving() ? "Saving..." : "Save"}
        </button>
      </div>

      <div class="schema-editor-body">
        <DefinitionList
          definitions={defNames()}
          selectedId={selectedDef()}
          onSelect={setSelectedDef}
          onAdd={addDefinition}
          onRemove={removeDefinition}
        />

        <div class="schema-editor-field-area">
          <Show when={selectedDef() && selectedField()} fallback={
            <div class="schema-editor-empty">Select a definition to edit</div>
          }>
            <FieldEditor
              field={selectedField()!}
              name={selectedDef()!}
              onChange={(f) => updateDefinition(selectedDef()!, f)}
              onRename={(newName) => {
                const oldName = selectedDef()!;
                if (newName === oldName) return;
                setDraft((prev) => {
                  const updated: Record<string, unknown> = {};
                  for (const [k, v] of Object.entries(prev.definitions)) {
                    updated[k === oldName ? newName : k] = v;
                  }
                  return { ...prev, definitions: updated };
                });
                setSelectedDef(newName);
                markDirty();
              }}
            />
          </Show>
        </div>
      </div>

      <style>{`
        .schema-editor {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .schema-editor-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 16px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
          flex-shrink: 0;
        }
        .schema-editor-name {
          font-size: 14px;
          font-weight: 600;
          color: var(--rash-text);
        }
        .schema-editor-body {
          display: flex;
          flex: 1;
          overflow: hidden;
        }
        .schema-editor-field-area {
          flex: 1;
          padding: 12px 16px;
          overflow-y: auto;
        }
        .schema-editor-empty {
          display: flex;
          align-items: center;
          justify-content: center;
          height: 100%;
          color: var(--rash-text-muted);
          font-size: 13px;
        }
      `}</style>
    </div>
  );
};
