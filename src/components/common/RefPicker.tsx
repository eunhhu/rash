import { Component, For, Show, createSignal, createEffect, onCleanup } from "solid-js";
import { useProjectStore } from "../../stores/projectStore";
import { SpecIcon } from "./SpecIcon";
import type { TreeNode } from "../../ipc/types";

const SECTION_IDS: Record<string, string> = {
  route: "section:routes",
  schema: "section:schemas",
  model: "section:models",
  middleware: "section:middleware",
  handler: "section:handlers",
};

interface RefPickerProps {
  kind: string;
  value: string;
  onChange: (ref: string) => void;
  placeholder?: string;
}

export const RefPicker: Component<RefPickerProps> = (props) => {
  const project = useProjectStore();
  const [open, setOpen] = createSignal(false);
  const [search, setSearch] = createSignal("");
  let containerRef: HTMLDivElement | undefined;

  const nodes = (): TreeNode[] => {
    const tree = project.project();
    if (!tree) return [];
    const sectionId = SECTION_IDS[props.kind];
    const section = tree.nodes.find((n) => n.id === sectionId);
    return section?.children ?? [];
  };

  const filteredNodes = () => {
    const q = search().toLowerCase();
    if (!q) return nodes();
    return nodes().filter((n) => n.label.toLowerCase().includes(q));
  };

  const selectedLabel = () => {
    if (!props.value) return "";
    const node = nodes().find((n) => {
      return n.path === props.value || n.label === props.value ||
        (n.path && props.value.endsWith(n.label));
    });
    return node?.label ?? props.value;
  };

  // Close on outside click
  createEffect(() => {
    if (!open()) return;
    const handleClick = (e: MouseEvent) => {
      if (containerRef && !containerRef.contains(e.target as Node)) {
        setOpen(false);
        setSearch("");
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
        setSearch("");
      }
    };
    window.addEventListener("mousedown", handleClick);
    window.addEventListener("keydown", handleKey);
    onCleanup(() => {
      window.removeEventListener("mousedown", handleClick);
      window.removeEventListener("keydown", handleKey);
    });
  });

  return (
    <div ref={containerRef} class="ref-picker">
      <div
        class="ref-picker-trigger"
        onClick={() => setOpen((p) => !p)}
      >
        <SpecIcon kind={props.kind} size={12} class="flex-shrink-0" />
        <span class={`ref-picker-value ${!props.value ? "placeholder" : ""}`}>
          {props.value ? selectedLabel() : (props.placeholder ?? `Select ${props.kind}...`)}
        </span>
        <span class="ref-picker-chevron">&#9660;</span>
      </div>

      <Show when={open()}>
        <div class="ref-picker-dropdown">
          <input
            class="ref-picker-search"
            type="text"
            value={search()}
            onInput={(e) => setSearch(e.currentTarget.value)}
            placeholder="Search..."
            autofocus
          />
          <div class="ref-picker-options">
            <div
              class={`ref-picker-option ${!props.value ? "active" : ""}`}
              onClick={() => {
                props.onChange("");
                setOpen(false);
                setSearch("");
              }}
            >
              <span class="text-rash-text-muted italic">(none)</span>
            </div>
            <For each={filteredNodes()}>
              {(node) => {
                const refPath = node.path ?? node.label;
                const isActive = () => props.value === refPath || props.value === node.label;
                return (
                  <div
                    class={`ref-picker-option ${isActive() ? "active" : ""}`}
                    onClick={() => {
                      props.onChange(refPath);
                      setOpen(false);
                      setSearch("");
                    }}
                  >
                    <SpecIcon kind={node.kind} size={12} class="flex-shrink-0" />
                    <span>{node.label}</span>
                  </div>
                );
              }}
            </For>
            <Show when={filteredNodes().length === 0}>
              <div class="ref-picker-empty">No matches</div>
            </Show>
          </div>
        </div>
      </Show>

      <style>{`
        .ref-picker {
          position: relative;
        }
        .ref-picker-trigger {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 6px 10px;
          background: var(--rash-bg-secondary);
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
          cursor: pointer;
          transition: border-color 0.15s ease;
          font-size: 13px;
          min-height: 32px;
        }
        .ref-picker-trigger:hover {
          border-color: var(--rash-text-muted);
        }
        .ref-picker-value {
          flex: 1;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          color: var(--rash-text);
          font-family: var(--rash-font);
          font-size: 12px;
        }
        .ref-picker-value.placeholder {
          color: var(--rash-text-muted);
        }
        .ref-picker-chevron {
          font-size: 8px;
          color: var(--rash-text-muted);
        }
        .ref-picker-dropdown {
          position: absolute;
          top: 100%;
          left: 0;
          right: 0;
          margin-top: 4px;
          background: var(--rash-bg-secondary);
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius);
          box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
          z-index: 50;
          overflow: hidden;
        }
        .ref-picker-search {
          width: 100%;
          padding: 8px 10px;
          border: none;
          border-bottom: 1px solid var(--rash-border);
          background: transparent;
          font-size: 12px;
          outline: none;
        }
        .ref-picker-options {
          max-height: 200px;
          overflow-y: auto;
        }
        .ref-picker-option {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 6px 10px;
          font-size: 12px;
          color: var(--rash-text-secondary);
          cursor: pointer;
          transition: background 0.1s ease;
        }
        .ref-picker-option:hover {
          background: var(--rash-surface);
          color: var(--rash-text);
        }
        .ref-picker-option.active {
          background: color-mix(in srgb, var(--rash-accent) 10%, transparent);
          color: var(--rash-accent);
        }
        .ref-picker-empty {
          padding: 12px;
          text-align: center;
          font-size: 12px;
          color: var(--rash-text-muted);
        }
      `}</style>
    </div>
  );
};
