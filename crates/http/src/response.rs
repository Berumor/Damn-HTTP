//! What came back.

use std::path::PathBuf;
use std::time::Duration;

/// A completed exchange. The body is in a file, not in memory, so a large
/// response costs nothing until the UI asks for a part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    /// The standard reason phrase for `status` (`Not Found`); empty for an
    /// unknown code. The phrase the server actually sent is not available.
    pub status_text: String,
    /// `HTTP/1.1`, `HTTP/2`, ...
    pub http_version: String,
    /// In the order received. Values that are not UTF-8 are converted lossily.
    pub headers: Vec<(String, String)>,
    /// The URL that produced this response, after redirects.
    pub final_url: String,
    /// The redirects that were followed, in order.
    pub redirects: Vec<RedirectHop>,
    /// The file the body was written to (the path given to `send`).
    pub body_path: PathBuf,
    /// Bytes in the body file, after decompression.
    pub body_size: u64,
    /// Approximate size of the status line and headers, in bytes.
    pub headers_size: u64,
    pub timing: Timing,
    /// The address the response came from (the proxy's, when one is used).
    pub remote_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectHop {
    /// The status of the response that redirected (301, 302, ...).
    pub status: u16,
    /// Where it redirected to.
    pub url: String,
}

/// How long the exchange took. Connection setup (DNS, TCP, TLS) is part of
/// `time_to_headers`; it is not measured separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Timing {
    /// From sending the request to receiving the response headers,
    /// including any redirects.
    pub time_to_headers: Duration,
    /// Reading the body.
    pub download: Duration,
    pub total: Duration,
}
