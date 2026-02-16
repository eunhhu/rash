import { Component, Show } from "solid-js";
import type { EndpointSpec, HttpMethod, RequestSpec, ResponseSpec } from "../../ipc/types";
import { RequestEditor } from "./RequestEditor";
import { ResponseEditor } from "./ResponseEditor";

interface MethodPanelProps {
  method: HttpMethod;
  endpoint: EndpointSpec;
  onChange: (endpoint: EndpointSpec) => void;
}

export const MethodPanel: Component<MethodPanelProps> = (props) => {
  const update = <K extends keyof EndpointSpec>(key: K, value: EndpointSpec[K]) => {
    props.onChange({ ...props.endpoint, [key]: value });
  };

  const addMiddleware = () => {
    const current = props.endpoint.middleware ?? [];
    update("middleware", [...current, { ref: "" }]);
  };

  const removeMiddleware = (index: number) => {
    const current = [...(props.endpoint.middleware ?? [])];
    current.splice(index, 1);
    update("middleware", current);
  };

  const updateMiddlewareRef = (index: number, ref: string) => {
    const current = [...(props.endpoint.middleware ?? [])];
    current[index] = { ...current[index], ref };
    update("middleware", current);
  };

  return (
    <div class="method-panel">
      {/* operationId */}
      <div class="method-panel-field">
        <label>Operation ID</label>
        <input
          value={props.endpoint.operationId ?? ""}
          placeholder={`${props.method.toLowerCase()}Resource`}
          onInput={(e) => update("operationId", e.currentTarget.value)}
        />
      </div>

      {/* summary */}
      <div class="method-panel-field">
        <label>Summary</label>
        <input
          value={props.endpoint.summary ?? ""}
          placeholder="Short description"
          onInput={(e) => update("summary", e.currentTarget.value)}
        />
      </div>

      {/* handler ref */}
      <div class="method-panel-field">
        <label>Handler Ref</label>
        <input
          value={props.endpoint.handler?.ref ?? ""}
          placeholder="handlers/myHandler"
          onInput={(e) => update("handler", { ref: e.currentTarget.value })}
        />
      </div>

      {/* middleware */}
      <div class="method-panel-section">
        <div class="method-panel-section-header">
          <label>Middleware</label>
          <button class="btn btn-sm" onClick={addMiddleware}>+ Add</button>
        </div>
        <div class="method-panel-middleware-list">
          {(props.endpoint.middleware ?? []).map((mw, i) => (
            <div class="method-panel-middleware-row">
              <input
                value={mw.ref}
                placeholder="middleware/auth"
                onInput={(e) => updateMiddlewareRef(i, e.currentTarget.value)}
              />
              <button class="btn-icon" onClick={() => removeMiddleware(i)}>{"\u00D7"}</button>
            </div>
          ))}
        </div>
      </div>

      {/* request */}
      <div class="method-panel-section">
        <RequestEditor
          request={props.endpoint.request ?? {}}
          onChange={(req) => update("request", req)}
        />
      </div>

      {/* response */}
      <div class="method-panel-section">
        <ResponseEditor
          responses={props.endpoint.response ?? {}}
          onChange={(res) => update("response", res)}
        />
      </div>

      <style>{`
        .method-panel {
          display: flex;
          flex-direction: column;
          gap: 16px;
          padding: 16px;
          overflow-y: auto;
        }
        .method-panel-field {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .method-panel-field label,
        .method-panel-section label {
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text-secondary);
        }
        .method-panel-section {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .method-panel-section-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .method-panel-middleware-list {
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .method-panel-middleware-row {
          display: flex;
          gap: 6px;
          align-items: center;
        }
        .method-panel-middleware-row input {
          flex: 1;
        }
      `}</style>
    </div>
  );
};
