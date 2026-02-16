import { Component, For, Show, Switch, Match, createSignal, createEffect } from "solid-js";
import { useEditorStore, type EditorTab } from "../../stores/editorStore";
import { invoke } from "../../ipc/invoke";
import type { RouteSpec, SchemaSpec, ModelSpec, MiddlewareSpec, HandlerSpec } from "../../ipc/types";
import { RouteEditor } from "../route/RouteEditor";
import { SchemaEditor } from "../schema/SchemaEditor";
import { ModelEditor } from "../model/ModelEditor";
import { MiddlewareEditor } from "../middleware/MiddlewareEditor";
import { HandlerEditor } from "../handler/HandlerEditor";
import { SplitPane } from "./SplitPane";
import { CodePreview } from "../preview/CodePreview";
import "./layout.css";

const READ_COMMANDS: Record<string, string> = {
  route: "read_route",
  schema: "read_schema",
  model: "read_model",
  middleware: "read_middleware",
  handler: "read_handler",
};

interface EditorWrapperProps {
  tab: EditorTab;
}

const EditorWrapper: Component<EditorWrapperProps> = (props) => {
  const [data, setData] = createSignal<unknown>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const { markDirty, markClean } = useEditorStore();

  createEffect(() => {
    const cmd = READ_COMMANDS[props.tab.kind];
    if (!cmd) {
      setError(`Unknown editor kind: ${props.tab.kind}`);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    invoke(cmd, { filePath: props.tab.filePath })
      .then((result) => {
        setData(result);
        setLoading(false);
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
        setLoading(false);
      });
  });

  const handleDirty = (dirty: boolean) => {
    if (dirty) markDirty(props.tab.id);
    else markClean(props.tab.id);
  };

  return (
    <Show when={!loading()} fallback={
      <div class="editor-loading">Loading...</div>
    }>
      <Show when={!error()} fallback={
        <div class="editor-error">Error: {error()}</div>
      }>
        <SplitPane direction="horizontal" initialSize={70}>
          <Switch fallback={
            <div class="editor-error">No editor for kind: {props.tab.kind}</div>
          }>
            <Match when={props.tab.kind === "route"}>
              <RouteEditor
                route={data() as RouteSpec}
                filePath={props.tab.filePath}
                onDirty={handleDirty}
              />
            </Match>
            <Match when={props.tab.kind === "schema"}>
              <SchemaEditor
                schema={data() as SchemaSpec}
                filePath={props.tab.filePath}
                onDirty={handleDirty}
              />
            </Match>
            <Match when={props.tab.kind === "model"}>
              <ModelEditor
                model={data() as ModelSpec}
                filePath={props.tab.filePath}
                onDirty={handleDirty}
              />
            </Match>
            <Match when={props.tab.kind === "middleware"}>
              <MiddlewareEditor
                middleware={data() as MiddlewareSpec}
                filePath={props.tab.filePath}
                onDirty={handleDirty}
              />
            </Match>
            <Match when={props.tab.kind === "handler"}>
              <HandlerEditor
                handler={data() as HandlerSpec}
                filePath={props.tab.filePath}
                onDirty={handleDirty}
              />
            </Match>
          </Switch>
          <CodePreview specData={data()} />
        </SplitPane>
      </Show>
    </Show>
  );
};

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
            <EditorWrapper tab={tab()} />
          </div>
        )}
      </Show>
    </div>
  );
};
