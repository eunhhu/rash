import { Component, For, Show, createSignal, createEffect } from "solid-js";
import { invoke } from "../../ipc/invoke";
import type { MiddlewareSpec, MiddlewareType, MiddlewareError } from "../../ipc/types";

interface MiddlewareEditorProps {
  middleware: MiddlewareSpec;
  filePath: string;
  onDirty?: (dirty: boolean) => void;
}

const MIDDLEWARE_TYPES: MiddlewareType[] = ["request", "response", "error", "composed"];

export const MiddlewareEditor: Component<MiddlewareEditorProps> = (props) => {
  const [draft, setDraft] = createSignal<MiddlewareSpec>(structuredClone(props.middleware));
  const [dirty, setDirty] = createSignal(false);
  const [saving, setSaving] = createSignal(false);

  createEffect(() => {
    setDraft(structuredClone(props.middleware));
    setDirty(false);
  });

  const markDirty = () => {
    setDirty(true);
    props.onDirty?.(true);
  };

  const update = <K extends keyof MiddlewareSpec>(key: K, value: MiddlewareSpec[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
    markDirty();
  };

  // Provides
  const providesEntries = () => Object.entries(draft().provides ?? {});

  const addProvide = () => {
    update("provides", { ...(draft().provides ?? {}), newKey: "" });
  };

  const removeProvide = (key: string) => {
    const updated = { ...(draft().provides ?? {}) };
    delete updated[key];
    update("provides", updated);
  };

  const updateProvideKey = (oldKey: string, newKey: string) => {
    if (!newKey || newKey === oldKey) return;
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(draft().provides ?? {})) {
      result[k === oldKey ? newKey : k] = v;
    }
    update("provides", result);
  };

  const updateProvideValue = (key: string, value: string) => {
    update("provides", { ...(draft().provides ?? {}), [key]: value });
  };

  // Errors
  const errorEntries = () => Object.entries(draft().errors ?? {});

  const addError = () => {
    update("errors", { ...(draft().errors ?? {}), NEW_ERROR: { status: 400, message: "" } });
  };

  const removeError = (key: string) => {
    const updated = { ...(draft().errors ?? {}) };
    delete updated[key];
    update("errors", updated);
  };

  const updateErrorKey = (oldKey: string, newKey: string) => {
    if (!newKey || newKey === oldKey) return;
    const result: Record<string, MiddlewareError> = {};
    for (const [k, v] of Object.entries(draft().errors ?? {})) {
      result[k === oldKey ? newKey : k] = v;
    }
    update("errors", result);
  };

  const updateErrorField = (key: string, field: keyof MiddlewareError, value: unknown) => {
    const updated = { ...(draft().errors ?? {}) };
    updated[key] = { ...updated[key], [field]: value };
    update("errors", updated);
  };

  // Compose refs (for composed type)
  const composeRefs = () => draft().compose ?? [];

  const addComposeRef = () => {
    update("compose", [...composeRefs(), { ref: "" }]);
  };

  const removeComposeRef = (i: number) => {
    const updated = [...composeRefs()];
    updated.splice(i, 1);
    update("compose", updated);
  };

  const updateComposeRef = (i: number, ref: string) => {
    const updated = [...composeRefs()];
    updated[i] = { ...updated[i], ref };
    update("compose", updated);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await invoke("write_middleware", { filePath: props.filePath, value: draft() });
      setDirty(false);
      props.onDirty?.(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="mw-editor">
      <div class="mw-editor-header">
        <span class="mw-editor-name">{draft().name}</span>
        <button
          class="btn btn-primary btn-sm"
          disabled={!dirty() || saving()}
          onClick={handleSave}
        >
          {saving() ? "Saving..." : "Save"}
        </button>
      </div>

      <div class="mw-editor-content">
        {/* Name */}
        <div class="mw-editor-field">
          <label>Name</label>
          <input
            value={draft().name}
            onInput={(e) => update("name", e.currentTarget.value)}
          />
        </div>

        {/* Type */}
        <div class="mw-editor-field">
          <label>Type</label>
          <select
            value={draft().type}
            onChange={(e) => update("type", e.currentTarget.value as MiddlewareType)}
          >
            <For each={MIDDLEWARE_TYPES}>{(t) => <option value={t}>{t}</option>}</For>
          </select>
        </div>

        {/* Description */}
        <div class="mw-editor-field">
          <label>Description</label>
          <textarea
            rows={2}
            value={draft().description ?? ""}
            onInput={(e) => update("description", e.currentTarget.value || undefined)}
          />
        </div>

        {/* Handler ref (non-composed) */}
        <Show when={draft().type !== "composed"}>
          <div class="mw-editor-field">
            <label>Handler Ref</label>
            <input
              value={draft().handler?.ref ?? ""}
              placeholder="handlers/authMiddleware"
              onInput={(e) => update("handler", e.currentTarget.value ? { ref: e.currentTarget.value } : undefined)}
            />
          </div>
        </Show>

        {/* Compose refs (composed type) */}
        <Show when={draft().type === "composed"}>
          <div class="mw-editor-section">
            <div class="mw-editor-section-header">
              <label>Composed Middleware</label>
              <button class="btn btn-sm" onClick={addComposeRef}>+ Add</button>
            </div>
            <For each={composeRefs()}>
              {(ref, i) => (
                <div class="mw-editor-row">
                  <input
                    value={ref.ref}
                    placeholder="middleware/name"
                    onInput={(e) => updateComposeRef(i(), e.currentTarget.value)}
                  />
                  <button class="btn-icon" onClick={() => removeComposeRef(i())}>{"\u00D7"}</button>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* Provides */}
        <div class="mw-editor-section">
          <div class="mw-editor-section-header">
            <label>Provides (context keys)</label>
            <button class="btn btn-sm" onClick={addProvide}>+ Add</button>
          </div>
          <For each={providesEntries()}>
            {([key, val]) => (
              <div class="mw-editor-row">
                <input
                  value={key}
                  onBlur={(e) => updateProvideKey(key, e.currentTarget.value)}
                  placeholder="key"
                />
                <input
                  value={String(val ?? "")}
                  onInput={(e) => updateProvideValue(key, e.currentTarget.value)}
                  placeholder="type/description"
                />
                <button class="btn-icon" onClick={() => removeProvide(key)}>{"\u00D7"}</button>
              </div>
            )}
          </For>
        </div>

        {/* Errors */}
        <div class="mw-editor-section">
          <div class="mw-editor-section-header">
            <label>Errors</label>
            <button class="btn btn-sm" onClick={addError}>+ Add</button>
          </div>
          <For each={errorEntries()}>
            {([key, err]) => (
              <div class="mw-editor-error-row">
                <input
                  class="mw-editor-error-key"
                  value={key}
                  onBlur={(e) => updateErrorKey(key, e.currentTarget.value)}
                />
                <input
                  class="mw-editor-error-status"
                  type="number"
                  value={err.status}
                  onInput={(e) => updateErrorField(key, "status", Number(e.currentTarget.value))}
                />
                <input
                  class="mw-editor-error-msg"
                  value={err.message}
                  placeholder="Error message"
                  onInput={(e) => updateErrorField(key, "message", e.currentTarget.value)}
                />
                <button class="btn-icon" onClick={() => removeError(key)}>{"\u00D7"}</button>
              </div>
            )}
          </For>
        </div>
      </div>

      <style>{`
        .mw-editor {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .mw-editor-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 16px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
          flex-shrink: 0;
        }
        .mw-editor-name {
          font-size: 14px;
          font-weight: 600;
          color: var(--rash-text);
        }
        .mw-editor-content {
          flex: 1;
          padding: 16px;
          overflow-y: auto;
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
        .mw-editor-field {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .mw-editor-field label,
        .mw-editor-section label {
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .mw-editor-section {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .mw-editor-section-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .mw-editor-row {
          display: flex;
          gap: 6px;
          align-items: center;
        }
        .mw-editor-row input {
          flex: 1;
        }
        .mw-editor-error-row {
          display: flex;
          gap: 6px;
          align-items: center;
        }
        .mw-editor-error-key {
          flex: 1;
          font-family: var(--rash-font);
          font-size: 12px;
        }
        .mw-editor-error-status {
          width: 60px;
          text-align: center;
        }
        .mw-editor-error-msg {
          flex: 2;
        }
      `}</style>
    </div>
  );
};
