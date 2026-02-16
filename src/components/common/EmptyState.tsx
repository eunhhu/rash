import { Component, JSX, Show } from "solid-js";

interface EmptyStateProps {
  icon?: JSX.Element;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
}

export const EmptyState: Component<EmptyStateProps> = (props) => {
  return (
    <div class="flex flex-col items-center justify-center flex-1 gap-3 p-8 text-center">
      <Show when={props.icon}>
        <div class="text-rash-text-muted">{props.icon}</div>
      </Show>
      <h3 class="text-sm font-medium text-rash-text-secondary">{props.title}</h3>
      <Show when={props.description}>
        <p class="text-xs text-rash-text-muted max-w-[280px]">{props.description}</p>
      </Show>
      <Show when={props.action}>
        {(action) => (
          <button
            class="carbon-btn-primary mt-2 text-xs"
            onClick={action().onClick}
          >
            {action().label}
          </button>
        )}
      </Show>
    </div>
  );
};
