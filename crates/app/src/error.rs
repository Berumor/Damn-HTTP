//! The one error type that crosses to the frontend.

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
}
