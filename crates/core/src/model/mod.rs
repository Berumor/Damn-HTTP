//! The data model: what a workspace is made of, independent of how it is
//! stored on disk or shown in the UI.

mod auth;
mod collection;
mod id;
mod local;
mod order;
mod request;
mod validation;
mod variable;

pub use auth::Auth;
pub use collection::{Collection, Folder, Workspace};
pub use id::{EnvironmentId, IdParseError, RequestId};
pub use local::{CommittedRequest, LocalQueryValue, RequestLocalValues};
pub use order::{OrderKey, OrderKeyError};
pub use request::{
    Body, Header, Method, Part, PartValue, PathParam, QueryParam, Request, RequestSettings,
};
pub use validation::ValidationError;
pub use variable::{Environment, Variable};
