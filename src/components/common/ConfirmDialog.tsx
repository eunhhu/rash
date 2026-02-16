import { Component, Show, onCleanup, createEffect } from "solid-js";
import { useUiStore } from "../../stores/uiStore";

export const ConfirmDialog: Component = () => {
  const { confirmDialog, setConfirmDialog } = useUiStore();

  const handleClose = () => {
    setConfirmDialog(null);
  };

  const handleConfirm = () => {
    const dialog = confirmDialog();
    if (dialog?.onConfirm) {
      dialog.onConfirm();
    }
    setConfirmDialog(null);
  };

  createEffect(() => {
    if (!confirmDialog()) return;

    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleClose();
      }
    };
    window.addEventListener("keydown", handler);
    onCleanup(() => window.removeEventListener("keydown", handler));
  });

  return (
    <Show when={confirmDialog()}>
      {(dialog) => (
        <div
          class="fixed inset-0 z-[1000] flex items-center justify-center bg-black/55"
          onClick={(e) => {
            if (e.target === e.currentTarget) handleClose();
          }}
        >
          <div class="glass-panel p-0 min-w-[360px] max-w-[440px]">
            <div class="px-4 py-3 border-b border-rash-border">
              <h2 class="text-sm font-semibold text-rash-text">
                {dialog().title}
              </h2>
            </div>
            <div class="px-4 py-4">
              <p class="text-xs text-rash-text-secondary leading-relaxed">
                {dialog().message}
              </p>
            </div>
            <div class="flex items-center justify-end gap-2 px-4 py-3 border-t border-rash-border">
              <button class="carbon-btn-secondary text-xs" onClick={handleClose}>
                Cancel
              </button>
              <button
                class={`text-xs ${dialog().danger ? "carbon-btn-danger" : "carbon-btn-primary"}`}
                onClick={handleConfirm}
              >
                {dialog().confirmLabel ?? "Confirm"}
              </button>
            </div>
          </div>
        </div>
      )}
    </Show>
  );
};
