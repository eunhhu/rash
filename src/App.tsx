import { Component, Show, createSignal, onMount, onCleanup } from "solid-js";
import { useProjectStore } from "./stores/projectStore";
import { useEditorStore } from "./stores/editorStore";
import { useUiStore } from "./stores/uiStore";
import { setupKeyboardShortcuts } from "./utils/keyboard";
import { TopBar } from "./components/layout/TopBar";
import { Sidebar } from "./components/layout/Sidebar";
import { MainPanel } from "./components/layout/MainPanel";
import { BottomPanel } from "./components/layout/BottomPanel";
import { CreateProjectDialog } from "./components/dialogs/CreateProjectDialog";
import { ImportDialog } from "./components/dialogs/ImportDialog";
import { ExportDialog } from "./components/dialogs/ExportDialog";
import { ToastContainer } from "./components/common/ToastContainer";
import { ConfirmDialog } from "./components/common/ConfirmDialog";
import { ContextMenu } from "./components/common/ContextMenu";

const App: Component = () => {
  const { project, showCreateDialog, setShowCreateDialog, openProject, refreshTree } = useProjectStore();
  const { activeTabId, closeTab } = useEditorStore();
  const { startCreate } = useUiStore();
  const [showImportDialog, setShowImportDialog] = createSignal(false);
  const [showExportDialog, setShowExportDialog] = createSignal(false);

  const handleOpenProject = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        await openProject(selected);
      }
    } catch (err) {
      console.error("Failed to open project:", err);
    }
  };

  onMount(() => {
    // Auto-open mock project in browser (non-Tauri) environment
    if (!(window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ && !project()) {
      openProject("/mock/demo-api").catch(() => {});
    }

    const cleanup = setupKeyboardShortcuts({
      newSpec: () => {
        startCreate("route");
      },
      save: () => {
        // Save is handled by the active editor component
      },
      search: () => {
        const searchInput = document.querySelector<HTMLInputElement>(
          ".sidebar input[type='text']"
        );
        searchInput?.focus();
      },
      closeTab: () => {
        const id = activeTabId();
        if (id) closeTab(id);
      },
    });
    onCleanup(cleanup);
  });

  return (
    <div class="app">
      <TopBar
        onImport={() => setShowImportDialog(true)}
        onExport={() => setShowExportDialog(true)}
      />
      <div class="app-body">
        <Show when={project()} fallback={<WelcomeScreen onNew={() => setShowCreateDialog(true)} onOpen={handleOpenProject} />}>
          <Sidebar />
          <div class="app-main">
            <MainPanel />
            <BottomPanel />
          </div>
        </Show>
      </div>
      <Show when={showCreateDialog()}>
        <CreateProjectDialog onClose={() => setShowCreateDialog(false)} />
      </Show>
      <Show when={showImportDialog()}>
        <ImportDialog
          onClose={() => setShowImportDialog(false)}
          onImported={() => refreshTree().catch(() => {})}
        />
      </Show>
      <Show when={showExportDialog()}>
        <ExportDialog onClose={() => setShowExportDialog(false)} />
      </Show>
      <ToastContainer />
      <ConfirmDialog />
      <ContextMenu />
    </div>
  );
};

const WelcomeScreen: Component<{ onNew: () => void; onOpen: () => void }> = (props) => {
  return (
    <div class="welcome">
      <h1>Rash</h1>
      <p>Design your server visually</p>
      <div class="welcome-actions">
        <button class="btn btn-primary" onClick={props.onNew}>
          New Project
        </button>
        <button class="btn btn-secondary" onClick={props.onOpen}>
          Open Project
        </button>
      </div>
    </div>
  );
};

export default App;
