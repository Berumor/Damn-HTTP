//! Commands the frontend can call. Names follow `<area>_<verb>`.
//!
//! Each command is a thin wrapper: the work is in [`crate::ops`], which is
//! tested without Tauri. Commands that touch the disk are `async` so they
//! run off the main thread and never freeze the window.

use serde::Serialize;
use specta::Type;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::dto::{RecentWorkspace, TreeDto, WorkspaceInfo};
use crate::error::AppError;
use crate::ops;
use crate::state::AppState;

/// The command names, for the test that checks `build.rs` and the
/// capability file against them.
#[cfg(test)]
pub(crate) const NAMES: &[&str] = &[
    "app_info",
    "workspace_pick_folder",
    "workspace_create",
    "workspace_open",
    "workspace_close",
    "workspace_current",
    "workspace_recent",
    "workspace_tree",
    "collection_create",
    "folder_create",
    "request_create",
    "node_rename",
    "node_duplicate",
    "node_move",
    "node_delete",
];

/// Facts about the running app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AppInfo {
    pub name: String,
    /// The version stamped into the build; `0.0.0` in development.
    pub version: String,
    pub identifier: String,
    /// `linux`, `macos` or `windows`.
    pub platform: String,
}

#[tauri::command]
#[specta::specta]
pub fn app_info(app: tauri::AppHandle) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        identifier: app.config().identifier.clone(),
        platform: std::env::consts::OS.to_owned(),
    }
}

/// Shows the native folder dialog and returns the chosen folder, or `null`
/// if the user cancelled. Only folders chosen here (or opened before) can
/// be passed to `workspace_create` and `workspace_open`.
#[tauri::command]
#[specta::specta]
pub async fn workspace_pick_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    let Some(picked) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let folder = picked
        .into_path()
        .map_err(|_| AppError::new("INVALID_PATH", "the chosen folder is not a local path"))?;
    state.allow(&folder);
    Ok(Some(folder.to_string_lossy().into_owned()))
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_create(
    state: State<'_, AppState>,
    folder: String,
    name: String,
) -> Result<WorkspaceInfo, AppError> {
    ops::workspace_create(&state, &folder, &name)
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_open(
    state: State<'_, AppState>,
    folder: String,
) -> Result<WorkspaceInfo, AppError> {
    ops::workspace_open(&state, &folder)
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_close(state: State<'_, AppState>) -> Result<(), AppError> {
    ops::workspace_close(&state);
    Ok(())
}

/// The open workspace, or `null`. Lets the window restore itself after a reload.
#[tauri::command]
#[specta::specta]
pub async fn workspace_current(
    state: State<'_, AppState>,
) -> Result<Option<WorkspaceInfo>, AppError> {
    ops::workspace_current(&state)
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_recent(
    state: State<'_, AppState>,
) -> Result<Vec<RecentWorkspace>, AppError> {
    Ok(ops::workspace_recent(&state))
}

#[tauri::command]
#[specta::specta]
pub async fn workspace_tree(state: State<'_, AppState>) -> Result<TreeDto, AppError> {
    ops::workspace_tree(&state)
}

/// Returns the new collection's path.
#[tauri::command]
#[specta::specta]
pub async fn collection_create(
    state: State<'_, AppState>,
    name: String,
) -> Result<String, AppError> {
    ops::collection_create(&state, &name)
}

/// Returns the new folder's path.
#[tauri::command]
#[specta::specta]
pub async fn folder_create(
    state: State<'_, AppState>,
    parent: String,
    name: String,
) -> Result<String, AppError> {
    ops::folder_create(&state, &parent, &name)
}

/// Returns the new request's path.
#[tauri::command]
#[specta::specta]
pub async fn request_create(
    state: State<'_, AppState>,
    parent: String,
    name: String,
) -> Result<String, AppError> {
    ops::request_create(&state, &parent, &name)
}

/// Returns the item's path after the rename, which changes with the name.
#[tauri::command]
#[specta::specta]
pub async fn node_rename(
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> Result<String, AppError> {
    ops::node_rename(&state, &path, &name)
}

/// Returns the copy's path.
#[tauri::command]
#[specta::specta]
pub async fn node_duplicate(state: State<'_, AppState>, path: String) -> Result<String, AppError> {
    ops::node_duplicate(&state, &path)
}

/// Moves an item to `position` among the children of `new_parent` (a
/// collection or folder path; `null` reorders collections). `position`
/// counts the children without the moved item. Returns the new path.
#[tauri::command]
#[specta::specta]
pub async fn node_move(
    state: State<'_, AppState>,
    path: String,
    new_parent: Option<String>,
    position: u32,
) -> Result<String, AppError> {
    ops::node_move(&state, &path, new_parent.as_deref(), position)
}

#[tauri::command]
#[specta::specta]
pub async fn node_delete(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    ops::node_delete(&state, &path)
}
