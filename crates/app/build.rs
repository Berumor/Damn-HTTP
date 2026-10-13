//! Tauri build script.
//!
//! Every command the frontend may call is listed here. Tauri then generates
//! one permission per command (`allow-app-info`, ...), and a command is only
//! callable from a window whose capability grants it
//! (`capabilities/default.json`). A command missing from this list cannot be
//! called at all.

const COMMANDS: &[&str] = &["app_info"];

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    // A build script reports failure by panicking.
    #[allow(clippy::expect_used)]
    tauri_build::try_build(attributes).expect("failed to run the Tauri build script");
}
