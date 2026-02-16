import { Component, createEffect, createMemo, createSignal } from "solid-js";
import { Modal } from "../common/Modal";
import { Input } from "../common/Input";
import { Select } from "../common/Select";
import { Button } from "../common/Button";
import { useProjectStore } from "../../stores/projectStore";
import type { Language, Framework, Runtime } from "../../ipc/types";
import "../layout/layout.css";

// --- Framework / Runtime mappings per language ---

const frameworksByLanguage: Record<Language, { value: string; label: string }[]> = {
  typescript: [
    { value: "express", label: "Express" },
    { value: "fastify", label: "Fastify" },
    { value: "hono", label: "Hono" },
    { value: "elysia", label: "Elysia" },
    { value: "nestjs", label: "NestJS" },
  ],
  rust: [
    { value: "actix", label: "Actix Web" },
    { value: "axum", label: "Axum" },
    { value: "rocket", label: "Rocket" },
  ],
  python: [
    { value: "fastapi", label: "FastAPI" },
    { value: "flask", label: "Flask" },
    { value: "django", label: "Django" },
  ],
  go: [
    { value: "gin", label: "Gin" },
    { value: "echo", label: "Echo" },
    { value: "fiber", label: "Fiber" },
  ],
};

const runtimesByLanguage: Record<Language, { value: string; label: string }[]> = {
  typescript: [
    { value: "bun", label: "Bun" },
    { value: "node", label: "Node.js" },
    { value: "deno", label: "Deno" },
  ],
  rust: [
    { value: "cargo", label: "Cargo" },
  ],
  python: [
    { value: "python", label: "Python" },
  ],
  go: [
    { value: "go", label: "Go" },
  ],
};

const languageOptions = [
  { value: "typescript", label: "TypeScript" },
  { value: "rust", label: "Rust" },
  { value: "python", label: "Python" },
  { value: "go", label: "Go" },
];

// --- Component ---

interface CreateProjectDialogProps {
  onClose: () => void;
}

export const CreateProjectDialog: Component<CreateProjectDialogProps> = (props) => {
  const { createProject } = useProjectStore();

  const [name, setName] = createSignal("");
  const [path, setPath] = createSignal("");
  const [language, setLanguage] = createSignal<Language>("typescript");
  const [framework, setFramework] = createSignal("");
  const [runtime, setRuntime] = createSignal("");
  const [creating, setCreating] = createSignal(false);

  const frameworkOptions = createMemo(() => frameworksByLanguage[language()] ?? []);
  const runtimeOptions = createMemo(() => runtimesByLanguage[language()] ?? []);

  // Reset framework & runtime when language changes
  createEffect(() => {
    const fw = frameworksByLanguage[language()];
    const rt = runtimesByLanguage[language()];
    setFramework(fw?.[0]?.value ?? "");
    setRuntime(rt?.[0]?.value ?? "");
  });

  const canCreate = () =>
    name().trim().length > 0 &&
    path().trim().length > 0 &&
    framework().length > 0 &&
    runtime().length > 0;

  const handleBrowse = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        setPath(selected);
      }
    } catch {
      // Dialog API unavailable (e.g., running outside Tauri)
    }
  };

  const handleCreate = async () => {
    if (!canCreate() || creating()) return;
    setCreating(true);
    try {
      await createProject({
        name: name().trim(),
        path: path().trim(),
        language: language(),
        framework: framework(),
        runtime: runtime(),
      });
      props.onClose();
    } catch {
      // TODO: surface error in dialog
    } finally {
      setCreating(false);
    }
  };

  return (
    <Modal open={true} onClose={props.onClose} title="New Project">
      <div style={{ display: "flex", "flex-direction": "column", gap: "12px" }}>
        <Input
          label="Project Name"
          value={name()}
          onInput={setName}
          placeholder="my-api"
        />

        <div class="form-field">
          <label for="project-path">Project Path</label>
          <div class="input-with-action">
            <input
              id="project-path"
              type="text"
              value={path()}
              onInput={(e) => setPath(e.currentTarget.value)}
              placeholder="/path/to/project"
            />
            <button class="btn btn-secondary btn-sm" onClick={handleBrowse}>
              Browse
            </button>
          </div>
        </div>

        <Select
          label="Language"
          value={language()}
          onChange={(v) => setLanguage(v as Language)}
          options={languageOptions}
        />

        <div class="form-row">
          <Select
            label="Framework"
            value={framework()}
            onChange={setFramework}
            options={frameworkOptions()}
          />
          <Select
            label="Runtime"
            value={runtime()}
            onChange={setRuntime}
            options={runtimeOptions()}
          />
        </div>
      </div>

      <div class="modal-footer">
        <Button variant="secondary" onClick={props.onClose}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onClick={handleCreate}
          disabled={!canCreate() || creating()}
        >
          {creating() ? "Creating..." : "Create Project"}
        </Button>
      </div>
    </Modal>
  );
};
