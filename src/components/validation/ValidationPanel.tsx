import { Component, Show, createSignal, createMemo } from "solid-js";
import { useProjectStore } from "../../stores/projectStore";
import type { Severity, ErrorEntry } from "../../ipc/types";
import { ErrorList } from "./ErrorList";

interface ValidationPanelProps {
  onClickFile?: (file: string, path: string) => void;
}

type FilterMode = "all" | Severity;

export const ValidationPanel: Component<ValidationPanelProps> = (props) => {
  const { validationReport, validateProject } = useProjectStore();
  const [filter, setFilter] = createSignal<FilterMode>("all");
  const [validating, setValidating] = createSignal(false);

  const errors = createMemo(() => validationReport()?.errors ?? []);

  const counts = createMemo(() => {
    const e = errors();
    return {
      error: e.filter((x) => x.severity === "error").length,
      warning: e.filter((x) => x.severity === "warning").length,
      info: e.filter((x) => x.severity === "info").length,
    };
  });

  const filtered = createMemo(() => {
    const f = filter();
    if (f === "all") return errors();
    return errors().filter((e) => e.severity === f);
  });

  const handleValidate = async () => {
    setValidating(true);
    try {
      await validateProject();
    } finally {
      setValidating(false);
    }
  };

  return (
    <div class="validation-panel">
      <div class="validation-panel-header">
        <div class="validation-panel-filters">
          <button
            class="validation-panel-filter"
            classList={{ "validation-panel-filter-active": filter() === "all" }}
            onClick={() => setFilter("all")}
          >
            All ({errors().length})
          </button>
          <button
            class="validation-panel-filter"
            classList={{ "validation-panel-filter-active": filter() === "error" }}
            onClick={() => setFilter("error")}
          >
            <span class="validation-panel-badge" style={{ background: "var(--rash-error)" }}>
              {counts().error}
            </span>
            Errors
          </button>
          <button
            class="validation-panel-filter"
            classList={{ "validation-panel-filter-active": filter() === "warning" }}
            onClick={() => setFilter("warning")}
          >
            <span class="validation-panel-badge" style={{ background: "var(--rash-warning)" }}>
              {counts().warning}
            </span>
            Warnings
          </button>
          <button
            class="validation-panel-filter"
            classList={{ "validation-panel-filter-active": filter() === "info" }}
            onClick={() => setFilter("info")}
          >
            <span class="validation-panel-badge" style={{ background: "var(--rash-info)" }}>
              {counts().info}
            </span>
            Info
          </button>
        </div>
        <button
          class="btn btn-sm"
          disabled={validating()}
          onClick={handleValidate}
        >
          {validating() ? "Validating..." : "Validate"}
        </button>
      </div>

      <div class="validation-panel-body">
        <Show when={validationReport()} fallback={
          <div class="validation-panel-empty">
            Run validation to check your project for issues.
          </div>
        }>
          <ErrorList errors={filtered()} onClickFile={props.onClickFile} />
        </Show>
      </div>

      <style>{`
        .validation-panel {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .validation-panel-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 6px 12px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
          flex-shrink: 0;
        }
        .validation-panel-filters {
          display: flex;
          gap: 2px;
        }
        .validation-panel-filter {
          display: flex;
          align-items: center;
          gap: 4px;
          padding: 3px 10px;
          border: none;
          background: transparent;
          color: var(--rash-text-muted);
          font-size: 12px;
          font-family: var(--rash-font-ui);
          cursor: pointer;
          border-radius: var(--rash-radius-sm);
          transition: background 0.1s ease, color 0.1s ease;
        }
        .validation-panel-filter:hover {
          background: var(--rash-surface);
          color: var(--rash-text);
        }
        .validation-panel-filter-active {
          background: var(--rash-surface) !important;
          color: var(--rash-text) !important;
        }
        .validation-panel-badge {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          min-width: 18px;
          height: 16px;
          padding: 0 4px;
          border-radius: 8px;
          font-size: 10px;
          font-weight: 700;
          color: var(--rash-bg);
        }
        .validation-panel-body {
          flex: 1;
          overflow: hidden;
        }
        .validation-panel-empty {
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
