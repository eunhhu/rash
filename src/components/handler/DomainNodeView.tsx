import { Component, Show } from "solid-js";
import type { AstNode } from "../../ipc/types";
import { Badge } from "../common/Badge";

interface DomainNodeViewProps {
  node: AstNode;
  onSelect: (node: AstNode) => void;
  selected?: boolean;
}

function renderDbQuery(node: AstNode) {
  return (
    <div class="domain-node-fields">
      <div class="domain-node-field">
        <span class="domain-node-field-label">model</span>
        <span class="domain-node-field-value">{String(node.model ?? "")}</span>
      </div>
      <div class="domain-node-field">
        <span class="domain-node-field-label">operation</span>
        <span class="domain-node-field-value">{String(node.operation ?? "")}</span>
      </div>
      <Show when={node.where}>
        <div class="domain-node-field">
          <span class="domain-node-field-label">where</span>
          <span class="domain-node-field-value">{(node.where as AstNode)?.type ?? "..."}</span>
        </div>
      </Show>
    </div>
  );
}

function renderDbMutate(node: AstNode) {
  return (
    <div class="domain-node-fields">
      <div class="domain-node-field">
        <span class="domain-node-field-label">model</span>
        <span class="domain-node-field-value">{String(node.model ?? "")}</span>
      </div>
      <div class="domain-node-field">
        <span class="domain-node-field-label">operation</span>
        <span class="domain-node-field-value">{String(node.operation ?? "")}</span>
      </div>
    </div>
  );
}

function renderHttpRespond(node: AstNode) {
  return (
    <div class="domain-node-fields">
      <div class="domain-node-field">
        <span class="domain-node-field-label">status</span>
        <span class="domain-node-field-value">{String(node.status ?? 200)}</span>
      </div>
      <Show when={node.body}>
        <div class="domain-node-field">
          <span class="domain-node-field-label">body</span>
          <span class="domain-node-field-value">{(node.body as AstNode)?.type ?? "..."}</span>
        </div>
      </Show>
    </div>
  );
}

function renderCtxGet(node: AstNode) {
  return (
    <div class="domain-node-fields">
      <div class="domain-node-field">
        <span class="domain-node-field-label">key</span>
        <span class="domain-node-field-value">{String(node.key ?? "")}</span>
      </div>
    </div>
  );
}

function renderValidate(node: AstNode) {
  return (
    <div class="domain-node-fields">
      <div class="domain-node-field">
        <span class="domain-node-field-label">schema</span>
        <span class="domain-node-field-value">{String(node.schema ?? "")}</span>
      </div>
    </div>
  );
}

function renderGenericDomain(node: AstNode) {
  return (
    <div class="domain-node-fields">
      <span class="domain-node-field-value">{node.type}</span>
    </div>
  );
}

const DOMAIN_RENDERERS: Record<string, (node: AstNode) => any> = {
  DbQuery: renderDbQuery,
  DbMutate: renderDbMutate,
  HttpRespond: renderHttpRespond,
  CtxGet: renderCtxGet,
  Validate: renderValidate,
  HashPassword: renderGenericDomain,
  SignToken: renderGenericDomain,
  NativeBridge: renderGenericDomain,
};

export const DomainNodeView: Component<DomainNodeViewProps> = (props) => {
  const render = () => {
    const fn = DOMAIN_RENDERERS[props.node.type];
    return fn ? fn(props.node) : renderGenericDomain(props.node);
  };

  return (
    <div
      class="domain-node"
      classList={{ "domain-node-selected": props.selected }}
      onClick={(e) => {
        e.stopPropagation();
        props.onSelect(props.node);
      }}
    >
      <div class="domain-node-header">
        <Badge variant="tier" value={props.node.tier} />
        <span class="domain-node-type">{props.node.type}</span>
      </div>
      {render()}

      <style>{`
        .domain-node {
          border: 1px solid var(--rash-border);
          border-left: 3px solid var(--rash-tier-1);
          border-radius: var(--rash-radius-sm);
          background: var(--rash-bg);
          padding: 6px 8px;
          margin: 2px 0;
          cursor: pointer;
          transition: border-color 0.1s ease;
        }
        .domain-node:hover {
          border-color: var(--rash-surface-hover);
          border-left-color: var(--rash-tier-1);
        }
        .domain-node-selected {
          border-color: var(--rash-accent) !important;
          border-left-color: var(--rash-tier-1) !important;
          box-shadow: 0 0 0 1px var(--rash-accent);
        }
        .domain-node-header {
          display: flex;
          align-items: center;
          gap: 6px;
          margin-bottom: 4px;
        }
        .domain-node-type {
          font-family: var(--rash-font);
          font-size: 12px;
          font-weight: 600;
          color: var(--rash-tier-1);
        }
        .domain-node-fields {
          display: flex;
          flex-direction: column;
          gap: 2px;
          padding-left: 8px;
        }
        .domain-node-field {
          display: flex;
          gap: 6px;
          font-size: 11px;
        }
        .domain-node-field-label {
          color: var(--rash-text-muted);
          min-width: 60px;
        }
        .domain-node-field-value {
          color: var(--rash-text);
          font-family: var(--rash-font);
        }
      `}</style>
    </div>
  );
};
