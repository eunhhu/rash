import { Component, Show } from "solid-js";
import { useProjectStore } from "./stores/projectStore";
import { TopBar } from "./components/layout/TopBar";
import { Sidebar } from "./components/layout/Sidebar";
import { MainPanel } from "./components/layout/MainPanel";
import { BottomPanel } from "./components/layout/BottomPanel";
import { CreateProjectDialog } from "./components/dialogs/CreateProjectDialog";

const App: Component = () => {
  const { project, showCreateDialog, setShowCreateDialog } = useProjectStore();

  return (
    <div class="app">
      <TopBar />
      <div class="app-body">
        <Show when={project()} fallback={<WelcomeScreen onNew={() => setShowCreateDialog(true)} />}>
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

const WelcomeScreen: Component<{ onNew: () => void }> = (props) => {
  return (
    <div class="welcome">
      <h1>Rash</h1>
      <p>Design your server visually</p>
      <div class="welcome-actions">
        <button class="btn btn-primary" onClick={props.onNew}>
          New Project
        </button>
        <button class="btn btn-secondary" onClick={() => {/* open project */}}>
          Open Project
        </button>
      </div>
    </div>
  );
};

export default App;
