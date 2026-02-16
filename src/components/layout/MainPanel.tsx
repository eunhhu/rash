import { Component, For, Show, Switch, Match, createSignal, createEffect, onMount } from "solid-js";
import { FiChevronLeft, FiChevronRight } from "solid-icons/fi";
import { useEditorStore, type EditorTab } from "../../stores/editorStore";
import { readRoute, readSchema, readModel, readMiddleware, readHandler } from "../../ipc/commands";
import type { RouteSpec, SchemaSpec, ModelSpec, MiddlewareSpec, HandlerSpec } from "../../ipc/types";
import { RouteEditor } from "../route/RouteEditor";
import { SchemaEditor } from "../schema/SchemaEditor";
import { ModelEditor } from "../model/ModelEditor";
import { MiddlewareEditor } from "../middleware/MiddlewareEditor";
import { HandlerEditor } from "../handler/HandlerEditor";
import { SplitPane } from "./SplitPane";
import { CodePreview } from "../preview/CodePreview";
import { Spinner } from "../common/Spinner";
import { EmptyState } from "../common/EmptyState";
import { SpecIcon } from "../common/SpecIcon";
import "./layout.css";

const READ_FNS: Record<string, (filePath: string) => Promise<unknown>> = {
  route: readRoute,
  schema: readSchema,
  model: readModel,
  middleware: readMiddleware,
  handler: readHandler,
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
    const readFn = READ_FNS[props.tab.kind];
    if (!readFn) {
      setError(`Unknown editor kind: ${props.tab.kind}`);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    readFn(props.tab.filePath)
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
      <div class="editor-loading">
        <Spinner size="lg" />
      </div>
    }>
      <Show when={!error()} fallback={
        <div class="flex flex-col items-center justify-center flex-1 gap-3">
          <p class="text-xs text-rash-error">{error()}</p>
          <button
            class="carbon-btn-secondary text-xs"
            onClick={() => {
              setLoading(true);
              setError(null);
              const readFn = READ_FNS[props.tab.kind];
              if (readFn) {
                readFn(props.tab.filePath)
                  .then((result) => { setData(result); setLoading(false); })
                  .catch((err) => { setError(err instanceof Error ? err.message : String(err)); setLoading(false); });
              }
            }}
          >
            Retry
          </button>
        </div>
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
  let tabBarRef: HTMLDivElement | undefined;
  const [canScrollLeft, setCanScrollLeft] = createSignal(false);
  const [canScrollRight, setCanScrollRight] = createSignal(false);

  const activeTab = () => tabs.find((t) => t.id === activeTabId());

  const updateScrollState = () => {
    if (!tabBarRef) return;
    setCanScrollLeft(tabBarRef.scrollLeft > 0);
    setCanScrollRight(tabBarRef.scrollLeft + tabBarRef.clientWidth < tabBarRef.scrollWidth - 1);
  };

  const scrollTabs = (dir: "left" | "right") => {
    if (!tabBarRef) return;
    tabBarRef.scrollBy({ left: dir === "left" ? -150 : 150, behavior: "smooth" });
  };

  // Auto-scroll to active tab
  createEffect(() => {
    const id = activeTabId();
    if (!id || !tabBarRef) return;
    const el = tabBarRef.querySelector(`[data-tab-id="${id}"]`) as HTMLElement | null;
    if (el) {
      el.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
    }
    setTimeout(updateScrollState, 100);
  });

  return (
    <div class="main-panel">
      <Show when={tabs.length > 0}>
        <div class="tab-bar-wrapper">
          <Show when={canScrollLeft()}>
            <button class="tab-scroll-btn tab-scroll-left" onClick={() => scrollTabs("left")}>
              <FiChevronLeft size={14} />
            </button>
          </Show>
          <div
            ref={tabBarRef}
            class="tab-bar"
            onScroll={updateScrollState}
          >
            <For each={tabs}>
              {(tab) => (
                <div
                  class={`tab ${tab.id === activeTabId() ? "active" : ""}`}
                  data-tab-id={tab.id}
                  onClick={() => setActiveTab(tab.id)}
                >
                  <Show when={tab.dirty}>
                    <span class="tab-dirty" />
                  </Show>
                  <SpecIcon kind={tab.kind} size={12} />
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
          <Show when={canScrollRight()}>
            <button class="tab-scroll-btn tab-scroll-right" onClick={() => scrollTabs("right")}>
              <FiChevronRight size={14} />
            </button>
          </Show>
        </div>
      </Show>

      <Show
        when={activeTab()}
        fallback={
          <div class="flex flex-col items-center justify-center flex-1 gap-4 p-8 text-center">
            <h3 class="text-sm font-medium text-rash-text-secondary">No editor open</h3>
            <p class="text-xs text-rash-text-muted max-w-[320px]">
              Select an item from the sidebar to start editing, or use a keyboard shortcut:
            </p>
            <div class="flex flex-col gap-2 text-xs text-rash-text-muted">
              <div class="flex items-center gap-3">
                <kbd class="px-1.5 py-0.5 rounded bg-rash-surface text-rash-text-secondary font-mono text-[11px]">Cmd+N</kbd>
                <span>New spec</span>
              </div>
              <div class="flex items-center gap-3">
                <kbd class="px-1.5 py-0.5 rounded bg-rash-surface text-rash-text-secondary font-mono text-[11px]">Cmd+P</kbd>
                <span>Search specs</span>
              </div>
              <div class="flex items-center gap-3">
                <kbd class="px-1.5 py-0.5 rounded bg-rash-surface text-rash-text-secondary font-mono text-[11px]">Cmd+W</kbd>
                <span>Close tab</span>
              </div>
            </div>
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
