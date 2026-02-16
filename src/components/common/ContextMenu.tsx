import { Component, For, Show, createEffect, onCleanup } from "solid-js";
import { useUiStore } from "../../stores/uiStore";

export const ContextMenu: Component = () => {
  const { contextMenu, setContextMenu } = useUiStore();

  const handleClose = () => {
    setContextMenu(null);
  };

  createEffect(() => {
    if (!contextMenu()) return;

    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    const handleClick = () => handleClose();

    window.addEventListener("keydown", handleKey);
    window.addEventListener("click", handleClick);
    onCleanup(() => {
      window.removeEventListener("keydown", handleKey);
      window.removeEventListener("click", handleClick);
    });
  });

  return (
    <Show when={contextMenu()}>
      {(menu) => (
        <div
          class="fixed z-[1001] glass-panel py-1 min-w-[160px]"
          style={{
            left: `${menu().x}px`,
            top: `${menu().y}px`,
          }}
          role="menu"
          onClick={(e) => e.stopPropagation()}
        >
          <For each={menu().items}>
            {(item) => (
              <Show
                when={!item.separator}
                fallback={<div class="h-px bg-rash-border my-1" />}
              >
                <button
                  role="menuitem"
                  class={`w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left transition-colors
                    ${item.danger ? "text-rash-error hover:bg-rash-error/10" : "text-rash-text-secondary hover:bg-rash-surface hover:text-rash-text"}
                  `}
                  onClick={() => {
                    item.action();
                    handleClose();
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
      )}
    </Show>
  );
};
