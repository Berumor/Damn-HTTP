//! Commands the frontend can call. Names follow `<area>_<verb>`.

use serde::Serialize;
use specta::Type;

/// The command names, for the test that checks `build.rs` and the
/// capability file against them.
#[cfg(test)]
pub(crate) const NAMES: &[&str] = &["app_info"];

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
