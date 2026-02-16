import { Component, Show, createSignal, For } from "solid-js";
import { Modal } from "../common/Modal";
import { Button } from "../common/Button";
import { importOpenapi, importFromCode } from "../../ipc/commands";
import type { ImportResult } from "../../ipc/types";

type ImportMode = "openapi" | "code";

interface ImportDialogProps {
  onClose: () => void;
  onImported: () => void;
}

export const ImportDialog: Component<ImportDialogProps> = (props) => {
  const [mode, setMode] = createSignal<ImportMode>("openapi");
  const [filePath, setFilePath] = createSignal("");
  const [targetDir, setTargetDir] = createSignal("");
  const [importing, setImporting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [result, setResult] = createSignal<ImportResult | null>(null);

  const handleSelectFile = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: false,
        filters: [{ name: "OpenAPI", extensions: ["json", "yaml", "yml"] }],
      });
      if (selected && typeof selected === "string") {
        setFilePath(selected);
      }
    } catch {
      // Dialog unavailable or user cancelled
    }
  };

  const handleSelectDir = async (setter: (v: string) => void) => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        setter(selected);
      }
    } catch {
      // Dialog unavailable or user cancelled
    }
  };

  const canImport = () => {
    if (mode() === "openapi") {
      return filePath().trim().length > 0 && targetDir().trim().length > 0;
    }
    return filePath().trim().length > 0 && targetDir().trim().length > 0;
  };

  const handleImport = async () => {
    if (!canImport() || importing()) return;
    setImporting(true);
    setError(null);
    setResult(null);

    try {
      if (mode() === "openapi") {
        const { readTextFile } = await import("@tauri-apps/plugin-fs");
        const content = await readTextFile(filePath());
        const res = await importOpenapi(content, targetDir());
        setResult(res);
      } else {
        const res = await importFromCode(filePath(), targetDir());
        setResult(res);
      }
      props.onImported();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal open={true} onClose={props.onClose} title="Import Project">
      <div style={{ display: "flex", "flex-direction": "column", gap: "12px" }}>
        {/* Mode selector */}
        <div class="form-field">
          <label>Import Source</label>
          <div style={{ display: "flex", gap: "8px" }}>
            <button
              class={`btn btn-sm ${mode() === "openapi" ? "btn-primary" : "btn-secondary"}`}
              onClick={() => { setMode("openapi"); setFilePath(""); setResult(null); setError(null); }}
            >
              OpenAPI File
            </button>
            <button
              class={`btn btn-sm ${mode() === "code" ? "btn-primary" : "btn-secondary"}`}
              onClick={() => { setMode("code"); setFilePath(""); setResult(null); setError(null); }}
            >
              Existing Code
            </button>
          </div>
        </div>

        {/* Source selection */}
        <div class="form-field">
          <label>{mode() === "openapi" ? "OpenAPI File" : "Source Directory"}</label>
          <div class="input-with-action">
            <input
              type="text"
              value={filePath()}
              onInput={(e) => setFilePath(e.currentTarget.value)}
              placeholder={mode() === "openapi" ? "/path/to/openapi.json" : "/path/to/project"}
              readOnly
            />
            <button
              class="btn btn-secondary btn-sm"
              onClick={() =>
                mode() === "openapi" ? handleSelectFile() : handleSelectDir(setFilePath)
              }
            >
              Browse
            </button>
          </div>
        </div>

        {/* Target directory */}
        <div class="form-field">
          <label>Target Project Directory</label>
          <div class="input-with-action">
            <input
              type="text"
              value={targetDir()}
              onInput={(e) => setTargetDir(e.currentTarget.value)}
              placeholder="/path/to/new-project"
              readOnly
            />
            <button
              class="btn btn-secondary btn-sm"
              onClick={() => handleSelectDir(setTargetDir)}
            >
              Browse
            </button>
          </div>
        </div>

        {/* Result */}
        <Show when={result()}>
          {(res) => (
            <div style={{ "font-size": "12px" }}>
              <Show when={res().filesCreated.length > 0}>
                <div style={{ color: "var(--rash-success)", "margin-bottom": "4px" }}>
                  Created {res().filesCreated.length} file(s)
                </div>
              </Show>
              <Show when={res().warnings.length > 0}>
                <For each={res().warnings}>
                  {(w) => (
                    <div style={{ color: "var(--rash-warning)", "margin-bottom": "2px" }}>{w}</div>
                  )}
                </For>
              </Show>
            </div>
          )}
        </Show>

        <Show when={error()}>
          <div style={{ color: "var(--rash-error)", "font-size": "12px" }}>{error()}</div>
        </Show>
      </div>

      <div class="modal-footer">
        <Button variant="secondary" onClick={props.onClose}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onClick={handleImport}
          disabled={!canImport() || importing()}
        >
          {importing() ? "Importing..." : "Import"}
        </Button>
      </div>
    </Modal>
  );
};
