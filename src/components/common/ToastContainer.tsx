import { Component, For, Switch, Match } from "solid-js";
import { FiCheck, FiAlertCircle, FiAlertTriangle, FiInfo, FiX } from "solid-icons/fi";
import { useNotificationStore, type Toast } from "../../stores/notificationStore";

const TOAST_STYLES: Record<Toast["type"], { icon: typeof FiCheck; color: string }> = {
  success: { icon: FiCheck, color: "text-rash-success" },
  error: { icon: FiAlertCircle, color: "text-rash-error" },
  warning: { icon: FiAlertTriangle, color: "text-rash-warning" },
  info: { icon: FiInfo, color: "text-rash-info" },
};

const ToastItem: Component<{ toast: Toast }> = (props) => {
  const { removeToast } = useNotificationStore();
  const style = () => TOAST_STYLES[props.toast.type];

  return (
    <div class="glass-surface flex items-center gap-3 px-4 py-3 min-w-[280px] max-w-[400px] animate-slide-in">
      <Switch>
        <Match when={props.toast.type === "success"}>
          <FiCheck size={16} class="text-rash-success flex-shrink-0" />
        </Match>
        <Match when={props.toast.type === "error"}>
          <FiAlertCircle size={16} class="text-rash-error flex-shrink-0" />
        </Match>
        <Match when={props.toast.type === "warning"}>
          <FiAlertTriangle size={16} class="text-rash-warning flex-shrink-0" />
        </Match>
        <Match when={props.toast.type === "info"}>
          <FiInfo size={16} class="text-rash-info flex-shrink-0" />
        </Match>
      </Switch>
      <span class="flex-1 text-xs text-rash-text">{props.toast.message}</span>
      <button
        class="carbon-btn-icon p-0.5 flex-shrink-0"
        onClick={() => removeToast(props.toast.id)}
      >
        <FiX size={12} />
      </button>
    </div>
  );
};

export const ToastContainer: Component = () => {
  const { toasts } = useNotificationStore();

  return (
    <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      <For each={toasts}>
        {(toast) => <ToastItem toast={toast} />}
      </For>
    </div>
  );
};
