import { Component, Show } from "solid-js";
import { useProjectStore } from "../../stores/projectStore";
import { useRuntimeStore } from "../../stores/runtimeStore";
import "./layout.css";

interface TopBarProps {
  onImport?: () => void;
  onExport?: () => void;
}

export const TopBar: Component<TopBarProps> = (props) => {
  const { project, validateProject } = useProjectStore();
  const { serverStatus, building, build, stop } = useRuntimeStore();

  const isRunning = () => serverStatus() === "running";
  const isTransitioning = () =>
    serverStatus() === "starting" || serverStatus() === "stopping";

  return (
    <div class="topbar">
      <div class="topbar-left">
        <Show when={project()} fallback={<span class="project-name">Rash</span>}>
          {(p) => (
            <span class="project-name">
              <span
                class={`status-dot status-${serverStatus()}`}
                title={serverStatus()}
              />
              {p().name}
            </span>
          )}
        </Show>
      </div>

      <div class="topbar-center">
        <Show when={project()}>
          {(p) => {
            const target = () => (p().config as Record<string, unknown>)?.target as Record<string, unknown> | undefined;
            return (
              <>
                <span class="language-badge">{String(target()?.language ?? "")}</span>
                <span class="framework-badge">{String(target()?.framework ?? "")}</span>
              </>
            );
          }}
        </Show>
      </div>

      <div class="topbar-right">
        <button
          class="btn btn-secondary btn-sm"
          onClick={() => props.onImport?.()}
        >
          Import
        </button>
        <Show when={project()}>
          <button
            class="btn btn-secondary btn-sm"
            onClick={() => props.onExport?.()}
          >
            Export
          </button>
          <button
            class="btn btn-secondary btn-sm"
            onClick={() => validateProject()}
          >
            Validate
          </button>
          <Show
            when={!isRunning()}
            fallback={
              <button
                class="btn btn-danger btn-sm"
                disabled={isTransitioning()}
                onClick={() => stop()}
              >
                Stop
              </button>
            }
          >
            <button
              class="btn btn-primary btn-sm"
              disabled={building() || isTransitioning()}
              onClick={() => build()}
            >
              {building() ? "Building..." : "Build"}
            </button>
          </Show>
        </Show>
      </div>
    </div>
  );
};
