use std::path::PathBuf;
use std::sync::Mutex;

use rash_spec::index::SpecIndex;
use rash_spec::loader::LoadedProject;

/// Open project state
pub struct OpenProject {
    pub root: PathBuf,
    pub project: LoadedProject,
    pub index: SpecIndex,
}

/// Application state managed by Tauri
#[derive(Default)]
pub struct AppState {
    pub project: Mutex<Option<OpenProject>>,
}
