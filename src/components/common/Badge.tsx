import { Component } from "solid-js";

interface BadgeProps {
  variant: "method" | "tier";
  value: string | number;
}

const METHOD_COLORS: Record<string, string> = {
  GET: "var(--rash-method-get)",
  POST: "var(--rash-method-post)",
  PUT: "var(--rash-method-put)",
  PATCH: "var(--rash-method-patch)",
  DELETE: "var(--rash-method-delete)",
  HEAD: "var(--rash-text-muted)",
  OPTIONS: "var(--rash-text-muted)",
};

const TIER_COLORS: Record<number, string> = {
  0: "var(--rash-tier-0)",
  1: "var(--rash-tier-1)",
  2: "var(--rash-tier-2)",
  3: "var(--rash-tier-3)",
};

export const Badge: Component<BadgeProps> = (props) => {
  const color = () => {
    if (props.variant === "method") {
      return METHOD_COLORS[String(props.value).toUpperCase()] ?? "var(--rash-text-muted)";
    }
    return TIER_COLORS[Number(props.value)] ?? "var(--rash-text-muted)";
  };

  const label = () => {
    if (props.variant === "tier") {
      return `T${props.value}`;
    }
    return String(props.value).toUpperCase();
  };

  return (
    <span
      class="badge"
      style={{
        color: color(),
        "border-color": color(),
      }}
    >
      {label()}

      <style>{`
        .badge {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          padding: 1px 6px;
          font-size: 10px;
          font-weight: 700;
          font-family: var(--rash-font);
          letter-spacing: 0.5px;
          border: 1px solid;
          border-radius: var(--rash-radius-sm);
          line-height: 1.4;
          white-space: nowrap;
        }
      `}</style>
    </span>
  );
};
