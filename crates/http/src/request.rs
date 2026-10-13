//! What to send and how.

use std::path::PathBuf;
use std::time::Duration;

/// A request with every variable already resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// An HTTP method token, such as `GET`.
    pub method: String,
    /// The full URL including the query string. `http://` is assumed when
    /// the scheme is missing (`localhost:8080/x`). The query is sent as
    /// written: only characters that cannot appear in a URL are
    /// percent-encoded, and existing `%XX` sequences and `+` are left alone.
    pub url: String,
    /// Sent in this order. A `Content-Type` here replaces the one implied
    /// by the body (except for multipart, whose boundary must match).
    pub headers: Vec<(String, String)>,
    pub body: HttpBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HttpBody {
    #[default]
    None,
    /// Bytes sent as they are, for JSON and text bodies.
    Raw {
        content: Vec<u8>,
        content_type: Option<String>,
    },
    /// `application/x-www-form-urlencoded`.
    FormUrlencoded(Vec<(String, String)>),
    /// `multipart/form-data`.
    Multipart(Vec<MultipartField>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartField {
    pub name: String,
    pub value: MultipartValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultipartValue {
    Text(String),
    /// An absolute path; the file is streamed, not loaded into memory.
    File(PathBuf),
}

/// How to route requests through a proxy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxyMode {
    /// Use the operating system or environment settings (`HTTPS_PROXY`, ...).
    #[default]
    System,
    /// Connect directly, ignoring any system setting.
    Disabled,
    /// Send everything through this proxy (`http://`, `https://` or `socks5://`).
    Url(String),
}

/// Transport settings for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpOptions {
    pub follow_redirects: bool,
    /// Ignored when `follow_redirects` is false.
    pub max_redirects: usize,
    /// For the whole exchange, including the download. `None` waits forever.
    pub timeout: Option<Duration>,
    /// `false` accepts any certificate ("skip TLS verification").
    pub verify_tls: bool,
    pub proxy: ProxyMode,
    /// Extra certificate authorities to trust, as PEM text (one or more
    /// certificates per entry), in addition to the system's.
    pub extra_ca_pem: Vec<String>,
}

impl Default for HttpOptions {
    fn default() -> Self {
        Self {
            follow_redirects: true,
            max_redirects: 10,
            timeout: Some(Duration::from_secs(30)),
            verify_tls: true,
            proxy: ProxyMode::default(),
            extra_ca_pem: Vec::new(),
        }
    }
}
