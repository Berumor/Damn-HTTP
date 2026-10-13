//! Tests against a small local server. Nothing here touches the network
//! beyond the loopback interface.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose,
};
use tempfile::TempDir;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

use super::*;

// ----- a tiny HTTP/1.1 server -----

#[derive(Debug, Clone)]
struct Received {
    method: String,
    /// The request target as sent: a path, or a full URL through a proxy.
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Received {
    fn headers_named(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers_named(name).first().copied()
    }

    fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

#[derive(Debug, Clone, Default)]
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    /// Wait before sending anything.
    delay: Duration,
    /// Wait between the headers and the body.
    body_delay: Duration,
}

impl Reply {
    fn ok(body: &str) -> Self {
        Self {
            status: 200,
            body: body.as_bytes().to_vec(),
            ..Self::default()
        }
    }

    fn redirect(status: u16, location: &str) -> Self {
        Self {
            status,
            headers: vec![("Location".to_owned(), location.to_owned())],
            ..Self::default()
        }
    }

    fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

type Handler = Arc<dyn Fn(&Received) -> Reply + Send + Sync>;

struct Server {
    address: SocketAddr,
    received: Arc<Mutex<Vec<Received>>>,
}

impl Server {
    async fn start(handler: impl Fn(&Received) -> Reply + Send + Sync + 'static) -> Self {
        Self::start_with(handler, None).await
    }

    async fn start_with(
        handler: impl Fn(&Received) -> Reply + Send + Sync + 'static,
        tls: Option<TlsAcceptor>,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));
        let handler: Handler = Arc::new(handler);
        let log = Arc::clone(&received);
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let handler = Arc::clone(&handler);
                let log = Arc::clone(&log);
                let tls = tls.clone();
                tokio::spawn(async move {
                    match tls {
                        // A failed handshake is what some tests are about.
                        Some(acceptor) => {
                            if let Ok(stream) = acceptor.accept(stream).await {
                                serve(stream, handler, log).await;
                            }
                        }
                        None => serve(stream, handler, log).await,
                    }
                });
            }
        });
        Self { address, received }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }

    fn https_url(&self, path: &str) -> String {
        format!("https://{}{path}", self.address)
    }

    fn requests(&self) -> Vec<Received> {
        self.received.lock().unwrap().clone()
    }

    fn only_request(&self) -> Received {
        let requests = self.requests();
        assert_eq!(requests.len(), 1, "{requests:?}");
        requests.into_iter().next().unwrap()
    }
}

/// Reads one request, answers it and closes the connection.
async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    handler: Handler,
    log: Arc<Mutex<Vec<Received>>>,
) {
    let mut buffer = Vec::new();
    let head_end = loop {
        if let Some(position) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break position + 4;
        }
        let mut chunk = [0u8; 4096];
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
        }
    };
    let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split(' ');
    let method = request_line.next().unwrap_or_default().to_owned();
    let target = request_line.next().unwrap_or_default().to_owned();
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    let mut body = buffer[head_end..].to_vec();
    while body.len() < length {
        let mut chunk = [0u8; 65536];
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(read) => body.extend_from_slice(&chunk[..read]),
        }
    }

    let received = Received {
        method,
        target,
        headers,
        body,
    };
    let reply = handler(&received);
    log.lock().unwrap().push(received);

    tokio::time::sleep(reply.delay).await;
    let mut head = format!("HTTP/1.1 {} Test\r\n", reply.status);
    for (name, value) in &reply.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        reply.body.len()
    ));
    // The client may have gone away (cancelled, timed out): ignore errors.
    let _ = stream.write_all(head.as_bytes()).await;
    let _ = stream.flush().await;
    tokio::time::sleep(reply.body_delay).await;
    let _ = stream.write_all(&reply.body).await;
    let _ = stream.shutdown().await;
}

// ----- helpers -----

/// Options for tests: never use a proxy from the environment.
fn options() -> HttpOptions {
    HttpOptions {
        proxy: ProxyMode::Disabled,
        timeout: Some(Duration::from_secs(10)),
        ..HttpOptions::default()
    }
}

fn get(url: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".to_owned(),
        url: url.to_owned(),
        headers: Vec::new(),
        body: HttpBody::None,
    }
}

fn post(url: &str, body: HttpBody) -> HttpRequest {
    HttpRequest {
        method: "POST".to_owned(),
        body,
        ..get(url)
    }
}

struct Scratch {
    dir: TempDir,
}

impl Scratch {
    fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
    }

    fn body_path(&self) -> std::path::PathBuf {
        self.dir.path().join("body.bin")
    }
}

fn header<'a>(response: &'a HttpResponse, name: &str) -> Option<&'a str> {
    response
        .headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

// ----- the basics -----

#[tokio::test]
async fn a_get_returns_status_headers_body_sizes_and_timing() {
    let server = Server::start(|_| Reply::ok("hello").header("X-Test", "yes")).await;
    let scratch = Scratch::new();

    let response = send(
        &get(&server.url("/hello")),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.status_text, "OK");
    assert_eq!(response.http_version, "HTTP/1.1");
    assert_eq!(header(&response, "x-test"), Some("yes"));
    assert_eq!(response.final_url, server.url("/hello"));
    assert_eq!(response.redirects, []);
    assert_eq!(response.body_path, scratch.body_path());
    assert_eq!(std::fs::read(scratch.body_path()).unwrap(), b"hello");
    assert_eq!(response.body_size, 5);
    assert!(response.headers_size > 40, "{}", response.headers_size);
    assert_eq!(response.remote_address, Some(server.address.to_string()));
    assert!(response.timing.total >= response.timing.time_to_headers);
    assert_eq!(
        response.timing.total,
        response.timing.time_to_headers + response.timing.download
    );

    let request = server.only_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.target, "/hello");
    assert!(
        request
            .header("user-agent")
            .unwrap()
            .starts_with("DamnHTTP/")
    );
    assert_eq!(request.header("referer"), None);
}

#[tokio::test]
async fn an_error_status_is_still_a_response() {
    let server = Server::start(|_| Reply {
        status: 404,
        body: b"nope".to_vec(),
        ..Reply::default()
    })
    .await;
    let scratch = Scratch::new();

    let response = send(
        &get(&server.url("/missing")),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap();

    assert_eq!(
        (response.status, response.status_text.as_str()),
        (404, "Not Found")
    );
    assert_eq!(std::fs::read(scratch.body_path()).unwrap(), b"nope");
}

#[tokio::test]
async fn method_headers_and_query_are_sent_as_given() {
    let server = Server::start(|_| Reply::ok("")).await;
    let scratch = Scratch::new();
    let request = HttpRequest {
        method: "PROPFIND".to_owned(),
        url: server.url("/a%2Fb/c d?q=hello%20world+x&plus=%2B&a=b c&empty=&flag"),
        headers: vec![
            ("X-Token".to_owned(), "abc 123".to_owned()),
            ("Accept".to_owned(), "application/json".to_owned()),
            ("X-Multi".to_owned(), "one".to_owned()),
            ("X-Multi".to_owned(), "two".to_owned()),
            ("User-Agent".to_owned(), "custom/1.0".to_owned()),
            ("X-Unicode".to_owned(), "città".to_owned()),
        ],
        body: HttpBody::None,
    };

    send(&request, &options(), &scratch.body_path())
        .await
        .unwrap();

    let seen = server.only_request();
    assert_eq!(seen.method, "PROPFIND");
    assert_eq!(
        seen.target,
        "/a%2Fb/c%20d?q=hello%20world+x&plus=%2B&a=b%20c&empty=&flag"
    );
    assert_eq!(seen.header("x-token"), Some("abc 123"));
    assert_eq!(seen.header("accept"), Some("application/json"));
    assert_eq!(seen.headers_named("x-multi"), ["one", "two"]);
    assert_eq!(seen.headers_named("user-agent"), ["custom/1.0"]);
    assert_eq!(seen.header("x-unicode"), Some("città"));
}

#[tokio::test]
async fn http_is_assumed_when_the_scheme_is_missing() {
    let server = Server::start(|_| Reply::ok("ok")).await;
    let scratch = Scratch::new();

    let response = send(
        &get(&format!("{}/no-scheme", server.address)),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(server.only_request().target, "/no-scheme");
}

// ----- bodies -----

#[tokio::test]
async fn a_raw_body_is_sent_with_its_content_type() {
    let server = Server::start(|_| Reply::ok("")).await;
    let scratch = Scratch::new();
    let body = HttpBody::Raw {
        content: br#"{"name":"Rex"}"#.to_vec(),
        content_type: Some("application/json".to_owned()),
    };

    send(
        &post(&server.url("/pets"), body.clone()),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap();
    let mut with_header = post(&server.url("/pets"), body);
    with_header.headers.push((
        "content-type".to_owned(),
        "application/vnd.api+json".to_owned(),
    ));
    send(&with_header, &options(), &scratch.body_path())
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].body_text(), r#"{"name":"Rex"}"#);
    assert_eq!(
        requests[0].headers_named("content-type"),
        ["application/json"]
    );
    // A Content-Type typed by the user replaces the implied one.
    assert_eq!(
        requests[1].headers_named("content-type"),
        ["application/vnd.api+json"]
    );
}

#[tokio::test]
async fn a_form_body_is_url_encoded() {
    let server = Server::start(|_| Reply::ok("")).await;
    let scratch = Scratch::new();
    let body = HttpBody::FormUrlencoded(vec![
        ("name".to_owned(), "Rex & Co".to_owned()),
        ("città".to_owned(), "a=b+c".to_owned()),
        ("empty".to_owned(), String::new()),
    ]);

    send(
        &post(&server.url("/form"), body),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap();

    let seen = server.only_request();
    assert_eq!(
        seen.header("content-type"),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(
        seen.body_text(),
        "name=Rex+%26+Co&citt%C3%A0=a%3Db%2Bc&empty="
    );
}

#[tokio::test]
async fn a_multipart_body_carries_text_and_files() {
    let server = Server::start(|_| Reply::ok("")).await;
    let scratch = Scratch::new();
    let file = scratch.dir.path().join("avatar.png");
    let file_bytes: Vec<u8> = (0..=255u8).cycle().take(100_000).collect();
    std::fs::write(&file, &file_bytes).unwrap();
    let body = HttpBody::Multipart(vec![
        MultipartField {
            name: "title".to_owned(),
            value: MultipartValue::Text("My pet".to_owned()),
        },
        MultipartField {
            name: "avatar".to_owned(),
            value: MultipartValue::File(file),
        },
    ]);
    let mut request = post(&server.url("/upload"), body);
    // Ignored: the boundary is generated when the body is built.
    request.headers.push((
        "Content-Type".to_owned(),
        "multipart/form-data; boundary=mine".to_owned(),
    ));

    send(&request, &options(), &scratch.body_path())
        .await
        .unwrap();

    let seen = server.only_request();
    let content_types = seen.headers_named("content-type");
    assert_eq!(content_types.len(), 1);
    let boundary = content_types[0]
        .strip_prefix("multipart/form-data; boundary=")
        .unwrap();
    assert_ne!(boundary, "mine");
    let text = seen.body_text();
    assert!(text.contains(boundary));
    assert!(text.contains("name=\"title\"") && text.contains("My pet"));
    assert!(
        text.contains("name=\"avatar\"; filename=\"avatar.png\""),
        "{}",
        &text[..300]
    );
    assert!(text.contains("image/png"));
    assert!(seen.body.windows(1000).any(|w| w == &file_bytes[..1000]));
    assert!(seen.body.len() > file_bytes.len());
}

#[tokio::test]
async fn a_missing_body_file_is_reported_before_anything_is_sent() {
    let server = Server::start(|_| Reply::ok("")).await;
    let scratch = Scratch::new();
    let body = HttpBody::Multipart(vec![MultipartField {
        name: "avatar".to_owned(),
        value: MultipartValue::File(scratch.dir.path().join("missing.png")),
    }]);

    let error = send(
        &post(&server.url("/upload"), body),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.kind, HttpErrorKind::BodyFile);
    assert!(error.detail.contains("missing.png"));
    assert_eq!(server.requests().len(), 0);
}

#[tokio::test]
async fn a_large_response_is_streamed_to_the_file() {
    let big: Vec<u8> = (0..8 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let body = big.clone();
    let server = Server::start(move |_| Reply {
        status: 200,
        body: body.clone(),
        ..Reply::default()
    })
    .await;
    let scratch = Scratch::new();

    let response = send(&get(&server.url("/big")), &options(), &scratch.body_path())
        .await
        .unwrap();

    assert_eq!(response.body_size, big.len() as u64);
    assert_eq!(std::fs::read(scratch.body_path()).unwrap(), big);
}

#[tokio::test]
async fn a_compressed_response_is_stored_decompressed() {
    // gzip of "hello gzip\n" three times.
    const GZIPPED: &[u8] = &[
        31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 203, 72, 205, 201, 201, 87, 72, 175, 202, 44, 224, 202,
        192, 202, 4, 0, 58, 167, 104, 204, 33, 0, 0, 0,
    ];
    let server = Server::start(|_| Reply {
        status: 200,
        headers: vec![("Content-Encoding".to_owned(), "gzip".to_owned())],
        body: GZIPPED.to_vec(),
        ..Reply::default()
    })
    .await;
    let scratch = Scratch::new();

    let response = send(&get(&server.url("/gz")), &options(), &scratch.body_path())
        .await
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(scratch.body_path()).unwrap(),
        "hello gzip\nhello gzip\nhello gzip\n"
    );
    assert_eq!(response.body_size, 33);
    assert!(
        server
            .only_request()
            .header("accept-encoding")
            .unwrap()
            .contains("gzip")
    );
}

// ----- redirects -----

fn redirecting(request: &Received) -> Reply {
    match request.target.as_str() {
        "/start" => Reply::redirect(302, "/middle"),
        "/middle" => Reply::redirect(307, "/end?x=1"),
        "/loop" => Reply::redirect(302, "/loop"),
        _ => Reply::ok("arrived"),
    }
}

#[tokio::test]
async fn redirects_are_followed_and_recorded() {
    let server = Server::start(redirecting).await;
    let scratch = Scratch::new();

    let response = send(
        &get(&server.url("/start")),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.final_url, server.url("/end?x=1"));
    assert_eq!(
        response.redirects,
        [
            RedirectHop {
                status: 302,
                url: server.url("/middle")
            },
            RedirectHop {
                status: 307,
                url: server.url("/end?x=1")
            },
        ]
    );
    assert_eq!(std::fs::read(scratch.body_path()).unwrap(), b"arrived");
}

#[tokio::test]
async fn redirects_can_be_turned_off() {
    let server = Server::start(redirecting).await;
    let scratch = Scratch::new();
    for no_follow in [
        HttpOptions {
            follow_redirects: false,
            ..options()
        },
        HttpOptions {
            max_redirects: 0,
            ..options()
        },
    ] {
        let response = send(
            &get(&server.url("/start")),
            &no_follow,
            &scratch.body_path(),
        )
        .await
        .unwrap();
        assert_eq!(response.status, 302);
        assert_eq!(header(&response, "location"), Some("/middle"));
        assert_eq!(response.redirects, []);
    }
    assert_eq!(server.requests().len(), 2);
}

#[tokio::test]
async fn too_many_redirects_is_an_error() {
    let server = Server::start(redirecting).await;
    let scratch = Scratch::new();
    let limited = HttpOptions {
        max_redirects: 3,
        ..options()
    };

    let error = send(&get(&server.url("/loop")), &limited, &scratch.body_path())
        .await
        .unwrap_err();
    assert_eq!(error.kind, HttpErrorKind::TooManyRedirects);
    // The original request and three redirects were made, not a fourth.
    assert_eq!(server.requests().len(), 4);
    assert!(!scratch.body_path().exists());

    // Exactly at the limit is fine.
    let exact = HttpOptions {
        max_redirects: 2,
        ..options()
    };
    let response = send(&get(&server.url("/start")), &exact, &scratch.body_path())
        .await
        .unwrap();
    assert_eq!(response.redirects.len(), 2);
}

// ----- failures -----

#[tokio::test]
async fn a_slow_server_times_out() {
    let server = Server::start(|_| Reply {
        delay: Duration::from_secs(5),
        ..Reply::ok("late")
    })
    .await;
    let scratch = Scratch::new();
    let impatient = HttpOptions {
        timeout: Some(Duration::from_millis(150)),
        ..options()
    };

    let error = send(&get(&server.url("/slow")), &impatient, &scratch.body_path())
        .await
        .unwrap_err();

    assert_eq!(error.kind, HttpErrorKind::Timeout);
    assert!(!scratch.body_path().exists());
}

#[tokio::test]
async fn a_timeout_during_the_download_removes_the_partial_file() {
    let server = Server::start(|_| Reply {
        body_delay: Duration::from_secs(5),
        ..Reply::ok("late body")
    })
    .await;
    let scratch = Scratch::new();
    let impatient = HttpOptions {
        timeout: Some(Duration::from_millis(300)),
        ..options()
    };

    let error = send(
        &get(&server.url("/slow-body")),
        &impatient,
        &scratch.body_path(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.kind, HttpErrorKind::Timeout);
    assert!(!scratch.body_path().exists());
}

#[tokio::test]
async fn a_closed_port_is_a_connection_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let scratch = Scratch::new();

    let error = send(
        &get(&format!("http://{address}/secret-path?token=hunter2")),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.kind, HttpErrorKind::Connect);
    // The URL may hold secrets; it must not be in the message.
    let message = error.to_string();
    assert!(
        !message.contains("hunter2") && !message.contains("secret-path"),
        "{message}"
    );
}

#[tokio::test]
async fn an_unknown_host_is_a_dns_error() {
    let scratch = Scratch::new();
    // `.invalid` is reserved and never resolves (RFC 2606); no packet leaves
    // the machine for it beyond the local resolver.
    let error = send(
        &get("http://damnhttp-test.invalid/x"),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind, HttpErrorKind::Dns, "{error}");
}

#[tokio::test]
async fn invalid_requests_are_refused_before_connecting() {
    let server = Server::start(|_| Reply::ok("")).await;
    let scratch = Scratch::new();
    let kind_of = |request: HttpRequest, options: HttpOptions| {
        let path = scratch.body_path();
        async move { send(&request, &options, &path).await.unwrap_err().kind }
    };
    let url = server.url("/x");

    assert_eq!(
        kind_of(get("ftp://x.test/"), options()).await,
        HttpErrorKind::UnsupportedScheme
    );
    assert_eq!(kind_of(get(""), options()).await, HttpErrorKind::InvalidUrl);
    assert_eq!(
        kind_of(get("{{baseUrl}}/x"), options()).await,
        HttpErrorKind::UnresolvedVariables
    );
    let mut bad_method = get(&url);
    bad_method.method = "BAD METHOD".to_owned();
    assert_eq!(
        kind_of(bad_method, options()).await,
        HttpErrorKind::InvalidMethod
    );
    let mut bad_name = get(&url);
    bad_name
        .headers
        .push(("Bad Name".to_owned(), "v".to_owned()));
    assert_eq!(
        kind_of(bad_name, options()).await,
        HttpErrorKind::InvalidHeader
    );
    let bad_proxy = HttpOptions {
        proxy: ProxyMode::Url("not a proxy url".to_owned()),
        ..options()
    };
    assert_eq!(
        kind_of(get(&url), bad_proxy).await,
        HttpErrorKind::InvalidOptions
    );
    let bad_ca = HttpOptions {
        extra_ca_pem: vec!["this is not a certificate".to_owned()],
        ..options()
    };
    assert_eq!(
        kind_of(get(&url), bad_ca).await,
        HttpErrorKind::InvalidOptions
    );

    assert_eq!(server.requests().len(), 0);
}

#[tokio::test]
async fn an_invalid_header_value_is_reported_without_the_value() {
    let scratch = Scratch::new();
    let mut request = get("http://127.0.0.1:1/x");
    request.headers.push((
        "Authorization".to_owned(),
        "Bearer secret\nInjected: yes".to_owned(),
    ));

    let error = send(&request, &options(), &scratch.body_path())
        .await
        .unwrap_err();

    assert_eq!(error.kind, HttpErrorKind::InvalidHeader);
    assert!(error.detail.contains("Authorization"));
    assert!(!error.to_string().contains("secret"));
}

// ----- cancelling -----

#[tokio::test]
async fn cancelling_stops_the_request_and_removes_the_partial_file() {
    let server = Server::start(|_| Reply {
        body_delay: Duration::from_secs(10),
        ..Reply::ok("never fully sent")
    })
    .await;
    let scratch = Scratch::new();
    let started = std::time::Instant::now();

    let error = send_cancellable(
        &get(&server.url("/slow-body")),
        &options(),
        &scratch.body_path(),
        tokio::time::sleep(Duration::from_millis(300)),
    )
    .await
    .unwrap_err();

    assert_eq!(error.kind, HttpErrorKind::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(server.requests().len(), 1, "the request had been sent");
    assert!(!scratch.body_path().exists());
}

#[tokio::test]
async fn a_request_that_is_not_cancelled_completes() {
    let server = Server::start(|_| Reply::ok("done")).await;
    let scratch = Scratch::new();

    let response = send_cancellable(
        &get(&server.url("/")),
        &options(),
        &scratch.body_path(),
        std::future::pending(),
    )
    .await
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(std::fs::read(scratch.body_path()).unwrap(), b"done");
}

// ----- proxy -----

#[tokio::test]
async fn a_configured_proxy_receives_the_request() {
    // The "proxy" is the test server: a proxied request has a full URL as
    // its target, which is what it records.
    let proxy = Server::start(|_| Reply::ok("from the proxy")).await;
    let scratch = Scratch::new();
    let through_proxy = HttpOptions {
        proxy: ProxyMode::Url(format!("http://{}", proxy.address)),
        ..options()
    };

    let response = send(
        &get("http://upstream.invalid/data?x=1"),
        &through_proxy,
        &scratch.body_path(),
    )
    .await
    .unwrap();

    assert_eq!(
        std::fs::read(scratch.body_path()).unwrap(),
        b"from the proxy"
    );
    assert_eq!(response.remote_address, Some(proxy.address.to_string()));
    let seen = proxy.only_request();
    assert_eq!(seen.target, "http://upstream.invalid/data?x=1");
    assert_eq!(seen.header("host"), Some("upstream.invalid"));
}

// ----- TLS -----

struct TestAuthority {
    ca_pem: String,
    acceptor: TlsAcceptor,
}

/// A throwaway certificate authority and a server certificate for
/// `127.0.0.1` signed by it.
fn test_authority() -> TestAuthority {
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "Damn HTTP test CA");
    let ca_certificate = ca_params.self_signed(&ca_key).unwrap();
    let issuer = Issuer::new(ca_params, ca_key);

    let server_key = KeyPair::generate().unwrap();
    let mut server_params = CertificateParams::new(vec!["127.0.0.1".to_owned()]).unwrap();
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server_certificate = server_params.signed_by(&server_key, &issuer).unwrap();

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![server_certificate.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(server_key.serialize_der())),
        )
        .unwrap();
    TestAuthority {
        ca_pem: ca_certificate.pem(),
        acceptor: TlsAcceptor::from(Arc::new(config)),
    }
}

#[tokio::test]
async fn an_untrusted_certificate_is_refused_by_default() {
    let authority = test_authority();
    let server = Server::start_with(|_| Reply::ok("secure"), Some(authority.acceptor)).await;
    let scratch = Scratch::new();

    let error = send(
        &get(&server.https_url("/")),
        &options(),
        &scratch.body_path(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.kind, HttpErrorKind::Tls, "{error}");
    assert_eq!(server.requests().len(), 0);
}

#[tokio::test]
async fn skipping_verification_accepts_any_certificate() {
    let authority = test_authority();
    let server = Server::start_with(|_| Reply::ok("secure"), Some(authority.acceptor)).await;
    let scratch = Scratch::new();
    let trusting = HttpOptions {
        verify_tls: false,
        ..options()
    };

    let response = send(
        &get(&server.https_url("/")),
        &trusting,
        &scratch.body_path(),
    )
    .await
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(std::fs::read(scratch.body_path()).unwrap(), b"secure");
}

#[tokio::test]
async fn a_custom_certificate_authority_is_trusted() {
    let authority = test_authority();
    let other_authority = test_authority();
    let server = Server::start_with(|_| Reply::ok("secure"), Some(authority.acceptor)).await;
    let scratch = Scratch::new();

    let with_ca = HttpOptions {
        extra_ca_pem: vec![authority.ca_pem.clone()],
        ..options()
    };
    let response = send(&get(&server.https_url("/")), &with_ca, &scratch.body_path())
        .await
        .unwrap();
    assert_eq!(response.status, 200);

    // Trusting a different authority does not help.
    let with_wrong_ca = HttpOptions {
        extra_ca_pem: vec![other_authority.ca_pem],
        ..options()
    };
    let error = send(
        &get(&server.https_url("/")),
        &with_wrong_ca,
        &scratch.body_path(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind, HttpErrorKind::Tls, "{error}");

    // Public certificate authorities are still trusted alongside a custom one:
    // the custom CA is merged with the system's, not a replacement.
    let plain = Server::start(|_| Reply::ok("plain")).await;
    let response = send(&get(&plain.url("/")), &with_ca, &scratch.body_path())
        .await
        .unwrap();
    assert_eq!(response.status, 200);
}
