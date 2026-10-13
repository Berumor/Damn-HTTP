//! What the app remembers while it runs.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use damnhttp_core::store::WorkspaceStore;

use crate::dto::RecentWorkspace;
use crate::error::AppError;

const RECENT_FILE: &str = "recent-workspaces.json";
const MAX_RECENT: usize = 10;

/// Shared by every command.
pub struct AppState {
    /// The open workspace, if any. One window, one workspace.
    workspace: Mutex<Option<WorkspaceStore>>,
    /// Folders the webview may ask to open or create a workspace in.
    ///
    /// The webview cannot name arbitrary folders: a folder gets here only
    /// when the user picks it in the native folder dialog, or when it is in
    /// the list of workspaces opened before. This keeps a compromised page
    /// from pointing the app at any directory on the machine.
    allowed: Mutex<HashSet<PathBuf>>,
    /// Where app-level data (the recent list) is stored.
    data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let state = Self {
            workspace: Mutex::new(None),
            allowed: Mutex::new(HashSet::new()),
            data_dir,
        };
        for recent in state.recent() {
            state.allow(Path::new(&recent.path));
        }
        state
    }

    /// Marks `folder` as chosen by the user.
    pub fn allow(&self, folder: &Path) {
        lock(&self.allowed).insert(normalize(folder));
    }

    pub fn is_allowed(&self, folder: &Path) -> bool {
        lock(&self.allowed).contains(&normalize(folder))
    }

    pub fn set_workspace(&self, store: Option<WorkspaceStore>) {
        *lock(&self.workspace) = store;
    }

    /// Runs `action` on the open workspace.
    pub fn with_workspace<T>(
        &self,
        action: impl FnOnce(&WorkspaceStore) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        match lock(&self.workspace).as_ref() {
            Some(store) => action(store),
            None => Err(AppError::no_workspace()),
        }
    }

    /// Workspaces opened before, most recent first. Folders that no longer
    /// exist are left out.
    pub fn recent(&self) -> Vec<RecentWorkspace> {
        let Ok(text) = fs::read_to_string(self.data_dir.join(RECENT_FILE)) else {
            return Vec::new();
        };
        // A damaged list is not worth an error: start again from empty.
        let list: Vec<RecentWorkspace> = serde_json::from_str(&text).unwrap_or_default();
        list.into_iter()
            .filter(|recent| Path::new(&recent.path).is_dir())
            .collect()
    }

    /// Puts a workspace at the top of the recent list.
    pub fn remember(&self, name: &str, root: &Path) {
        let path = root.to_string_lossy().into_owned();
        let mut list = self.recent();
        list.retain(|recent| recent.path != path);
        list.insert(
            0,
            RecentWorkspace {
                name: name.to_owned(),
                path,
            },
        );
        list.truncate(MAX_RECENT);
        // Best effort: failing to save the list must not stop the user
        // from working in the workspace that was just opened.
        if fs::create_dir_all(&self.data_dir).is_ok()
            && let Ok(text) = serde_json::to_string_pretty(&list)
        {
            let _ = fs::write(self.data_dir.join(RECENT_FILE), text);
        }
    }
}

/// A poisoned lock only means another command panicked; the data is still
/// usable, so keep going instead of failing every later command.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The same folder always compares equal: symbolic links and `..` are
/// resolved when the folder exists.
fn normalize(folder: &Path) -> PathBuf {
    fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf())
}
