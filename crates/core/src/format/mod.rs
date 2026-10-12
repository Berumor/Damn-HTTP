//! The file format: reading and writing workspace files (D-008, D-009).
//!
//! The format itself is specified in `docs/FORMAT.md`, with JSON Schemas in
//! `schemas/`. In short:
//!
//! - **Writing** is done by our own emitter over a small YAML subset, with a
//!   fixed key order per kind of file, so the same model always gives the
//!   same bytes. Defaults are left out.
//! - **Reading** uses `saphyr-parser` and maps the result onto the model
//!   strictly: unknown keys, wrong types, anchors, aliases and tags are
//!   errors with a line number. A file with a newer `version` than
//!   [`FORMAT_VERSION`] is refused with [`FormatErrorKind::NewerVersion`].
//! - **Committed files never hold local data.** A request file is written
//!   from a [`CommittedRequest`](crate::model::CommittedRequest) only
//!   (D-033), and a secret variable is written without its value (D-019).
//!
//! These functions work on text. Paths, file names and directories are the
//! workspace store's job.

mod common;
mod containers;
mod emit;
mod environment;
mod error;
mod read;
mod request;
mod tree;
mod values;

#[cfg(test)]
mod tests;

pub use containers::{
    read_collection, read_folder, read_workspace, write_collection, write_folder, write_workspace,
};
pub use environment::{EnvironmentFile, read_environment, write_environment};
pub use error::{FormatError, FormatErrorKind};
pub use request::{RequestFile, read_request, write_request};
pub use values::{read_local_values, write_local_values};

/// The newest format version this build reads and the version it writes.
pub const FORMAT_VERSION: u64 = 1;
