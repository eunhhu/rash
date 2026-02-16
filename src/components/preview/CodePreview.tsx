import { Component, For, createSignal, createEffect, onCleanup } from "solid-js";
import { previewCode } from "../../ipc/commands";
import type { Language, Framework } from "../../ipc/types";
import { CodeViewer } from "../common/CodeViewer";
import { LanguageSelector } from "./LanguageSelector";

interface CodePreviewProps {
  /** Reactive trigger — changing this value re-fetches the preview. */
  specData?: unknown;
}

export const CodePreview: Component<CodePreviewProps> = (props) => {
  const [language, setLanguage] = createSignal<Language>("typescript");
  const [framework, setFramework] = createSignal<Framework>("express");
  const [code, setCode] = createSignal("// Loading preview...");
  const [loading, setLoading] = createSignal(false);

  let timer: ReturnType<typeof setTimeout> | undefined;

  const [selectedFile, setSelectedFile] = createSignal<string>("");
  const [files, setFiles] = createSignal<Record<string, string>>({});

  createEffect(() => {
    // Track reactive dependencies
    const _data = props.specData;
    const _lang = language();
    const _fw = framework();

    clearTimeout(timer);
    timer = setTimeout(async () => {
      setLoading(true);
      try {
        const result = await previewCode({ language: _lang, framework: _fw });
        setFiles(result);
        const fileNames = Object.keys(result);
        if (fileNames.length > 0) {
          const currentFile = selectedFile();
          const targetFile = result[currentFile] ? currentFile : fileNames[0];
          setSelectedFile(targetFile);
          setCode(result[targetFile] ?? "// No preview available");
        } else {
          setSelectedFile("");
          setCode("// No preview available");
        }
      } catch (err) {
        setFiles({});
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
        <select
          class="code-preview-file-select"
          value={selectedFile()}
          onChange={(e) => {
            const f = e.currentTarget.value;
            setSelectedFile(f);
            setCode(files()[f] ?? "");
          }}
        >
          <For each={Object.keys(files())}>
            {(f) => <option value={f}>{f}</option>}
          </For>
        </select>
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
        .code-preview-file-select {
          font-size: 11px;
          padding: 2px 6px;
          max-width: 200px;
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
