import { Component, Show, createSignal, createEffect, For } from "solid-js";
import { writeRoute } from "../../ipc/commands";
import type { RouteSpec, HttpMethod, EndpointSpec } from "../../ipc/types";
import { useNotificationStore } from "../../stores/notificationStore";
import { createAutoSave } from "../../utils/autoSave";
import { Badge } from "../common/Badge";
import { DropdownMenu, type DropdownItem } from "../common/DropdownMenu";
import { MethodBadge } from "../common/MethodBadge";
import { TabPanel, type TabItem } from "../common/TabPanel";
import { MethodPanel } from "./MethodPanel";
import { ParamEditor } from "./ParamEditor";

interface RouteEditorProps {
  route: RouteSpec;
  filePath: string;
  onDirty?: (dirty: boolean) => void;
}

const ALL_METHODS: HttpMethod[] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

export const RouteEditor: Component<RouteEditorProps> = (props) => {
  const toast = useNotificationStore();
  const [draft, setDraft] = createSignal<RouteSpec>(structuredClone(props.route));
  const [activeMethod, setActiveMethod] = createSignal<HttpMethod | null>(null);
  const [dirty, setDirty] = createSignal(false);
  const [editingPath, setEditingPath] = createSignal(false);
  const [pathDraft, setPathDraft] = createSignal("");

  createEffect(() => {
    setDraft(structuredClone(props.route));
    setDirty(false);
    // Select first method tab
    const methods = Object.keys(props.route.methods) as HttpMethod[];
    setActiveMethod(methods[0] ?? null);
  });

  const methods = () => Object.keys(draft().methods) as HttpMethod[];

  const methodTabs = (): TabItem[] =>
    methods().map((m) => ({ id: m, label: m, closable: false }));

  const markDirty = () => {
    setDirty(true);
    props.onDirty?.(true);
    autoSave.trigger();
  };

  const updateMethod = (method: HttpMethod, endpoint: EndpointSpec) => {
    setDraft((prev) => ({
      ...prev,
      methods: { ...prev.methods, [method]: endpoint },
    }));
    markDirty();
  };

  const addMethod = (method: HttpMethod) => {
    setDraft((prev) => ({
      ...prev,
      methods: { ...prev.methods, [method]: { handler: { ref: "" } } },
    }));
    setActiveMethod(method);
    markDirty();
  };

  const removeMethod = (method: HttpMethod) => {
    setDraft((prev) => {
      const updated = { ...prev.methods };
      delete updated[method];
      return { ...prev, methods: updated };
    });
    if (activeMethod() === method) {
      const remaining = methods().filter((m) => m !== method);
      setActiveMethod(remaining[0] ?? null);
    }
    markDirty();
  };

  const unusedMethods = () => ALL_METHODS.filter((m) => !draft().methods[m]);

  const startEditPath = () => {
    setPathDraft(draft().path);
    setEditingPath(true);
  };

  const confirmPath = () => {
    const newPath = pathDraft().trim();
    if (newPath && newPath !== draft().path) {
      setDraft((prev) => ({ ...prev, path: newPath }));
      markDirty();
    }
    setEditingPath(false);
  };

  const addMethodItems = (): DropdownItem[] =>
    unusedMethods().map((m) => ({
      label: m,
      action: () => addMethod(m),
    }));

  const handleSave = async () => {
    try {
      await writeRoute(props.filePath, draft());
      setDirty(false);
      props.onDirty?.(false);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Save failed");
    }
  };

  const autoSave = createAutoSave(handleSave);

  return (
    <div class="route-editor">
      {/* Path header */}
      <div class="route-editor-header">
        <Show when={!editingPath()} fallback={
          <input
            class="route-editor-path-input"
            value={pathDraft()}
            onInput={(e) => setPathDraft(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") confirmPath();
              if (e.key === "Escape") setEditingPath(false);
            }}
            onBlur={() => confirmPath()}
            autofocus
          />
        }>
          <span class="route-editor-path" onClick={startEditPath} title="Click to edit path">
            {draft().path}
          </span>
        </Show>
      </div>

      {/* Method badge bar */}
      <div class="route-editor-method-bar">
        <For each={methods()}>
          {(m) => (
            <MethodBadge
              method={m}
              active={activeMethod() === m}
              removable={methods().length > 1}
              onClick={() => setActiveMethod(m)}
              onRemove={() => removeMethod(m)}
            />
          )}
        </For>
        <Show when={unusedMethods().length > 0}>
          <DropdownMenu
            trigger={
              <span class="method-badge-add">+ Add</span>
            }
            items={addMethodItems()}
          />
        </Show>
      </div>

      {/* Params */}
      <div class="route-editor-params">
        <ParamEditor
          params={draft().params ?? {}}
          onChange={(params) => {
            setDraft((prev) => ({ ...prev, params }));
            markDirty();
          }}
        />
      </div>

      {/* Method tabs */}
      <Show when={methods().length > 0}>
        <TabPanel
          tabs={methodTabs()}
          activeId={activeMethod() ?? ""}
          onSelect={(id) => setActiveMethod(id as HttpMethod)}
        />

        <Show when={activeMethod() && draft().methods[activeMethod()!]}>
          <MethodPanel
            method={activeMethod()!}
            endpoint={draft().methods[activeMethod()!]!}
            onChange={(ep) => updateMethod(activeMethod()!, ep)}
          />
        </Show>
      </Show>

      <Show when={methods().length === 0}>
        <div class="route-editor-empty">
          No HTTP methods defined. Add one to get started.
        </div>
      </Show>

      <style>{`
        .route-editor {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .route-editor-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 16px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
          flex-shrink: 0;
        }
        .route-editor-path {
          font-family: var(--rash-font);
          font-size: 14px;
          font-weight: 600;
          color: var(--rash-text);
          cursor: pointer;
        }
        .route-editor-path:hover {
          text-decoration: underline;
          text-decoration-style: dashed;
        }
        .route-editor-path-input {
          font-family: var(--rash-font);
          font-size: 14px;
          font-weight: 600;
          padding: 2px 6px;
          width: 300px;
        }
        .route-editor-method-bar {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 8px 16px;
          border-bottom: 1px solid var(--rash-border);
          flex-shrink: 0;
          flex-wrap: wrap;
        }
        .method-badge-add {
          display: inline-flex;
          align-items: center;
          padding: 3px 8px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 600;
          color: var(--rash-text-muted);
          border: 1px dashed var(--rash-border);
          cursor: pointer;
          transition: all 0.15s ease;
        }
        .method-badge-add:hover {
          color: var(--rash-text-secondary);
          border-color: var(--rash-text-muted);
        }
        .route-editor-params {
          padding: 12px 16px;
          border-bottom: 1px solid var(--rash-border);
          flex-shrink: 0;
        }
        .route-editor-empty {
          display: flex;
          align-items: center;
          justify-content: center;
          flex: 1;
          color: var(--rash-text-muted);
          font-size: 13px;
        }
      `}</style>
    </div>
  );
};
