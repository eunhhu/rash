use std::path::PathBuf;
use std::sync::Mutex;

use rash_spec::index::SpecIndex;
use rash_spec::loader::LoadedProject;
use tokio::sync::Mutex as TokioMutex;

use rash_runtime::process_manager::{ProcessManager, ServerStatus};

/// Open project state
pub struct OpenProject {
    pub root: PathBuf,
    pub project: LoadedProject,
    pub index: SpecIndex,
}

/// Runtime state for the managed server process
pub struct RuntimeState {
    pub process_manager: ProcessManager,
    pub status_rx: tokio::sync::watch::Receiver<ServerStatus>,
}

/// Application state managed by Tauri
pub struct AppState {
    pub project: Mutex<Option<OpenProject>>,
    pub runtime: TokioMutex<Option<RuntimeState>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
            runtime: TokioMutex::new(None),
        }
    }
}
