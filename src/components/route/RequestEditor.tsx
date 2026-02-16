import { Component, For } from "solid-js";
import type { RequestSpec } from "../../ipc/types";

interface RequestEditorProps {
  request: RequestSpec;
  onChange: (request: RequestSpec) => void;
}

export const RequestEditor: Component<RequestEditorProps> = (props) => {
  const headers = () => Object.entries(props.request.headers ?? {});

  const updateQuery = (ref: string) => {
    props.onChange({ ...props.request, query: ref ? { ref } : undefined });
  };

  const updateBody = (field: "ref" | "contentType", value: string) => {
    const current = props.request.body ?? { ref: "" };
    const updated = { ...current, [field]: value };
    props.onChange({ ...props.request, body: updated.ref ? updated : undefined });
  };

  const addHeader = () => {
    const updated = { ...(props.request.headers ?? {}), "X-Custom": "" };
    props.onChange({ ...props.request, headers: updated });
  };

  const updateHeaderKey = (oldKey: string, newKey: string) => {
    if (!newKey || newKey === oldKey) return;
    const updated: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(props.request.headers ?? {})) {
      updated[k === oldKey ? newKey : k] = v;
    }
    props.onChange({ ...props.request, headers: updated });
  };

  const updateHeaderValue = (key: string, value: string) => {
    const updated = { ...(props.request.headers ?? {}), [key]: value };
    props.onChange({ ...props.request, headers: updated });
  };

  const removeHeader = (key: string) => {
    const updated = { ...(props.request.headers ?? {}) };
    delete (updated as Record<string, unknown>)[key];
    props.onChange({ ...props.request, headers: updated });
  };

  return (
    <div class="request-editor">
      {/* Query */}
      <div class="request-editor-section">
        <label class="request-editor-label">Query Schema Ref</label>
        <input
          value={props.request.query?.ref ?? ""}
          placeholder="schemas/QueryDto"
          onInput={(e) => updateQuery(e.currentTarget.value)}
        />
      </div>

      {/* Body */}
      <div class="request-editor-section">
        <label class="request-editor-label">Request Body</label>
        <div class="request-editor-row">
          <input
            value={props.request.body?.ref ?? ""}
            placeholder="schemas/CreateDto"
            onInput={(e) => updateBody("ref", e.currentTarget.value)}
          />
          <select
            value={props.request.body?.contentType ?? "application/json"}
            onChange={(e) => updateBody("contentType", e.currentTarget.value)}
          >
            <option value="application/json">application/json</option>
            <option value="multipart/form-data">multipart/form-data</option>
            <option value="application/x-www-form-urlencoded">application/x-www-form-urlencoded</option>
          </select>
        </div>
      </div>

      {/* Headers */}
      <div class="request-editor-section">
        <div class="request-editor-section-header">
          <label class="request-editor-label">Headers</label>
          <button class="btn btn-sm" onClick={addHeader}>+ Add</button>
        </div>
        <table class="request-editor-table">
          <thead>
            <tr>
              <th>Key</th>
              <th>Value</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <For each={headers()}>
              {([key, value]) => (
                <tr>
                  <td>
                    <input
                      value={key}
                      onBlur={(e) => updateHeaderKey(key, e.currentTarget.value)}
                    />
                  </td>
                  <td>
                    <input
                      value={String(value ?? "")}
                      onInput={(e) => updateHeaderValue(key, e.currentTarget.value)}
                    />
                  </td>
                  <td>
                    <button class="btn-icon" onClick={() => removeHeader(key)}>{"\u00D7"}</button>
                  </td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>

      <style>{`
        .request-editor {
          display: flex;
          flex-direction: column;
          gap: 16px;
        }
        .request-editor-section {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .request-editor-section-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .request-editor-label {
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .request-editor-row {
          display: flex;
          gap: 8px;
        }
        .request-editor-row input {
          flex: 1;
        }
        .request-editor-row select {
          flex-shrink: 0;
          width: 200px;
        }
        .request-editor-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 12px;
        }
        .request-editor-table th {
          text-align: left;
          padding: 4px 8px;
          color: var(--rash-text-muted);
          font-weight: 500;
          border-bottom: 1px solid var(--rash-border);
        }
        .request-editor-table td {
          padding: 4px 8px;
          border-bottom: 1px solid var(--rash-border);
        }
        .request-editor-table input {
          width: 100%;
          padding: 3px 6px;
          font-size: 12px;
        }
      `}</style>
    </div>
  );
};
