import { Component, Show, createSignal, createEffect } from "solid-js";
import { previewCode, writeHandler } from "../../ipc/commands";
import type { HandlerSpec, AstNode, Tier, Language } from "../../ipc/types";
import { createNode } from "../../utils/ast";
import { NodePalette } from "./NodePalette";
import { AstNodeView } from "./AstNodeView";
import { CodeViewer } from "../common/CodeViewer";
import { insertNode, removeNode } from "../../utils/ast";

interface HandlerEditorProps {
  handler: HandlerSpec;
  filePath: string;
  onDirty?: (dirty: boolean) => void;
}

export const HandlerEditor: Component<HandlerEditorProps> = (props) => {
  const [draft, setDraft] = createSignal<HandlerSpec>(structuredClone(props.handler));
  const [selectedNodeId, setSelectedNodeId] = createSignal<string | null>(null);
  const [dirty, setDirty] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [previewSrc, setPreviewSrc] = createSignal("");
  const [previewLang, setPreviewLang] = createSignal<Language>("typescript");

  createEffect(() => {
    setDraft(structuredClone(props.handler));
    setDirty(false);
    setSelectedNodeId(null);
  });

  // Debounced preview — calls project-wide codegen and picks a relevant file
  let previewTimer: ReturnType<typeof setTimeout> | undefined;
  createEffect(() => {
    // Access body reactively
    const _body = draft().body;
    const lang = previewLang();
    clearTimeout(previewTimer);
    previewTimer = setTimeout(async () => {
      try {
        const fileMap = await previewCode({ language: lang, framework: "express" });
        // Find a file matching this handler's name, or show first file
        const handlerName = draft().name.toLowerCase();
        const match = Object.entries(fileMap).find(([k]) =>
          k.toLowerCase().includes(handlerName)
        );
        setPreviewSrc(match ? match[1] : Object.values(fileMap)[0] ?? "// No output");
      } catch {
        setPreviewSrc("// Preview unavailable");
      }
    }, 300);
  });

  const markDirty = () => {
    setDirty(true);
    props.onDirty?.(true);
  };

  const handleAddNode = (type: string, tier: Tier) => {
    const node = createNode(type, tier);
    setDraft((prev) => ({
      ...prev,
      body: insertNode(prev.body, null, node),
    }));
    setSelectedNodeId((node as Record<string, unknown>).id as string);
    markDirty();
  };

  const handleSelectNode = (node: AstNode) => {
    const id = (node as Record<string, unknown>).id as string | undefined;
    setSelectedNodeId(id ?? null);
  };

  const handleDeleteSelected = () => {
    const id = selectedNodeId();
    if (!id) return;
    setDraft((prev) => ({
      ...prev,
      body: removeNode(prev.body, id),
    }));
    setSelectedNodeId(null);
    markDirty();
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await writeHandler(props.filePath, draft());
      setDirty(false);
      props.onDirty?.(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="handler-editor">
      <div class="handler-editor-header">
        <div class="handler-editor-info">
          <span class="handler-editor-name">{draft().name}</span>
          <Show when={draft().async}>
            <span class="handler-editor-async">async</span>
          </Show>
        </div>
        <div class="handler-editor-actions">
          <Show when={selectedNodeId()}>
            <button class="btn btn-sm" onClick={handleDeleteSelected}>
              Delete Block
            </button>
          </Show>
          <button
            class="btn btn-primary btn-sm"
            disabled={!dirty() || saving()}
            onClick={handleSave}
          >
            {saving() ? "Saving..." : "Save"}
          </button>
        </div>
      </div>

      <div class="handler-editor-body">
        {/* Left: palette */}
        <NodePalette onSelect={handleAddNode} />

        {/* Center: block tree */}
        <div class="handler-editor-canvas">
          <AstNodeView
            nodes={draft().body}
            selectedNodeId={selectedNodeId()}
            onSelect={handleSelectNode}
          />
        </div>

        {/* Right: code preview */}
        <div class="handler-editor-preview">
          <div class="handler-editor-preview-header">
            <select
              class="handler-editor-lang-select"
              value={previewLang()}
              onChange={(e) => setPreviewLang(e.currentTarget.value as Language)}
            >
              <option value="typescript">TypeScript</option>
              <option value="python">Python</option>
              <option value="rust">Rust</option>
              <option value="go">Go</option>
            </select>
          </div>
          <CodeViewer code={previewSrc()} language={previewLang()} />
        </div>
      </div>

      <style>{`
        .handler-editor {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .handler-editor-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 12px 16px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
          flex-shrink: 0;
        }
        .handler-editor-info {
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .handler-editor-name {
          font-size: 14px;
          font-weight: 600;
          color: var(--rash-text);
          font-family: var(--rash-font);
        }
        .handler-editor-async {
          font-size: 11px;
          padding: 1px 6px;
          border-radius: var(--rash-radius-sm);
          background: var(--rash-surface);
          color: var(--rash-accent);
        }
        .handler-editor-actions {
          display: flex;
          gap: 8px;
          align-items: center;
        }
        .handler-editor-body {
          display: flex;
          flex: 1;
          overflow: hidden;
        }
        .handler-editor-canvas {
          flex: 1;
          display: flex;
          flex-direction: column;
          overflow: hidden;
          border-right: 1px solid var(--rash-border);
        }
        .handler-editor-preview {
          display: flex;
          flex-direction: column;
          width: 320px;
          min-width: 240px;
          overflow: hidden;
        }
        .handler-editor-preview-header {
          display: flex;
          align-items: center;
          padding: 6px 8px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
        }
        .handler-editor-lang-select {
          font-size: 11px;
          padding: 2px 6px;
        }
      `}</style>
    </div>
  );
};
