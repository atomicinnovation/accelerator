//! A minimal std-only HTTP/1.1 mock server for the workspace's client tests:
//! per-`(method, path)` responses, hit counts, and the request bodies,
//! query strings and headers a test needs to assert an outbound request was
//! built as intended.
//!
//! Deliberately hand-rolled rather than built on `wiremock`, `mockito` or
//! `httpmock`: none of them can hold a connection open after promising a body
//! ([`Route::Stall`]), which is what makes a client's read timeout observable,
//! so the hand-rolled responder would survive alongside any of them.
//!
//! The server panics on a setup mistake — a poisoned lock, an unbindable
//! loopback port — because that is a fault in the test harness rather than a
//! condition a caller can recover from.
#![allow(clippy::expect_used, clippy::missing_panics_doc)]

use std::collections::HashMap;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// The status returned for a request matching no registered route, chosen
/// outside the range any route registers so a mis-keyed registration fails
/// loudly rather than being mistaken for a deliberate 404.
pub const UNMATCHED_STATUS: u16 = 599;

/// What the server returns for a `(method, path)` pair.
#[derive(Clone)]
pub enum Route {
    /// This status with this JSON body and a JSON content type.
    Json { status: u16, body: String },
    /// This status with these bytes and no content type.
    Bytes { status: u16, body: Vec<u8> },
    /// This status with an empty body.
    Status(u16),
    /// This status with these response headers and this body — the shape a
    /// client's `Retry-After` handling needs.
    Headers {
        status: u16,
        headers: Vec<(String, String)>,
        body: String,
    },
    /// A redirect response to this absolute Location.
    Redirect { status: u16, location: String },
    /// 500 for the first `fail_times` hits, then 200 with the bytes.
    FlakyThenOk { fail_times: usize, body: Vec<u8> },
    /// Send headers promising a body, then go idle for this long without
    /// sending it — so the client's body read stalls (exercises read timeout).
    Stall(Duration),
}

/// The routing and recording key: a method and a path, with any query string
/// held separately so a test can assert on it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RequestKey {
    pub method: String,
    pub path: String,
}

impl RequestKey {
    #[must_use]
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_ascii_uppercase(),
            path: path.to_owned(),
        }
    }

    #[must_use]
    pub fn get(path: &str) -> Self {
        Self::new("GET", path)
    }

    #[must_use]
    pub fn post(path: &str) -> Self {
        Self::new("POST", path)
    }

    #[must_use]
    pub fn put(path: &str) -> Self {
        Self::new("PUT", path)
    }
}

#[derive(Default)]
struct Received {
    query: Option<String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Default)]
struct Record {
    hits: usize,
    last: Option<Received>,
}

struct Shared {
    routes: Mutex<HashMap<RequestKey, Route>>,
    records: Mutex<HashMap<RequestKey, Record>>,
    stop: AtomicBool,
}

pub struct MockServer {
    port: u16,
    shared: Arc<Shared>,
}

impl MockServer {
    /// Binds an ephemeral loopback port and starts serving in a background
    /// thread.
    #[must_use]
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        let port = listener.local_addr().expect("addr").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let shared = Arc::new(Shared {
            routes: Mutex::new(HashMap::new()),
            records: Mutex::new(HashMap::new()),
            stop: AtomicBool::new(false),
        });
        let server_shared = Arc::clone(&shared);
        thread::spawn(move || serve(&listener, &server_shared));
        Self { port, shared }
    }

    /// The base URL for the server (no trailing slash).
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn route(&self, key: RequestKey, route: Route) {
        self.shared
            .routes
            .lock()
            .expect("routes")
            .insert(key, route);
    }

    /// How many requests exactly matching `key` were received.
    #[must_use]
    pub fn hits(&self, key: &RequestKey) -> usize {
        self.shared
            .records
            .lock()
            .expect("records")
            .get(key)
            .map_or(0, |record| record.hits)
    }

    /// The body of the most recent request matching `key`, or `None` if no
    /// such request was received.
    #[must_use]
    pub fn last_body(&self, key: &RequestKey) -> Option<Vec<u8>> {
        self.with_last(key, |received| Some(received.body.clone()))
    }

    /// The query string of the most recent request matching `key`, without
    /// its leading `?`, or `None` if there was no such request or it carried
    /// no query.
    #[must_use]
    pub fn last_query(&self, key: &RequestKey) -> Option<String> {
        self.with_last(key, |received| received.query.clone())
    }

    /// The named header of the most recent request matching `key`, matched
    /// case-insensitively.
    #[must_use]
    pub fn last_header(&self, key: &RequestKey, name: &str) -> Option<String> {
        let name = name.to_ascii_lowercase();
        self.with_last(key, |received| received.headers.get(&name).cloned())
    }

    fn with_last<T>(
        &self,
        key: &RequestKey,
        read: impl FnOnce(&Received) -> Option<T>,
    ) -> Option<T> {
        self.shared
            .records
            .lock()
            .expect("records")
            .get(key)
            .and_then(|record| record.last.as_ref())
            .and_then(read)
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
    // The accepted socket may inherit the listener's non-blocking flag; force
    // blocking so a large body's write_all cannot fail with WouldBlock.
    stream.set_nonblocking(false)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_ascii_uppercase();
    let target = parts.next().unwrap_or("/");
    let (path, query) = target.split_once('?').map_or_else(
        || (target.to_owned(), None),
        |(path, query)| (path.to_owned(), Some(query.to_owned())),
    );

    let mut headers: HashMap<String, String> = HashMap::new();
    let mut header = String::new();
    loop {
        header.clear();
        let read = reader.read_line(&mut header)?;
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            headers.insert(
                name.trim().to_ascii_lowercase(),
                value.trim().to_owned(),
            );
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0_u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    let key = RequestKey { method, path };
    // Record before any response byte is written: the retry-count assertions
    // read this the instant the client call returns.
    let index = {
        let mut records = shared.records.lock().expect("records");
        let record = records.entry(key.clone()).or_default();
        record.last = Some(Received {
            query,
            headers,
            body,
        });
        record.hits += 1;
        let index = record.hits - 1;
        drop(records);
        index
    };

    let route = shared.routes.lock().expect("routes").get(&key).cloned();
    let response = match route {
        Some(Route::Json { status, body }) => http_response(
            status,
            &[("Content-Type", "application/json")],
            body.as_bytes(),
        ),
        Some(Route::Bytes { status, body }) => {
            http_response(status, &[], &body)
        }
        Some(Route::Status(status)) => http_response(status, &[], &[]),
        Some(Route::Headers {
            status,
            headers,
            body,
        }) => {
            let borrowed: Vec<(&str, &str)> = headers
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()))
                .collect();
            http_response(status, &borrowed, body.as_bytes())
        }
        Some(Route::Redirect { status, location }) => {
            http_response(status, &[("Location", location.as_str())], &[])
        }
        Some(Route::FlakyThenOk { fail_times, body }) => {
            if index < fail_times {
                http_response(500, &[], &[])
            } else {
                http_response(200, &[], &body)
            }
        }
        Some(Route::Stall(delay)) => {
            // Promise a body in the headers, flush them, then stall without
            // sending it, so the client blocks reading the body.
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n",
            )?;
            stream.flush()?;
            thread::sleep(delay);
            return Ok(());
        }
        None => http_response(UNMATCHED_STATUS, &[], &[]),
    };
    stream.write_all(&response)?;
    stream.flush()?;
    // Read-to-end guard so the client isn't reset before it reads the body.
    let mut sink = Vec::new();
    let _ = reader.get_mut().read_to_end(&mut sink);
    Ok(())
}

fn http_response(code: u16, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut bytes = format!(
        "HTTP/1.1 {code} status\r\n\
         Content-Length: {length}\r\n\
         Connection: close\r\n",
        length = body.len()
    )
    .into_bytes();
    for (name, value) in headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(body);
    bytes
}
