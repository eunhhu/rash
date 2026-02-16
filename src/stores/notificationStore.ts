import { createStore, produce } from "solid-js/store";

export interface Toast {
  id: string;
  type: "success" | "error" | "warning" | "info";
  message: string;
  duration: number;
}

function createNotificationStore() {
  const [toasts, setToasts] = createStore<Toast[]>([]);

  function addToast(type: Toast["type"], message: string, duration = 4000): string {
    const id = crypto.randomUUID();
    setToasts(produce((draft) => {
      draft.push({ id, type, message, duration });
    }));
    if (duration > 0) {
      setTimeout(() => removeToast(id), duration);
    }
    return id;
  }

  function removeToast(id: string): void {
    setToasts(produce((draft) => {
      const idx = draft.findIndex((t) => t.id === id);
      if (idx !== -1) draft.splice(idx, 1);
    }));
  }

  const success = (message: string) => addToast("success", message);
  const error = (message: string) => addToast("error", message, 6000);
  const warning = (message: string) => addToast("warning", message);
  const info = (message: string) => addToast("info", message);

  return { toasts, addToast, removeToast, success, error, warning, info };
}

let store: ReturnType<typeof createNotificationStore> | undefined;

export function useNotificationStore() {
  if (!store) {
    store = createNotificationStore();
  }
  return store;
}
