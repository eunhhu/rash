import { Component, Show, For } from "solid-js";
import type { AstNode } from "../../ipc/types";
import { Badge } from "../common/Badge";
import { ExpressionSlot } from "./ExpressionSlot";

interface StatementBlockProps {
  node: AstNode;
  onSelect: (node: AstNode) => void;
  onSelectExpression?: (node: AstNode) => void;
  selected?: boolean;
}

function getStatementLabel(node: AstNode): string {
  switch (node.type) {
    case "LetStatement":
      return `let ${node.mutable ? "mut " : ""}${node.name ?? ""}`;
    case "IfStatement":
      return "if";
    case "ForStatement":
      return `for ${node.variable ?? "item"} in`;
    case "WhileStatement":
      return "while";
    case "ReturnStatement":
      return "return";
    case "TryCatchStatement":
      return "try / catch";
    case "ThrowStatement":
      return "throw";
    case "MatchStatement":
      return "match";
    default:
      return node.type;
  }
}

function getValueSlot(node: AstNode): AstNode | null {
  switch (node.type) {
    case "LetStatement":
      return (node.value as AstNode) ?? null;
    case "ReturnStatement":
      return (node.value as AstNode) ?? null;
    case "ThrowStatement":
      return (node.value as AstNode) ?? null;
    case "IfStatement":
    case "WhileStatement":
      return (node.condition as AstNode) ?? null;
    case "ForStatement":
      return (node.iterable as AstNode) ?? null;
    case "MatchStatement":
      return (node.subject as AstNode) ?? null;
    default:
      return null;
  }
}

function getBodySlots(node: AstNode): { label: string; nodes: AstNode[] }[] {
  const slots: { label: string; nodes: AstNode[] }[] = [];
  if (Array.isArray(node.body)) slots.push({ label: "body", nodes: node.body as AstNode[] });
  if (Array.isArray(node.tryBody)) slots.push({ label: "try", nodes: node.tryBody as AstNode[] });
  if (Array.isArray(node.catchBody)) slots.push({ label: `catch (${node.catchParam ?? "err"})`, nodes: node.catchBody as AstNode[] });
  if (Array.isArray(node.elseBody) && (node.elseBody as AstNode[]).length > 0) {
    slots.push({ label: "else", nodes: node.elseBody as AstNode[] });
  }
  return slots;
}

export const StatementBlock: Component<StatementBlockProps> = (props) => {
  const valueNode = () => getValueSlot(props.node);
  const bodySlots = () => getBodySlots(props.node);

  return (
    <div
      class="stmt-block"
      classList={{ "stmt-block-selected": props.selected }}
      onClick={(e) => {
        e.stopPropagation();
        props.onSelect(props.node);
      }}
    >
      <div class="stmt-block-header">
        <Badge variant="tier" value={props.node.tier} />
        <span class="stmt-block-label">{getStatementLabel(props.node)}</span>
      </div>

      {/* Value / condition slot */}
      <Show when={valueNode() !== undefined}>
        <div class="stmt-block-value">
          <ExpressionSlot
            label={props.node.type === "LetStatement" ? "value" : props.node.type === "ForStatement" ? "iterable" : "condition"}
            node={valueNode()}
            onSelect={props.onSelectExpression}
          />
        </div>
      </Show>

      {/* Body slots */}
      <For each={bodySlots()}>
        {(slot) => (
          <div class="stmt-block-body">
            <span class="stmt-block-body-label">{slot.label}</span>
            <div class="stmt-block-body-content">
              <Show when={slot.nodes.length > 0} fallback={
                <div class="stmt-block-body-empty">empty</div>
              }>
                <For each={slot.nodes}>
                  {(child) => (
                    <StatementBlock
                      node={child}
                      onSelect={props.onSelect}
                      onSelectExpression={props.onSelectExpression}
                    />
                  )}
                </For>
              </Show>
            </div>
          </div>
        )}
      </For>

      <style>{`
        .stmt-block {
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
          background: var(--rash-bg);
          padding: 6px 8px;
          margin: 2px 0;
          cursor: pointer;
          transition: border-color 0.1s ease;
        }
        .stmt-block:hover {
          border-color: var(--rash-surface-hover);
        }
        .stmt-block-selected {
          border-color: var(--rash-accent) !important;
          box-shadow: 0 0 0 1px var(--rash-accent);
        }
        .stmt-block-header {
          display: flex;
          align-items: center;
          gap: 6px;
          margin-bottom: 4px;
        }
        .stmt-block-label {
          font-family: var(--rash-font);
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-text);
        }
        .stmt-block-value {
          margin: 4px 0 4px 16px;
        }
        .stmt-block-body {
          margin: 4px 0 2px 8px;
        }
        .stmt-block-body-label {
          font-size: 10px;
          color: var(--rash-text-muted);
          text-transform: uppercase;
          letter-spacing: 0.3px;
        }
        .stmt-block-body-content {
          margin-left: 8px;
          padding-left: 8px;
          border-left: 2px solid var(--rash-border);
        }
        .stmt-block-body-empty {
          font-size: 11px;
          color: var(--rash-text-muted);
          font-style: italic;
          padding: 4px 0;
        }
      `}</style>
    </div>
  );
};
