//! The one error type that crosses to the frontend.

use damnhttp_core::format::FormatErrorKind;
use damnhttp_core::store::StoreError;
use serde::Serialize;
use specta::Type;

/// An error returned by a command.
///
/// `code` is stable and is what the frontend translates. `message` is
/// English diagnostic text for logs and the Advanced view. Neither may
/// contain secret or local values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AppError {
    /// `SCREAMING_SNAKE_CASE`, for example `NOT_A_WORKSPACE`.
    pub code: String,
    pub message: String,
    /// Extra diagnostic text, such as captured git output. Shown only in
    /// the Advanced view.
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            details: None,
        }
    }

    /// A command that needs a workspace was called while none is open.
    pub fn no_workspace() -> Self {
        Self::new("NO_WORKSPACE_OPEN", "no workspace is open")
    }

    /// The webview asked for a folder the user did not choose (see
    /// `AppState::allow`).
    pub fn folder_not_allowed() -> Self {
        Self::new(
            "FOLDER_NOT_ALLOWED",
            "this folder was not chosen in the folder picker or opened before",
        )
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        let code = match &error {
            StoreError::Io { .. } => "IO",
            StoreError::Format { error, .. } => match error.kind {
                FormatErrorKind::NewerVersion { .. } => "NEWER_VERSION",
                _ => "INVALID_FILE",
            },
            StoreError::NotAWorkspace => "NOT_A_WORKSPACE",
            StoreError::AlreadyAWorkspace => "ALREADY_A_WORKSPACE",
            StoreError::InvalidPath(_) => "INVALID_PATH",
            StoreError::NotFound(_) => "NOT_FOUND",
            StoreError::WrongKind { .. } => "WRONG_KIND",
            StoreError::EmptyName => "EMPTY_NAME",
            StoreError::IdMismatch(_) => "ID_MISMATCH",
            StoreError::InvalidMove(_) => "INVALID_MOVE",
            StoreError::UnreadableFiles(_) => "UNREADABLE_FILES",
        };
        // Store messages name paths and keys, never values.
        Self::new(code, error.to_string())
    }
}
