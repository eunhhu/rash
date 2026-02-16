import { Component, Show } from "solid-js";
import { useProjectStore } from "../../stores/projectStore";
import "./layout.css";

export const TopBar: Component = () => {
  const { project, validateProject } = useProjectStore();

  return (
    <div class="topbar">
      <div class="topbar-left">
        <Show when={project()} fallback={<span class="project-name">Rash</span>}>
          {(p) => <span class="project-name">{p().name}</span>}
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
        <Show when={project()}>
          <button
            class="btn btn-secondary btn-sm"
            onClick={() => validateProject()}
          >
            Validate
          </button>
          <button class="btn btn-primary btn-sm">Build</button>
        </Show>
      </div>
    </div>
  );
};
