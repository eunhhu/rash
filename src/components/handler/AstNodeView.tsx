import { Component, For, Show, createSignal } from "solid-js";
import type { AstNode } from "../../ipc/types";
import { Badge } from "../common/Badge";
import { StatementBlock } from "./StatementBlock";
import { DomainNodeView } from "./DomainNodeView";
import { ExpressionSlot } from "./ExpressionSlot";

interface AstNodeViewProps {
  nodes: AstNode[];
  selectedNodeId: string | null;
  onSelect: (node: AstNode) => void;
}

const DOMAIN_TYPES = new Set([
  "DbQuery", "DbMutate", "HttpRespond", "CtxGet",
  "Validate", "HashPassword", "SignToken", "NativeBridge",
]);

const STATEMENT_TYPES = new Set([
  "LetStatement", "IfStatement", "ForStatement", "WhileStatement",
  "ReturnStatement", "TryCatchStatement", "ThrowStatement", "MatchStatement",
]);

const NodeItem: Component<{
  node: AstNode;
  selectedNodeId: string | null;
  onSelect: (node: AstNode) => void;
}> = (props) => {
  const nodeId = () => (props.node as Record<string, unknown>).id as string | undefined;
  const isSelected = () => nodeId() != null && nodeId() === props.selectedNodeId;

  // Domain nodes get special rendering
  if (DOMAIN_TYPES.has(props.node.type)) {
    return (
      <DomainNodeView
        node={props.node}
        onSelect={props.onSelect}
        selected={isSelected()}
      />
    );
  }

  // Statement nodes get block rendering
  if (STATEMENT_TYPES.has(props.node.type)) {
    return (
      <StatementBlock
        node={props.node}
        onSelect={props.onSelect}
        onSelectExpression={props.onSelect}
        selected={isSelected()}
      />
    );
  }

  // Expression nodes (rare at top level, but handle anyway)
  return (
    <div
      class="ast-node-generic"
      classList={{ "ast-node-generic-selected": isSelected() }}
      onClick={(e) => {
        e.stopPropagation();
        props.onSelect(props.node);
      }}
    >
      <Badge variant="tier" value={props.node.tier} />
      <span class="ast-node-generic-type">{props.node.type}</span>
    </div>
  );
};

export const AstNodeView: Component<AstNodeViewProps> = (props) => {
  return (
    <div class="ast-node-view">
      <Show when={props.nodes.length > 0} fallback={
        <div class="ast-node-view-empty">
          No statements yet. Add blocks from the palette.
        </div>
      }>
        <For each={props.nodes}>
          {(node) => (
            <NodeItem
              node={node}
              selectedNodeId={props.selectedNodeId}
              onSelect={props.onSelect}
            />
          )}
        </For>
      </Show>

      <style>{`
        .ast-node-view {
          display: flex;
          flex-direction: column;
          gap: 2px;
          padding: 8px;
          overflow-y: auto;
          flex: 1;
        }
        .ast-node-view-empty {
          display: flex;
          align-items: center;
          justify-content: center;
          height: 100%;
          color: var(--rash-text-muted);
          font-size: 13px;
        }
        .ast-node-generic {
          display: flex;
          align-items: center;
          gap: 6px;
          padding: 6px 8px;
          border: 1px solid var(--rash-border);
          border-radius: var(--rash-radius-sm);
          cursor: pointer;
          transition: border-color 0.1s ease;
        }
        .ast-node-generic:hover {
          border-color: var(--rash-surface-hover);
        }
        .ast-node-generic-selected {
          border-color: var(--rash-accent) !important;
          box-shadow: 0 0 0 1px var(--rash-accent);
        }
        .ast-node-generic-type {
          font-family: var(--rash-font);
          font-size: 12px;
          color: var(--rash-text);
        }
      `}</style>
    </div>
  );
};
