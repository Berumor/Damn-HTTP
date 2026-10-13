//! Errors from sending a request.

use std::error::Error as StdError;

/// Why a request could not be completed.
///
/// `detail` is English diagnostic text for logs and the Advanced view; the
/// UI shows a translated message chosen by `kind`. It never contains the
/// request URL, header values or body, which may hold secrets.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind}: {detail}")]
pub struct HttpError {
    pub kind: HttpErrorKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HttpErrorKind {
    #[error("the URL is not valid")]
    InvalidUrl,
    #[error("only http and https URLs can be sent")]
    UnsupportedScheme,
    #[error("the request still contains unresolved variables")]
    UnresolvedVariables,
    #[error("the HTTP method is not valid")]
    InvalidMethod,
    #[error("a header is not valid")]
    InvalidHeader,
    #[error("the proxy or certificate settings are not valid")]
    InvalidOptions,
    #[error("a file for the request body could not be read")]
    BodyFile,
    #[error("the request timed out")]
    Timeout,
    #[error("the server name could not be resolved")]
    Dns,
    #[error("could not connect to the server")]
    Connect,
    #[error("the secure connection could not be established")]
    Tls,
    #[error("too many redirects")]
    TooManyRedirects,
    #[error("the request was cancelled")]
    Cancelled,
    #[error("the response could not be read")]
    ResponseBody,
    #[error("the response could not be saved")]
    Io,
    #[error("the request failed")]
    Other,
}

impl HttpError {
    pub(crate) fn new(kind: HttpErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// Classifies a `reqwest` error. The URL is removed first, because a
    /// query string or user info in it may be a secret.
    pub(crate) fn from_reqwest(error: reqwest::Error) -> Self {
        let error = error.without_url();
        let detail = chain(&error);
        let lower = detail.to_lowercase();
        let looks_like_tls = ["certificate", "tls", "handshake", "unknownissuer"]
            .iter()
            .any(|word| lower.contains(word));
        let kind = if error.is_timeout() {
            HttpErrorKind::Timeout
        } else if error.is_redirect() {
            HttpErrorKind::TooManyRedirects
        } else if error.is_connect() && looks_like_tls {
            HttpErrorKind::Tls
        } else if error.is_connect() && lower.contains("dns") {
            HttpErrorKind::Dns
        } else if error.is_connect() {
            HttpErrorKind::Connect
        } else if error.is_body() || error.is_decode() {
            HttpErrorKind::ResponseBody
        } else if error.is_builder() {
            HttpErrorKind::InvalidUrl
        } else {
            HttpErrorKind::Other
        };
        Self { kind, detail }
    }
}

/// The error and its causes on one line: `outer: cause: root cause`.
fn chain(error: &dyn StdError) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        let cause_text = cause.to_string();
        // Some layers repeat their cause in their own message.
        if !text.contains(&cause_text) {
            text.push_str(": ");
            text.push_str(&cause_text);
        }
        source = cause.source();
    }
    text
}
