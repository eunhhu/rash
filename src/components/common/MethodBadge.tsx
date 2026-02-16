import { Component, Show } from "solid-js";
import type { HttpMethod } from "../../ipc/types";

const METHOD_COLORS: Record<string, { bg: string; text: string }> = {
  GET: { bg: "color-mix(in srgb, var(--rash-method-get) 20%, transparent)", text: "var(--rash-method-get)" },
  POST: { bg: "color-mix(in srgb, var(--rash-method-post) 20%, transparent)", text: "var(--rash-method-post)" },
  PUT: { bg: "color-mix(in srgb, var(--rash-method-put) 20%, transparent)", text: "var(--rash-method-put)" },
  PATCH: { bg: "color-mix(in srgb, var(--rash-method-patch) 20%, transparent)", text: "var(--rash-method-patch)" },
  DELETE: { bg: "color-mix(in srgb, var(--rash-method-delete) 20%, transparent)", text: "var(--rash-method-delete)" },
  HEAD: { bg: "color-mix(in srgb, var(--rash-text-muted) 20%, transparent)", text: "var(--rash-text-muted)" },
  OPTIONS: { bg: "color-mix(in srgb, var(--rash-text-muted) 20%, transparent)", text: "var(--rash-text-muted)" },
};

interface MethodBadgeProps {
  method: HttpMethod;
  active?: boolean;
  removable?: boolean;
  onClick?: () => void;
  onRemove?: () => void;
}

export const MethodBadge: Component<MethodBadgeProps> = (props) => {
  const colors = () => METHOD_COLORS[props.method] ?? METHOD_COLORS.GET;

  return (
    <span
      class="method-badge"
      classList={{ active: props.active }}
      style={{
        background: colors().bg,
        color: colors().text,
        border: props.active ? `1px solid ${colors().text}` : "1px solid transparent",
      }}
      onClick={(e) => {
        e.stopPropagation();
        props.onClick?.();
      }}
    >
      <span class="method-badge-label">{props.method}</span>
      <Show when={props.removable}>
        <span
          class="method-badge-remove"
          onClick={(e) => {
            e.stopPropagation();
            props.onRemove?.();
          }}
        >
          ×
        </span>
      </Show>

      <style>{`
        .method-badge {
          display: inline-flex;
          align-items: center;
          gap: 4px;
          padding: 3px 8px;
          border-radius: 4px;
          font-size: 11px;
          font-weight: 700;
          cursor: pointer;
          transition: all 0.15s ease;
          user-select: none;
        }
        .method-badge:hover {
          filter: brightness(1.1);
        }
        .method-badge-label {
          line-height: 1;
        }
        .method-badge-remove {
          font-size: 14px;
          line-height: 1;
          opacity: 0.6;
          cursor: pointer;
        }
        .method-badge-remove:hover {
          opacity: 1;
        }
      `}</style>
    </span>
  );
};
