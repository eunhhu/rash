import { Component, For } from "solid-js";
import type { Tier } from "../../ipc/types";

interface NodeDef {
  type: string;
  tier: Tier;
  label: string;
}

interface NodePaletteProps {
  onSelect: (type: string, tier: Tier) => void;
}

interface Category {
  name: string;
  nodes: NodeDef[];
}

const CATEGORIES: Category[] = [
  {
    name: "Statements",
    nodes: [
      { type: "LetStatement", tier: 0, label: "let" },
      { type: "IfStatement", tier: 0, label: "if" },
      { type: "ForStatement", tier: 0, label: "for" },
      { type: "WhileStatement", tier: 0, label: "while" },
      { type: "ReturnStatement", tier: 0, label: "return" },
      { type: "TryCatchStatement", tier: 0, label: "try/catch" },
      { type: "ThrowStatement", tier: 0, label: "throw" },
      { type: "MatchStatement", tier: 0, label: "match" },
    ],
  },
  {
    name: "Expressions",
    nodes: [
      { type: "Literal", tier: 0, label: "literal" },
      { type: "Identifier", tier: 0, label: "identifier" },
      { type: "BinaryExpression", tier: 0, label: "binary" },
      { type: "CallExpression", tier: 0, label: "call" },
      { type: "MemberExpression", tier: 0, label: "member" },
      { type: "ObjectExpression", tier: 0, label: "object" },
      { type: "ArrayExpression", tier: 0, label: "array" },
      { type: "TemplateLiteral", tier: 0, label: "template" },
    ],
  },
  {
    name: "Domain (Tier 1)",
    nodes: [
      { type: "DbQuery", tier: 1, label: "dbQuery" },
      { type: "DbMutate", tier: 1, label: "dbMutate" },
      { type: "HttpRespond", tier: 1, label: "httpRespond" },
      { type: "CtxGet", tier: 1, label: "ctxGet" },
      { type: "Validate", tier: 1, label: "validate" },
      { type: "HashPassword", tier: 1, label: "hashPassword" },
      { type: "SignToken", tier: 1, label: "signToken" },
    ],
  },
  {
    name: "Bridge (Tier 3)",
    nodes: [
      { type: "NativeBridge", tier: 3, label: "nativeBridge" },
    ],
  },
];

const TIER_COLORS: Record<number, string> = {
  0: "var(--rash-tier-0)",
  1: "var(--rash-tier-1)",
  2: "var(--rash-tier-2)",
  3: "var(--rash-tier-3)",
};

export const NodePalette: Component<NodePaletteProps> = (props) => {
  return (
    <div class="node-palette">
      <div class="node-palette-title">Blocks</div>
      <For each={CATEGORIES}>
        {(category) => (
          <div class="node-palette-category">
            <div class="node-palette-category-name">{category.name}</div>
            <div class="node-palette-items">
              <For each={category.nodes}>
                {(node) => (
                  <button
                    class="node-palette-item"
                    style={{ "border-left-color": TIER_COLORS[node.tier] }}
                    onClick={() => props.onSelect(node.type, node.tier)}
                  >
                    <span class="node-palette-item-label">{node.label}</span>
                    <span
                      class="node-palette-item-tier"
                      style={{ color: TIER_COLORS[node.tier] }}
                    >
                      T{node.tier}
                    </span>
                  </button>
                )}
              </For>
            </div>
          </div>
        )}
      </For>

      <style>{`
        .node-palette {
          display: flex;
          flex-direction: column;
          gap: 4px;
          padding: 8px;
          overflow-y: auto;
          min-width: 160px;
          max-width: 200px;
          border-right: 1px solid var(--rash-border);
        }
        .node-palette-title {
          font-size: 11px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.5px;
          color: var(--rash-text-muted);
          padding: 4px 0;
        }
        .node-palette-category {
          margin-bottom: 8px;
        }
        .node-palette-category-name {
          font-size: 11px;
          font-weight: 600;
          color: var(--rash-text-secondary);
          padding: 4px 0;
          margin-bottom: 2px;
        }
        .node-palette-items {
          display: flex;
          flex-direction: column;
          gap: 2px;
        }
        .node-palette-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 4px 8px;
          border: none;
          border-left: 3px solid;
          background: var(--rash-bg-secondary);
          color: var(--rash-text);
          font-size: 12px;
          font-family: var(--rash-font);
          cursor: pointer;
          border-radius: 0 var(--rash-radius-sm) var(--rash-radius-sm) 0;
          transition: background 0.1s ease;
        }
        .node-palette-item:hover {
          background: var(--rash-surface);
        }
        .node-palette-item-label {
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .node-palette-item-tier {
          font-size: 9px;
          font-weight: 700;
          flex-shrink: 0;
        }
      `}</style>
    </div>
  );
};
