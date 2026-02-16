import { Component, For, Show } from "solid-js";
import { useEditorStore } from "../../stores/editorStore";
import "./layout.css";

export const MainPanel: Component = () => {
  const { tabs, activeTabId, setActiveTab, closeTab } = useEditorStore();

  const activeTab = () => tabs.find((t) => t.id === activeTabId());

  return (
    <div class="main-panel">
      <Show when={tabs.length > 0}>
        <div class="tab-bar">
          <For each={tabs}>
            {(tab) => (
              <div
                class={`tab ${tab.id === activeTabId() ? "active" : ""}`}
                onClick={() => setActiveTab(tab.id)}
              >
                <Show when={tab.dirty}>
                  <span class="tab-dirty" />
                </Show>
                <span>{tab.label}</span>
                <span
                  class="tab-close"
                  onClick={(e) => {
                    e.stopPropagation();
                    closeTab(tab.id);
                  }}
                >
                  &#x2715;
                </span>
              </div>
            )}
          </For>
        </div>
      </Show>

      <Show
        when={activeTab()}
        fallback={
          <div class="main-panel-empty">
            Select an item from the sidebar to start editing
          </div>
        }
      >
        {(tab) => (
          <div class="editor-area">
            <EditorPlaceholder kind={tab().kind} name={tab().label} />
          </div>
        )}
      </Show>
    </div>
  );
};

/**
 * Placeholder that renders based on tab kind.
 * Each kind will be replaced by a dedicated editor component later.
 */
const EditorPlaceholder: Component<{ kind: string; name: string }> = (props) => {
  return (
    <div style={{ color: "var(--rash-text-secondary)" }}>
      <h3 style={{ "margin-bottom": "8px", color: "var(--rash-text)" }}>
        {props.name}
      </h3>
      <p>
        Editor for <strong>{props.kind}</strong> will be rendered here.
      </p>
    </div>
  );
};
