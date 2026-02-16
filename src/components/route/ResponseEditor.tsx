import { Component, For } from "solid-js";
import type { ResponseSpec } from "../../ipc/types";

interface ResponseEditorProps {
  responses: Record<string, ResponseSpec>;
  onChange: (responses: Record<string, ResponseSpec>) => void;
}

const COMMON_STATUS_CODES = ["200", "201", "204", "400", "401", "403", "404", "409", "500"];

export const ResponseEditor: Component<ResponseEditorProps> = (props) => {
  const entries = () =>
    Object.entries(props.responses).sort(([a], [b]) => Number(a) - Number(b));

  const addResponse = () => {
    const used = new Set(Object.keys(props.responses));
    const next = COMMON_STATUS_CODES.find((c) => !used.has(c)) ?? "200";
    props.onChange({
      ...props.responses,
      [next]: { description: "" },
    });
  };

  const removeResponse = (code: string) => {
    const updated = { ...props.responses };
    delete updated[code];
    props.onChange(updated);
  };

  const updateCode = (oldCode: string, newCode: string) => {
    if (!newCode || newCode === oldCode) return;
    const updated: Record<string, ResponseSpec> = {};
    for (const [key, val] of Object.entries(props.responses)) {
      updated[key === oldCode ? newCode : key] = val;
    }
    props.onChange(updated);
  };

  const updateField = (code: string, field: "description" | "schemaRef", value: string) => {
    const entry = { ...props.responses[code] };
    if (field === "description") {
      entry.description = value;
    } else if (field === "schemaRef") {
      entry.schema = value ? { ref: value } : undefined;
    }
    props.onChange({ ...props.responses, [code]: entry });
  };

  return (
    <div class="response-editor">
      <div class="response-editor-header">
        <span class="response-editor-title">Responses</span>
        <button class="btn btn-sm" onClick={addResponse}>+ Add Status</button>
      </div>

      <div class="response-editor-list">
        <For each={entries()}>
          {([code, spec]) => (
            <div class="response-editor-item">
              <div class="response-editor-item-header">
                <input
                  class="response-editor-code"
                  value={code}
                  onBlur={(e) => updateCode(code, e.currentTarget.value)}
                />
                <span class="response-editor-status-hint">
                  {code.startsWith("2") ? "Success" : code.startsWith("4") ? "Client Error" : code.startsWith("5") ? "Server Error" : ""}
                </span>
                <button class="btn-icon" onClick={() => removeResponse(code)}>{"\u00D7"}</button>
              </div>
              <div class="response-editor-fields">
                <div class="response-editor-field">
                  <label>Description</label>
                  <input
                    value={spec.description ?? ""}
                    placeholder="Response description"
                    onInput={(e) => updateField(code, "description", e.currentTarget.value)}
                  />
                </div>
                <div class="response-editor-field">
                  <label>Schema Ref</label>
                  <input
                    value={spec.schema?.ref ?? ""}
                    placeholder="schemas/ResponseDto"
                    onInput={(e) => updateField(code, "schemaRef", e.currentTarget.value)}
                  />
                </div>
              </div>
            </div>
          )}
        </For>
      </div>

      <style>{`
        .response-editor {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .response-editor-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .response-editor-title {
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .response-editor-list {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .response-editor-item {
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
          padding: 8px;
          background: var(--rash-bg-secondary);
        }
        .response-editor-item-header {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-bottom: 8px;
        }
        .response-editor-code {
          width: 60px;
          font-family: var(--rash-font);
          font-weight: 700;
          text-align: center;
        }
        .response-editor-status-hint {
          flex: 1;
          font-size: 11px;
          color: var(--rash-text-muted);
        }
        .response-editor-fields {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .response-editor-field {
          display: flex;
          flex-direction: column;
          gap: 2px;
        }
        .response-editor-field label {
          font-size: 11px;
          color: var(--rash-text-muted);
        }
        .response-editor-field input {
          font-size: 12px;
          padding: 4px 8px;
        }
      `}</style>
    </div>
  );
};
