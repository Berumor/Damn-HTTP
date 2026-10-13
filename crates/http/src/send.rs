//! Performing the exchange.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use reqwest::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Certificate, Client, Method, Proxy, Version, multipart};
use tokio::io::AsyncWriteExt;

use crate::error::{HttpError, HttpErrorKind};
use crate::request::{HttpBody, HttpOptions, HttpRequest, MultipartValue, ProxyMode};
use crate::response::{HttpResponse, RedirectHop, Timing};
use crate::target::parse_target;

const USER_AGENT: &str = concat!("DamnHTTP/", env!("CARGO_PKG_VERSION"));

/// Sends `request` and writes the response body to `body_path`.
///
/// The body is streamed to the file as it arrives, so its size is limited
/// by the disk, not by memory. If the request fails, or the returned future
/// is dropped before it completes, the file is removed.
///
/// A response with an error status (404, 500, ...) is a successful
/// exchange and is returned as `Ok`.
pub async fn send(
    request: &HttpRequest,
    options: &HttpOptions,
    body_path: &Path,
) -> Result<HttpResponse, HttpError> {
    let url = parse_target(&request.url)?;
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|_| HttpError::new(HttpErrorKind::InvalidMethod, "not a valid method token"))?;

    let hops = Arc::new(Mutex::new(Vec::new()));
    let client = build_client(options, Arc::clone(&hops))?;
    let mut builder = client.request(method, url);

    let is_multipart = matches!(request.body, HttpBody::Multipart(_));
    let mut has_content_type = false;
    for (name, value) in &request.headers {
        // Messages name the header, never its value.
        let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            HttpError::new(
                HttpErrorKind::InvalidHeader,
                format!("invalid header name `{name}`"),
            )
        })?;
        let header_value = HeaderValue::from_bytes(value.as_bytes()).map_err(|_| {
            HttpError::new(
                HttpErrorKind::InvalidHeader,
                format!("invalid value for header `{name}`"),
            )
        })?;
        if header_name == CONTENT_TYPE {
            if is_multipart {
                // The multipart boundary is generated here; a typed
                // Content-Type could not match it.
                continue;
            }
            has_content_type = true;
        }
        builder = builder.header(header_name, header_value);
    }

    match &request.body {
        HttpBody::None => {}
        HttpBody::Raw {
            content,
            content_type,
        } => {
            if let Some(content_type) = content_type
                && !has_content_type
            {
                builder = builder.header(CONTENT_TYPE, content_type);
            }
            builder = builder.body(content.clone());
        }
        HttpBody::FormUrlencoded(fields) => {
            let mut encoder = url::form_urlencoded::Serializer::new(String::new());
            for (name, value) in fields {
                encoder.append_pair(name, value);
            }
            if !has_content_type {
                builder = builder.header(CONTENT_TYPE, "application/x-www-form-urlencoded");
            }
            builder = builder.body(encoder.finish());
        }
        HttpBody::Multipart(fields) => {
            let mut form = multipart::Form::new();
            for field in fields {
                form = match &field.value {
                    MultipartValue::Text(text) => form.text(field.name.clone(), text.clone()),
                    MultipartValue::File(path) => {
                        let part = multipart::Part::file(path).await.map_err(|error| {
                            HttpError::new(
                                HttpErrorKind::BodyFile,
                                format!("{}: {error}", path.display()),
                            )
                        })?;
                        form.part(field.name.clone(), part)
                    }
                };
            }
            builder = builder.multipart(form);
        }
    }

    let started = Instant::now();
    let mut response = builder.send().await.map_err(HttpError::from_reqwest)?;
    let time_to_headers = started.elapsed();

    let status = response.status();
    let http_version = match response.version() {
        Version::HTTP_09 => "HTTP/0.9".to_owned(),
        Version::HTTP_10 => "HTTP/1.0".to_owned(),
        Version::HTTP_11 => "HTTP/1.1".to_owned(),
        Version::HTTP_2 => "HTTP/2".to_owned(),
        Version::HTTP_3 => "HTTP/3".to_owned(),
        other => format!("{other:?}"),
    };
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let status_text = status.canonical_reason().unwrap_or_default().to_owned();
    // "HTTP/1.1 200 OK\r\n", then "name: value\r\n" per header, then "\r\n".
    let headers_size = headers
        .iter()
        .map(|(name, value)| name.len() + value.len() + 4)
        .sum::<usize>()
        + http_version.len()
        + status_text.len()
        + 9;
    let final_url = response.url().to_string();
    let remote_address = response.remote_addr().map(|address| address.to_string());

    // Armed until the body is complete: an error or a dropped future
    // removes the partial file.
    let mut partial = RemoveOnDrop::new(body_path);
    let io_error = |error: std::io::Error| HttpError::new(HttpErrorKind::Io, error.to_string());
    let mut file = tokio::fs::File::create(body_path).await.map_err(io_error)?;
    let mut body_size: u64 = 0;
    while let Some(chunk) = response.chunk().await.map_err(HttpError::from_reqwest)? {
        file.write_all(&chunk).await.map_err(io_error)?;
        body_size += chunk.len() as u64;
    }
    file.flush().await.map_err(io_error)?;
    drop(file);
    partial.keep();
    let total = started.elapsed();

    let redirects = hops.lock().map(|hops| hops.clone()).unwrap_or_default();
    Ok(HttpResponse {
        status: status.as_u16(),
        status_text,
        http_version,
        headers,
        final_url,
        redirects,
        body_path: body_path.to_path_buf(),
        body_size,
        headers_size: headers_size as u64,
        timing: Timing {
            time_to_headers,
            download: total.saturating_sub(time_to_headers),
            total,
        },
        remote_address,
    })
}

/// Like [`send`], but gives up with [`HttpErrorKind::Cancelled`] as soon as
/// `cancel` completes. The connection is closed and the partial body file
/// is removed.
pub async fn send_cancellable(
    request: &HttpRequest,
    options: &HttpOptions,
    body_path: &Path,
    cancel: impl Future<Output = ()>,
) -> Result<HttpResponse, HttpError> {
    tokio::select! {
        biased;
        () = cancel => Err(HttpError::new(HttpErrorKind::Cancelled, "cancelled by the user")),
        result = send(request, options, body_path) => result,
    }
}

fn build_client(
    options: &HttpOptions,
    hops: Arc<Mutex<Vec<RedirectHop>>>,
) -> Result<Client, HttpError> {
    let invalid = |detail: &str| HttpError::new(HttpErrorKind::InvalidOptions, detail);

    let redirect = if options.follow_redirects && options.max_redirects > 0 {
        let max = options.max_redirects;
        Policy::custom(move |attempt| {
            // `previous` holds the original URL and every redirect so far.
            if attempt.previous().len() > max {
                return attempt.error("too many redirects");
            }
            if let Ok(mut hops) = hops.lock() {
                hops.push(RedirectHop {
                    status: attempt.status().as_u16(),
                    url: attempt.url().to_string(),
                });
            }
            attempt.follow()
        })
    } else {
        Policy::none()
    };

    let mut builder = Client::builder()
        .user_agent(USER_AGENT)
        .referer(false)
        .redirect(redirect)
        .tls_danger_accept_invalid_certs(!options.verify_tls);
    if let Some(timeout) = options.timeout {
        builder = builder.timeout(timeout);
    }
    builder = match &options.proxy {
        ProxyMode::System => builder,
        ProxyMode::Disabled => builder.no_proxy(),
        // The proxy URL may contain credentials, so reqwest's message,
        // which can quote it, is not passed on.
        ProxyMode::Url(url) => {
            builder.proxy(Proxy::all(url).map_err(|_| invalid("the proxy URL is not valid"))?)
        }
    };
    let mut authorities = Vec::new();
    for pem in &options.extra_ca_pem {
        let certificates = Certificate::from_pem_bundle(pem.as_bytes())
            .map_err(|_| invalid("a custom CA certificate is not valid PEM"))?;
        if certificates.is_empty() {
            return Err(invalid("a custom CA file contains no certificate"));
        }
        authorities.extend(certificates);
    }
    if !authorities.is_empty() {
        builder = builder.tls_certs_merge(authorities);
    }
    builder.build().map_err(|error| {
        HttpError::new(
            HttpErrorKind::InvalidOptions,
            error.without_url().to_string(),
        )
    })
}

/// Removes a file when dropped, unless [`RemoveOnDrop::keep`] was called.
struct RemoveOnDrop {
    path: PathBuf,
    armed: bool,
}

impl RemoveOnDrop {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            armed: true,
        }
    }

    fn keep(&mut self) {
        self.armed = false;
    }
}

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        if self.armed {
            // Best effort: there is nobody to report a failure to.
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
