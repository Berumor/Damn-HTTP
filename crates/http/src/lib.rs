//! HTTP request execution for Damn HTTP.
//!
//! [`send`] takes a fully resolved [`HttpRequest`] (no `{{variables}}` left)
//! and [`HttpOptions`], performs the exchange with `reqwest` over `rustls`,
//! streams the response body to a file and returns an [`HttpResponse`] with
//! status, headers, sizes and timing. Nothing here depends on Tauri or on
//! anything UI-related, so a command-line runner can use it as it is.
//!
//! Requests are sent from here, never from the webview.

mod error;
mod request;
mod response;
mod send;
mod target;

#[cfg(test)]
mod tests;

pub use error::{HttpError, HttpErrorKind};
pub use request::{HttpBody, HttpOptions, HttpRequest, MultipartField, MultipartValue, ProxyMode};
pub use response::{HttpResponse, RedirectHop, Timing};
pub use send::{send, send_cancellable};
