import { Component, Show } from "solid-js";
import { useProjectStore } from "./stores/projectStore";
import { TopBar } from "./components/layout/TopBar";
import { Sidebar } from "./components/layout/Sidebar";
import { MainPanel } from "./components/layout/MainPanel";
import { BottomPanel } from "./components/layout/BottomPanel";
import { CreateProjectDialog } from "./components/dialogs/CreateProjectDialog";

const App: Component = () => {
  const { project, showCreateDialog, setShowCreateDialog, openProject } = useProjectStore();

  const handleOpenProject = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        await openProject(selected);
      }
    } catch {
      // Dialog API unavailable or user cancelled
    }
  };

  return (
    <div class="app">
      <TopBar />
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
