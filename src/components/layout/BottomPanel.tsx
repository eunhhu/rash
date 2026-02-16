import { Component, For, Show } from "solid-js";
import { useUiStore } from "../../stores/uiStore";
import { useProjectStore } from "../../stores/projectStore";
import type { ErrorEntry } from "../../ipc/types";
import "./layout.css";

export const BottomPanel: Component = () => {
  const { bottomPanelOpen, setBottomPanelOpen, bottomPanelHeight } = useUiStore();
  const { validationReport } = useProjectStore();

  const entries = (): ErrorEntry[] => validationReport()?.errors ?? [];
  const errorCount = () => entries().filter((e) => e.severity === "error").length;
  const warningCount = () => entries().filter((e) => e.severity === "warning").length;

  return (
    <>
      {/* Toggle button sits at the border between main area and bottom panel */}
      <div
        style={{
          display: "flex",
          "align-items": "center",
          "justify-content": "center",
          background: "var(--rash-bg-secondary)",
          "border-top": "1px solid var(--rash-border)",
          height: "28px",
          "min-height": "28px",
          cursor: "pointer",
          "user-select": "none",
          "font-size": "11px",
          color: "var(--rash-text-muted)",
          gap: "12px",
        }}
        onClick={() => setBottomPanelOpen(!bottomPanelOpen())}
      >
        <span>{bottomPanelOpen() ? "Hide Problems" : "Show Problems"}</span>
        <Show when={errorCount() > 0}>
          <span style={{ color: "var(--rash-error)" }}>
            {errorCount()} error{errorCount() !== 1 ? "s" : ""}
          </span>
        </Show>
        <Show when={warningCount() > 0}>
          <span style={{ color: "var(--rash-warning)" }}>
            {warningCount()} warning{warningCount() !== 1 ? "s" : ""}
          </span>
        </Show>
      </div>

      <div
        class={`bottom-panel ${bottomPanelOpen() ? "" : "closed"}`}
        style={{ height: bottomPanelOpen() ? `${bottomPanelHeight()}px` : "0px" }}
      >
        <div class="bottom-panel-header">
          <span>Problems</span>
          <button
            class="btn-icon"
            onClick={() => setBottomPanelOpen(false)}
            title="Close panel"
          >
            &#x2715;
          </button>
        </div>
        <div class="bottom-panel-body">
          <Show
            when={entries().length > 0}
            fallback={
              <div style={{ color: "var(--rash-text-muted)", "font-size": "12px" }}>
                No problems detected.
              </div>
            }
          >
            <For each={entries()}>
              {(entry) => (
                <div class="validation-entry">
                  <span class={`severity ${entry.severity}`}>
                    {entry.severity}
                  </span>
                  <span class="entry-message">{entry.message}</span>
                  <span class="entry-file">{entry.file}</span>
                </div>
              )}
            </For>
          </Show>
        </div>
      </div>
    </>
  );
};
