import { Component, For, Show, createSignal } from "solid-js";

interface JsonViewerProps {
  data: unknown;
}

const JsonNode: Component<{ label?: string; value: unknown; depth: number }> = (props) => {
  const [expanded, setExpanded] = createSignal(props.depth < 2);

  const isObject = () => props.value !== null && typeof props.value === "object" && !Array.isArray(props.value);
  const isArray = () => Array.isArray(props.value);
  const isExpandable = () => isObject() || isArray();

  const entries = () => {
    if (isArray()) {
      return (props.value as unknown[]).map((v, i) => [String(i), v] as [string, unknown]);
    }
    if (isObject()) {
      return Object.entries(props.value as Record<string, unknown>);
    }
    return [];
  };

  const renderPrimitive = () => {
    const v = props.value;
    if (v === null) return <span class="json-null">null</span>;
    if (v === undefined) return <span class="json-null">undefined</span>;
    if (typeof v === "string") return <span class="json-string">"{v}"</span>;
    if (typeof v === "number") return <span class="json-number">{String(v)}</span>;
    if (typeof v === "boolean") return <span class="json-boolean">{String(v)}</span>;
    return <span>{String(v)}</span>;
  };

  const bracket = () => (isArray() ? ["[", "]"] : ["{", "}"]);

  return (
    <div class="json-node" style={{ "padding-left": props.depth > 0 ? "16px" : "0" }}>
      <Show
        when={isExpandable()}
        fallback={
          <div class="json-leaf">
            <Show when={props.label}>
              <span class="json-key">{props.label}: </span>
            </Show>
            {renderPrimitive()}
          </div>
        }
      >
        <div class="json-expandable" onClick={() => setExpanded((p) => !p)}>
          <span class="json-chevron" classList={{ "json-chevron-open": expanded() }}>
            {"\u25B6"}
          </span>
          <Show when={props.label}>
            <span class="json-key">{props.label}: </span>
          </Show>
          <span class="json-bracket">{bracket()[0]}</span>
          <Show when={!expanded()}>
            <span class="json-collapsed"> ... {entries().length} items </span>
            <span class="json-bracket">{bracket()[1]}</span>
          </Show>
        </div>
        <Show when={expanded()}>
          <For each={entries()}>
            {([key, val]) => <JsonNode label={key} value={val} depth={props.depth + 1} />}
          </For>
          <div style={{ "padding-left": "16px" }}>
            <span class="json-bracket">{bracket()[1]}</span>
          </div>
        </Show>
      </Show>
    </div>
  );
};

export const JsonViewer: Component<JsonViewerProps> = (props) => {
  return (
    <div class="json-viewer">
      <JsonNode value={props.data} depth={0} />

      <style>{`
        .json-viewer {
          font-family: var(--rash-font);
          font-size: 12px;
          padding: 8px;
          overflow: auto;
          color: var(--rash-text);
        }
        .json-leaf {
          padding: 1px 0;
        }
        .json-expandable {
          cursor: pointer;
          padding: 1px 0;
          display: flex;
          align-items: center;
          gap: 4px;
        }
        .json-expandable:hover {
          background: var(--rash-surface);
          border-radius: 2px;
        }
        .json-chevron {
          font-size: 8px;
          width: 12px;
          text-align: center;
          color: var(--rash-text-muted);
          transition: transform 0.15s ease;
          flex-shrink: 0;
        }
        .json-chevron-open {
          transform: rotate(90deg);
        }
        .json-key {
          color: var(--rash-accent);
        }
        .json-string {
          color: var(--rash-success);
        }
        .json-number {
          color: var(--rash-warning);
        }
        .json-boolean {
          color: var(--rash-info);
        }
        .json-null {
          color: var(--rash-text-muted);
          font-style: italic;
        }
        .json-bracket {
          color: var(--rash-text-muted);
        }
        .json-collapsed {
          color: var(--rash-text-muted);
          font-style: italic;
        }
      `}</style>
    </div>
  );
};
