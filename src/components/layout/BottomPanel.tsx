import { Component, For, Show, createSignal } from "solid-js";
import { useUiStore } from "../../stores/uiStore";
import { useProjectStore } from "../../stores/projectStore";
import { useRuntimeStore } from "../../stores/runtimeStore";
import type { ErrorEntry } from "../../ipc/types";
import "./layout.css";

type BottomTab = "problems" | "logs";

export const BottomPanel: Component = () => {
  const { bottomPanelOpen, setBottomPanelOpen, bottomPanelHeight } = useUiStore();
  const { validationReport } = useProjectStore();
  const { logs, clearLogs } = useRuntimeStore();
  const [activeTab, setActiveTab] = createSignal<BottomTab>("problems");

  const entries = (): ErrorEntry[] => validationReport()?.errors ?? [];
  const errorCount = () => entries().filter((e) => e.severity === "error").length;
  const warningCount = () => entries().filter((e) => e.severity === "warning").length;
  const logCount = () => logs().length;

  return (
    <>
      {/* Toggle bar */}
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
        <span>{bottomPanelOpen() ? "Hide Panel" : "Show Panel"}</span>
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
        <Show when={logCount() > 0}>
          <span style={{ color: "var(--rash-text-secondary)" }}>
            {logCount()} log{logCount() !== 1 ? "s" : ""}
          </span>
        </Show>
      </div>

      <div
        class={`bottom-panel ${bottomPanelOpen() ? "" : "closed"}`}
        style={{ height: bottomPanelOpen() ? `${bottomPanelHeight()}px` : "0px" }}
      >
        <div class="bottom-panel-header">
          <div class="bottom-panel-tabs">
            <button
              class={`bottom-panel-tab ${activeTab() === "problems" ? "active" : ""}`}
              onClick={() => setActiveTab("problems")}
            >
              Problems
            </button>
            <button
              class={`bottom-panel-tab ${activeTab() === "logs" ? "active" : ""}`}
              onClick={() => setActiveTab("logs")}
            >
              Logs
            </button>
          </div>
          <div style={{ display: "flex", gap: "4px", "align-items": "center" }}>
            <Show when={activeTab() === "logs"}>
              <button
                class="btn-icon"
                onClick={() => clearLogs()}
                title="Clear logs"
              >
                Clear
              </button>
            </Show>
            <button
              class="btn-icon"
              onClick={() => setBottomPanelOpen(false)}
              title="Close panel"
            >
              &#x2715;
            </button>
          </div>
        </div>
        <div class="bottom-panel-body">
          <Show when={activeTab() === "problems"}>
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
          </Show>

          <Show when={activeTab() === "logs"}>
            <Show
              when={logs().length > 0}
              fallback={
                <div style={{ color: "var(--rash-text-muted)", "font-size": "12px" }}>
                  No server logs yet.
                </div>
              }
            >
              <For each={logs()}>
                {(log) => (
                  <div class="log-entry">
                    <span class="log-time">
                      {new Date(log.timestamp).toLocaleTimeString()}
                    </span>
                    <span class={`log-level log-level-${log.level}`}>
                      {log.level}
                    </span>
                    <span class="log-message">{log.message}</span>
                  </div>
                )}
              </For>
            </Show>
          </Show>
        </div>
      </div>
    </>
  );
};
