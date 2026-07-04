//! A minimal hand-rolled HTTP/1.1 mock server for the resolution tests.
//!
//! Std-only (no async runtime, no HTTP dev-dep — keeping cargo-deny's dev-dep
//! scan TLS-free). Serves fixed per-path responses, counts hits, and supports
//! 404/5xx/redirect shapes.

// Test-infrastructure module: expect/unwrap and format!-into-String are allowed.
#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::format_push_string
)]

use std::collections::HashMap;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// What the server returns for a path.
#[derive(Clone)]
pub enum Route {
    /// 200 with these bytes.
    Ok(Vec<u8>),
    /// This status code with an empty body.
    Status(u16),
    /// 302 redirect to this absolute Location.
    Redirect(String),
    /// 500 for the first `fail_times` hits, then 200 with the bytes.
    FlakyThenOk { fail_times: usize, body: Vec<u8> },
    /// Send headers promising a body, then go idle for this long without
    /// sending it — so the client's body read stalls (exercises read timeout).
    Stall(Duration),
}

struct Shared {
    routes: Mutex<HashMap<String, Route>>,
    hits: Mutex<HashMap<String, Arc<AtomicUsize>>>,
    stop: AtomicBool,
}

pub struct MockServer {
    port: u16,
    shared: Arc<Shared>,
}

impl MockServer {
    /// Bind an ephemeral loopback port and start serving in a background thread.
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        let port = listener.local_addr().expect("addr").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let shared = Arc::new(Shared {
            routes: Mutex::new(HashMap::new()),
            hits: Mutex::new(HashMap::new()),
            stop: AtomicBool::new(false),
        });
        let server_shared = Arc::clone(&shared);
        thread::spawn(move || serve(&listener, &server_shared));
        Self { port, shared }
    }

    /// The base URL for the server (no trailing slash).
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn route(&self, path: &str, route: Route) {
        self.shared
            .routes
            .lock()
            .expect("routes")
            .insert(path.to_owned(), route);
        self.shared
            .hits
            .lock()
            .expect("hits")
            .entry(path.to_owned())
            .or_insert_with(|| Arc::new(AtomicUsize::new(0)));
    }

    /// How many times `path` was requested.
    pub fn hits(&self, path: &str) -> usize {
        self.shared
            .hits
            .lock()
            .expect("hits")
            .get(path)
            .map_or(0, |count| count.load(Ordering::SeqCst))
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
    // Drain headers (we ignore them, but must consume to be well-behaved).
    let mut header = String::new();
    loop {
        header.clear();
        let read = reader.read_line(&mut header)?;
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
    }

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_owned();

    let hit = {
        let hits = shared.hits.lock().expect("hits");
        hits.get(&path).map(Arc::clone)
    };
    let index = hit.map_or(0, |count| count.fetch_add(1, Ordering::SeqCst));

    let route = shared.routes.lock().expect("routes").get(&path).cloned();
    let response = match route {
        Some(Route::Ok(body)) => http_response(200, "OK", &[], &body),
        Some(Route::Status(code)) => http_response(code, "STATUS", &[], &[]),
        Some(Route::Redirect(location)) => {
            http_response(302, "Found", &[("Location", location.as_str())], &[])
        }
        Some(Route::FlakyThenOk { fail_times, body }) => {
            if index < fail_times {
                http_response(500, "ERR", &[], &[])
            } else {
                http_response(200, "OK", &[], &body)
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
        None => http_response(404, "Not Found", &[], &[]),
    };
    stream.write_all(&response)?;
    stream.flush()?;
    // Read-to-end guard so the client isn't reset before it reads the body.
    let mut sink = Vec::new();
    let _ = reader.get_mut().read_to_end(&mut sink);
    Ok(())
}

fn http_response(
    code: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {code} {reason}\r\n");
    response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    response.push_str("Connection: close\r\n");
    for (name, value) in headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}
