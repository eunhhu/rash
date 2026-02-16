import { Component, createSignal, createEffect, onCleanup } from "solid-js";
import { invoke } from "../../ipc/invoke";
import type { Language, Framework } from "../../ipc/types";
import { CodeViewer } from "../common/CodeViewer";
import { LanguageSelector } from "./LanguageSelector";

interface CodePreviewProps {
  /** The spec data to preview (any spec type). */
  specData: unknown;
  /** The spec file path for context. */
  specPath: string;
}

export const CodePreview: Component<CodePreviewProps> = (props) => {
  const [language, setLanguage] = createSignal<Language>("typescript");
  const [framework, setFramework] = createSignal<Framework>("express");
  const [code, setCode] = createSignal("// Loading preview...");
  const [loading, setLoading] = createSignal(false);

  let timer: ReturnType<typeof setTimeout> | undefined;

  createEffect(() => {
    // Track reactive dependencies
    const _data = props.specData;
    const _lang = language();
    const _fw = framework();

    clearTimeout(timer);
    timer = setTimeout(async () => {
      setLoading(true);
      try {
        const result = await invoke<string>("preview_code", {
          path: props.specPath,
          language: _lang,
          framework: _fw,
        });
        setCode(result);
      } catch (err) {
        setCode(`// Preview error: ${err instanceof Error ? err.message : String(err)}`);
      } finally {
        setLoading(false);
      }
    }, 300);
  });

  onCleanup(() => clearTimeout(timer));

  return (
    <div class="code-preview">
      <div class="code-preview-header">
        <LanguageSelector
          language={language()}
          framework={framework()}
          onLanguageChange={setLanguage}
          onFrameworkChange={setFramework}
        />
        {loading() && <span class="code-preview-loading">Generating...</span>}
      </div>
      <div class="code-preview-body">
        <CodeViewer code={code()} language={language()} />
      </div>

      <style>{`
        .code-preview {
          display: flex;
          flex-direction: column;
          height: 100%;
          overflow: hidden;
        }
        .code-preview-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 8px 12px;
          border-bottom: 1px solid var(--rash-border);
          background: var(--rash-bg-secondary);
          flex-shrink: 0;
        }
        .code-preview-loading {
          font-size: 11px;
          color: var(--rash-text-muted);
          font-style: italic;
        }
        .code-preview-body {
          flex: 1;
          display: flex;
          overflow: hidden;
        }
      `}</style>
    </div>
  );
};
