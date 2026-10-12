//! Errors from the workspace store.

use std::io;

use crate::format::FormatError;

/// Why a workspace operation failed. Paths are workspace-relative.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("{path}: {error}")]
    Format { path: String, error: FormatError },
    #[error("this folder is not a Damn HTTP workspace (it has no damnhttp.yaml)")]
    NotAWorkspace,
    #[error("this folder is already a Damn HTTP workspace")]
    AlreadyAWorkspace,
    #[error("`{0}` is not a valid path inside the workspace")]
    InvalidPath(String),
    #[error("`{0}` does not exist")]
    NotFound(String),
    #[error("`{path}` is not a {expected}")]
    WrongKind {
        path: String,
        expected: &'static str,
    },
    #[error("the name is empty")]
    EmptyName,
    #[error("the request being saved has a different id than the file `{0}`")]
    IdMismatch(String),
    #[error("cannot move: {0}")]
    InvalidMove(&'static str),
    #[error("{0} file(s) could not be read; fix them before clearing local data")]
    UnreadableFiles(usize),
}
