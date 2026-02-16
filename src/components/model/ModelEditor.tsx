import { Component, Show, createSignal, createEffect, createMemo } from "solid-js";
import { writeModel } from "../../ipc/commands";
import type { ModelSpec, ColumnSpec, RelationSpec, IndexSpec } from "../../ipc/types";
import { TabPanel, type TabItem } from "../common/TabPanel";
import { ColumnEditor } from "./ColumnEditor";
import { RelationEditor } from "./RelationEditor";
import { IndexEditor } from "./IndexEditor";

interface ModelEditorProps {
  model: ModelSpec;
  filePath: string;
  onDirty?: (dirty: boolean) => void;
}

type ModelTab = "columns" | "relations" | "indexes";

const TABS: TabItem[] = [
  { id: "columns", label: "Columns", closable: false },
  { id: "relations", label: "Relations", closable: false },
  { id: "indexes", label: "Indexes", closable: false },
];

export const ModelEditor: Component<ModelEditorProps> = (props) => {
  const [draft, setDraft] = createSignal<ModelSpec>(structuredClone(props.model));
  const [activeTab, setActiveTab] = createSignal<ModelTab>("columns");
  const [dirty, setDirty] = createSignal(false);
  const [saving, setSaving] = createSignal(false);

  createEffect(() => {
    setDraft(structuredClone(props.model));
    setDirty(false);
  });

  const columnNames = createMemo(() => Object.keys(draft().columns));

  const markDirty = () => {
    setDirty(true);
    props.onDirty?.(true);
  };

  const updateColumns = (columns: Record<string, ColumnSpec>) => {
    setDraft((prev) => ({ ...prev, columns }));
    markDirty();
  };

  const updateRelations = (relations: Record<string, RelationSpec>) => {
    setDraft((prev) => ({ ...prev, relations }));
    markDirty();
  };

  const updateIndexes = (indexes: IndexSpec[]) => {
    setDraft((prev) => ({ ...prev, indexes }));
    markDirty();
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await writeModel(props.filePath, draft());
      setDirty(false);
      props.onDirty?.(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="model-editor">
      <div class="model-editor-header">
        <div class="model-editor-info">
          <span class="model-editor-name">{draft().name}</span>
          <Show when={draft().tableName}>
            <span class="model-editor-table-name">({draft().tableName})</span>
          </Show>
        </div>
        <button
          class="btn btn-primary btn-sm"
          disabled={!dirty() || saving()}
          onClick={handleSave}
        >
          {saving() ? "Saving..." : "Save"}
        </button>
      </div>

      <TabPanel
        tabs={TABS}
        activeId={activeTab()}
        onSelect={(id) => setActiveTab(id as ModelTab)}
      />

      <div class="model-editor-content">
        <Show when={activeTab() === "columns"}>
          <ColumnEditor columns={draft().columns} onChange={updateColumns} />
        </Show>
        <Show when={activeTab() === "relations"}>
          <RelationEditor
            relations={draft().relations ?? {}}
            onChange={updateRelations}
          />
        </Show>
        <Show when={activeTab() === "indexes"}>
          <IndexEditor
            indexes={draft().indexes ?? []}
            columnNames={columnNames()}
            onChange={updateIndexes}
          />
        </Show>
      </div>

      <style>{`
        .model-editor {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .model-editor-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 16px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
          flex-shrink: 0;
        }
        .model-editor-info {
          display: flex;
          align-items: baseline;
          gap: 8px;
        }
        .model-editor-name {
          font-size: 14px;
          font-weight: 600;
          color: var(--rash-text);
        }
        .model-editor-table-name {
          font-size: 12px;
          color: var(--rash-text-muted);
          font-family: var(--rash-font);
        }
        .model-editor-content {
          flex: 1;
          padding: 16px;
          overflow-y: auto;
        }
      `}</style>
    </div>
  );
};
