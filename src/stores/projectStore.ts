import { createSignal } from "solid-js";
import { invoke } from "../ipc/invoke";
import type { CreateProjectArgs, ProjectTree, ValidationReport } from "../ipc/types";

function createProjectStore() {
  const [project, setProject] = createSignal<ProjectTree | null>(null);
  const [validationReport, setValidationReport] = createSignal<ValidationReport | null>(null);
  const [showCreateDialog, setShowCreateDialog] = createSignal(false);

  async function openProject(path: string): Promise<void> {
    const tree = await invoke<ProjectTree>("open_project", { path });
    setProject(tree);
    setValidationReport(null);
  }

  async function closeProject(): Promise<void> {
    await invoke<void>("close_project");
    setProject(null);
    setValidationReport(null);
  }

  async function validateProject(): Promise<ValidationReport> {
    const report = await invoke<ValidationReport>("validate_project");
    setValidationReport(report);
    return report;
  }

  async function createProject(args: CreateProjectArgs): Promise<void> {
    const tree = await invoke<ProjectTree>("create_project", { args });
    setProject(tree);
    setValidationReport(null);
    setShowCreateDialog(false);
  }

  async function refreshTree(): Promise<void> {
    const tree = await invoke<ProjectTree>("get_project_tree");
    setProject(tree);
  }

  return {
    project,
    validationReport,
    showCreateDialog,
    setShowCreateDialog,
    openProject,
    closeProject,
    validateProject,
    createProject,
    refreshTree,
  };
}

// Singleton instance
let store: ReturnType<typeof createProjectStore> | undefined;

export function useProjectStore() {
  if (!store) {
    store = createProjectStore();
  }
  return store;
}
