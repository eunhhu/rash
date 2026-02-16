import { Component, Show, createSignal, onMount } from "solid-js";
import { Modal } from "../common/Modal";
import { Button } from "../common/Button";
import { exportOpenapi } from "../../ipc/commands";

interface ExportDialogProps {
  onClose: () => void;
}

export const ExportDialog: Component<ExportDialogProps> = (props) => {
  const [json, setJson] = createSignal("");
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [copied, setCopied] = createSignal(false);

  onMount(async () => {
    try {
      const result = await exportOpenapi();
      setJson(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  });

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(json());
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard API unavailable
    }
  };

  const handleSave = async () => {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        filters: [{ name: "JSON", extensions: ["json"] }],
        defaultPath: "openapi.json",
      });
      if (path) {
        const { writeTextFile } = await import("@tauri-apps/plugin-fs");
        await writeTextFile(path, json());
      }
    } catch {
      // Dialog unavailable or user cancelled
    }
  };

  return (
    <Modal open={true} onClose={props.onClose} title="Export OpenAPI">
      <div style={{ display: "flex", "flex-direction": "column", gap: "12px" }}>
        <Show when={loading()}>
          <div style={{ color: "var(--rash-text-muted)", "font-size": "13px" }}>
            Generating OpenAPI spec...
          </div>
        </Show>

        <Show when={error()}>
          <div style={{ color: "var(--rash-error)", "font-size": "12px" }}>{error()}</div>
        </Show>

        <Show when={!loading() && !error()}>
          <textarea
            readOnly
            value={json()}
            style={{
              width: "100%",
              height: "320px",
              resize: "vertical",
              "font-family": "var(--rash-font)",
              "font-size": "12px",
              background: "var(--rash-bg-tertiary)",
              color: "var(--rash-text)",
              border: "1px solid var(--rash-border)",
              "border-radius": "var(--rash-radius-sm)",
              padding: "8px",
            }}
          />
        </Show>
      </div>

      <div class="modal-footer">
        <Button variant="secondary" onClick={props.onClose}>
          Close
        </Button>
        <Show when={!loading() && !error()}>
          <Button variant="secondary" onClick={handleCopy}>
            {copied() ? "Copied!" : "Copy"}
          </Button>
          <Button variant="primary" onClick={handleSave}>
            Save as File
          </Button>
        </Show>
      </div>
    </Modal>
  );
};
