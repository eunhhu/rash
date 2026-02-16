import { Component, For, JSX, Show, createSignal, onCleanup, createEffect } from "solid-js";

export interface DropdownItem {
  type?: "item" | "separator";
  label?: string;
  icon?: JSX.Element;
  action?: () => void;
  danger?: boolean;
}

interface DropdownMenuProps {
  trigger: JSX.Element;
  items: DropdownItem[];
}

export const DropdownMenu: Component<DropdownMenuProps> = (props) => {
  const [open, setOpen] = createSignal(false);
  let containerRef: HTMLDivElement | undefined;

  createEffect(() => {
    if (!open()) return;

    const handleClick = (e: MouseEvent) => {
      if (containerRef && !containerRef.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };

    window.addEventListener("mousedown", handleClick);
    window.addEventListener("keydown", handleKey);
    onCleanup(() => {
      window.removeEventListener("mousedown", handleClick);
      window.removeEventListener("keydown", handleKey);
    });
  });

  return (
    <div ref={containerRef} class="relative">
      <div onClick={() => setOpen((prev) => !prev)}>
        {props.trigger}
      </div>
      <Show when={open()}>
        <div class="absolute top-full left-0 mt-1 z-50 glass-panel py-1 min-w-[180px]">
          <For each={props.items}>
            {(item) => (
              <Show
                when={item.type !== "separator"}
                fallback={<div class="h-px bg-rash-border my-1" />}
              >
                <button
                  class={`w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left transition-colors
                    ${item.danger ? "text-rash-error hover:bg-rash-error/10" : "text-rash-text-secondary hover:bg-rash-surface hover:text-rash-text"}
                  `}
                  onClick={() => {
                    item.action?.();
                    setOpen(false);
                  }}
                >
                  <Show when={item.icon}>
                    <span class="flex-shrink-0">{item.icon}</span>
                  </Show>
                  <span>{item.label}</span>
                </button>
              </Show>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};
