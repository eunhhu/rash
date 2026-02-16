import { Component, Show, createSignal, createEffect, For } from "solid-js";
import { writeRoute } from "../../ipc/commands";
import type { RouteSpec, HttpMethod, EndpointSpec } from "../../ipc/types";
import { Badge } from "../common/Badge";
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
  const [draft, setDraft] = createSignal<RouteSpec>(structuredClone(props.route));
  const [activeMethod, setActiveMethod] = createSignal<HttpMethod | null>(null);
  const [dirty, setDirty] = createSignal(false);
  const [saving, setSaving] = createSignal(false);

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

  const handleSave = async () => {
    setSaving(true);
    try {
      await writeRoute(props.filePath, draft());
      setDirty(false);
      props.onDirty?.(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="route-editor">
      {/* Path header */}
      <div class="route-editor-header">
        <span class="route-editor-path">{draft().path}</span>
        <div class="route-editor-actions">
          <Show when={unusedMethods().length > 0}>
            <select
              class="route-editor-add-method"
              value=""
              onChange={(e) => {
                const v = e.currentTarget.value as HttpMethod;
                if (v) addMethod(v);
                e.currentTarget.value = "";
              }}
            >
              <option value="" disabled>+ Add method</option>
              <For each={unusedMethods()}>
                {(m) => <option value={m}>{m}</option>}
              </For>
            </select>
          </Show>
          <button
            class="btn btn-primary btn-sm"
            disabled={!dirty() || saving()}
            onClick={handleSave}
          >
            {saving() ? "Saving..." : "Save"}
          </button>
        </div>
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
        }
        .route-editor-actions {
          display: flex;
          gap: 8px;
          align-items: center;
        }
        .route-editor-add-method {
          font-size: 12px;
          padding: 4px 8px;
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
