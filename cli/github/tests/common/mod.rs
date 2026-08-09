//! A minimal std-only HTTP/1.1 mock server for the `OctocrabClient` tests:
//! fixed per-(method, path) JSON/redirect responses, and the most recently
//! received `Authorization` header — so a test can assert the configured
//! token actually reached the outbound request.
//!
//! Ported from `cli/launcher/tests/common/mod.rs`'s `MockServer` structure
//! (this crate's real needs — a GET/PATCH JSON responder, no retry logic to
//! test since `retry` is not in this client's request path) rather than
//! shared as a crate, mirroring that file's own precedent for HTTP-level
//! test stubbing in this workspace (no `wiremock`/`mockito`).

#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::format_push_string
)]

use std::collections::HashMap;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// What the server returns for a `(method, path)` pair.
#[derive(Clone)]
pub enum Route {
    /// This status code with this JSON body.
    Json { status: u16, body: String },
    /// A redirect response (3xx) to this absolute Location.
    Redirect { status: u16, location: String },
}

struct Shared {
    routes: Mutex<HashMap<(String, String), Route>>,
    last_authorization: Mutex<Option<String>>,
    stop: AtomicBool,
}

pub struct MockServer {
    port: u16,
    shared: Arc<Shared>,
}

impl MockServer {
    /// Binds an ephemeral loopback port and starts serving in a background
    /// thread.
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        let port = listener.local_addr().expect("addr").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let shared = Arc::new(Shared {
            routes: Mutex::new(HashMap::new()),
            last_authorization: Mutex::new(None),
            stop: AtomicBool::new(false),
        });
        let server_shared = Arc::clone(&shared);
        thread::spawn(move || serve(&listener, &server_shared));
        Self { port, shared }
    }

    /// The base URI for the server, for `OctocrabClient::with_base_uri`.
    pub fn base_uri(&self) -> http::Uri {
        format!("http://127.0.0.1:{}", self.port)
            .parse()
            .expect("base uri")
    }

    pub fn route(&self, method: &str, path: &str, route: Route) {
        self.shared
            .routes
            .lock()
            .expect("routes")
            .insert((method.to_owned(), path.to_owned()), route);
    }

    /// The `Authorization` header value of the most recently received
    /// request, or `None` if none carried one.
    pub fn last_authorization(&self) -> Option<String> {
        self.shared.last_authorization.lock().expect("auth").clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
    }
}

fn serve(listener: &TcpListener, shared: &Arc<Shared>) {
    while !shared.stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let conn_shared = Arc::clone(shared);
                thread::spawn(move || {
                    let _ = handle(stream, &conn_shared);
                });
            }
            Err(ref error)
                if error.kind() == std::io::ErrorKind::WouldBlock =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }
}

fn handle(mut stream: TcpStream, shared: &Arc<Shared>) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_owned();
    let path = parts
        .next()
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_owned();

    let mut content_length: usize = 0;
    let mut authorization: Option<String> = None;
    let mut header = String::new();
    loop {
        header.clear();
        let read = reader.read_line(&mut header)?;
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            let value = value.trim().to_owned();
            match name.to_ascii_lowercase().as_str() {
                "content-length" => {
                    content_length = value.parse().unwrap_or(0);
                }
                "authorization" => authorization = Some(value),
                _ => {}
            }
        }
    }
    *shared.last_authorization.lock().expect("auth") = authorization;

    let mut body = vec![0_u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    let route = shared
        .routes
        .lock()
        .expect("routes")
        .get(&(method, path))
        .cloned();
    let response = match route {
        Some(Route::Json { status, body }) => {
            http_response(status, &[], body.as_bytes())
        }
        Some(Route::Redirect { status, location }) => {
            http_response(status, &[("Location", location.as_str())], &[])
        }
        None => http_response(404, &[], b"{\"message\":\"Not Found\"}"),
    };
    stream.write_all(&response)?;
    stream.flush()?;
    Ok(())
}

fn http_response(code: u16, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {code} status\r\n");
    response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    response.push_str("Content-Type: application/json\r\n");
    response.push_str("Connection: close\r\n");
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}
