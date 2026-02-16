import { Component, Show } from "solid-js";
import type { AstNode } from "../../ipc/types";
import { Badge } from "../common/Badge";

interface ExpressionSlotProps {
  label?: string;
  node: AstNode | null;
  onSelect?: (node: AstNode) => void;
  onClear?: () => void;
}

function expressionSummary(node: AstNode): string {
  switch (node.type) {
    case "Literal":
      return String(node.value ?? '""');
    case "Identifier":
      return String(node.name ?? "?");
    case "BinaryExpression":
      return String(node.operator ?? "op");
    case "CallExpression": {
      const callee = node.callee as AstNode | null;
      return callee ? `${callee.name ?? callee.type}()` : "call()";
    }
    case "MemberExpression":
      return `.${String(node.property ?? "?")}`;
    case "ObjectExpression":
      return `{...}`;
    case "ArrayExpression":
      return `[...]`;
    case "TemplateLiteral":
      return "`...`";
    default:
      return node.type;
  }
}

export const ExpressionSlot: Component<ExpressionSlotProps> = (props) => {
  return (
    <div class="expr-slot">
      <Show when={props.label}>
        <span class="expr-slot-label">{props.label}</span>
      </Show>
      <Show
        when={props.node}
        fallback={
          <div class="expr-slot-empty" onClick={() => props.onSelect?.({} as AstNode)}>
            <span>+ expression</span>
          </div>
        }
      >
        <div class="expr-slot-filled" onClick={() => props.onSelect?.(props.node!)}>
          <Badge variant="tier" value={props.node!.tier} />
          <span class="expr-slot-summary">{expressionSummary(props.node!)}</span>
          <Show when={props.onClear}>
            <button
              class="btn-icon expr-slot-clear"
              onClick={(e) => {
                e.stopPropagation();
                props.onClear!();
              }}
            >
              {"\u00D7"}
            </button>
          </Show>
        </div>
      </Show>

      <style>{`
        .expr-slot {
          display: flex;
          flex-direction: column;
          gap: 2px;
        }
        .expr-slot-label {
          font-size: 10px;
          color: var(--rash-text-muted);
          text-transform: uppercase;
          letter-spacing: 0.3px;
        }
        .expr-slot-empty {
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 6px 10px;
          border: 1px dashed var(--rash-border);
          border-radius: var(--rash-radius-sm);
          color: var(--rash-text-muted);
          font-size: 11px;
          cursor: pointer;
          transition: border-color 0.1s ease, color 0.1s ease;
        }
        .expr-slot-empty:hover {
          border-color: var(--rash-accent);
          color: var(--rash-accent);
        }
        .expr-slot-filled {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 4px 8px;
          background: var(--rash-bg-secondary);
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
          cursor: pointer;
          transition: border-color 0.1s ease;
        }
        .expr-slot-filled:hover {
          border-color: var(--rash-accent);
        }
        .expr-slot-summary {
          font-family: var(--rash-font);
          font-size: 12px;
          color: var(--rash-text);
          flex: 1;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .expr-slot-clear {
          opacity: 0;
          flex-shrink: 0;
        }
        .expr-slot-filled:hover .expr-slot-clear {
          opacity: 1;
        }
      `}</style>
    </div>
  );
};
