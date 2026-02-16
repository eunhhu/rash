import { Component, Show } from "solid-js";
import { FiPlus, FiUpload, FiDownload, FiCheckCircle, FiPlay, FiSquare, FiMoreHorizontal } from "solid-icons/fi";
import { TbOutlineRoute2, TbOutlineBraces, TbOutlineDatabase, TbOutlineFunction } from "solid-icons/tb";
import { FiLayers } from "solid-icons/fi";
import { useProjectStore } from "../../stores/projectStore";
import { useEditorStore } from "../../stores/editorStore";
import { useRuntimeStore } from "../../stores/runtimeStore";
import { useUiStore } from "../../stores/uiStore";
import { SpecIcon } from "../common/SpecIcon";
import { DropdownMenu, type DropdownItem } from "../common/DropdownMenu";
import "./layout.css";

interface TopBarProps {
  onImport?: () => void;
  onExport?: () => void;
}

export const TopBar: Component<TopBarProps> = (props) => {
  const { project, validateProject } = useProjectStore();
  const { tabs, activeTabId } = useEditorStore();
  const { serverStatus, building, build, stop } = useRuntimeStore();
  const { startCreate } = useUiStore();

  const isRunning = () => serverStatus() === "running";
  const isTransitioning = () =>
    serverStatus() === "starting" || serverStatus() === "stopping";

  const activeTab = () => tabs.find((t) => t.id === activeTabId());

  const newSpecItems: DropdownItem[] = [
    {
      label: "Route",
      icon: <TbOutlineRoute2 size={14} />,
      action: () => startCreate("route"),
    },
    {
      label: "Schema",
      icon: <TbOutlineBraces size={14} />,
      action: () => startCreate("schema"),
    },
    {
      label: "Model",
      icon: <TbOutlineDatabase size={14} />,
      action: () => startCreate("model"),
    },
    {
      label: "Middleware",
      icon: <FiLayers size={14} />,
      action: () => startCreate("middleware"),
    },
    {
      label: "Handler",
      icon: <TbOutlineFunction size={14} />,
      action: () => startCreate("handler"),
    },
    { type: "separator", label: "" },
    {
      label: "New Project",
      icon: <FiPlus size={14} />,
      action: () => {
        const store = useProjectStore();
        store.setShowCreateDialog(true);
      },
    },
  ];

  const moreItems = (): DropdownItem[] => [
    {
      label: "Import OpenAPI",
      icon: <FiUpload size={14} />,
      action: () => props.onImport?.(),
    },
    {
      label: "Export OpenAPI",
      icon: <FiDownload size={14} />,
      action: () => props.onExport?.(),
    },
    { type: "separator", label: "" },
    {
      label: "Validate Project",
      icon: <FiCheckCircle size={14} />,
      action: () => validateProject(),
    },
  ];

  return (
    <div class="topbar">
      <div class="topbar-left">
        <Show when={project()}>
          <DropdownMenu
            trigger={
              <button class="carbon-btn-secondary text-xs flex items-center gap-1.5">
                <FiPlus size={14} />
                <span>New</span>
              </button>
            }
            items={newSpecItems}
          />
        </Show>
        <Show when={!project()}>
          <span class="project-name">Rash</span>
        </Show>
      </div>

      <div class="topbar-center">
        <Show when={project()}>
          {(p) => {
            const target = () =>
              (p().config as Record<string, unknown>)?.target as
                | Record<string, unknown>
                | undefined;
            return (
              <>
                <span class="text-xs font-medium text-rash-text">{p().name}</span>
                <Show when={activeTab()}>
                  {(tab) => (
                    <>
                      <span class="text-rash-text-muted">/</span>
                      <SpecIcon kind={tab().kind} size={12} />
                      <span class="text-xs text-rash-text-secondary">{tab().kind}s</span>
                      <span class="text-rash-text-muted">/</span>
                      <span class="text-xs text-rash-text-secondary">{tab().label}</span>
                    </>
                  )}
                </Show>
                <Show when={target()?.language}>
                  <span class="language-badge">{String(target()?.language ?? "")}</span>
                </Show>
                <Show when={target()?.framework}>
                  <span class="framework-badge">{String(target()?.framework ?? "")}</span>
                </Show>
              </>
            );
          }}
        </Show>
      </div>

      <div class="topbar-right">
        <Show when={project()}>
          {/* Server status */}
          <div class="flex items-center gap-1.5 text-xs text-rash-text-secondary mr-1">
            <span
              class={`status-dot status-${serverStatus()}`}
            />
            <span class="capitalize">{serverStatus()}</span>
          </div>

          <Show
            when={!isRunning()}
            fallback={
              <button
                class="carbon-btn-danger text-xs"
                disabled={isTransitioning()}
                onClick={() => stop()}
              >
                <FiSquare size={12} />
                Stop
              </button>
            }
          >
            <button
              class="carbon-btn-primary text-xs"
              disabled={building() || isTransitioning()}
              onClick={() => build()}
            >
              <FiPlay size={12} />
              {building() ? "Building..." : "Build"}
            </button>
          </Show>

          <DropdownMenu
            trigger={
              <button class="carbon-btn-icon">
                <FiMoreHorizontal size={16} />
              </button>
            }
            items={moreItems()}
          />
        </Show>
        <Show when={!project()}>
          <button
            class="carbon-btn-secondary text-xs"
            onClick={() => props.onImport?.()}
          >
            Import
          </button>
        </Show>
      </div>
    </div>
  );
};
