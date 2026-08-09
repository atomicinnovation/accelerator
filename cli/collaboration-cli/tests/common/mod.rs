//! A minimal std-only HTTP/1.1 mock server for the `accelerator-collaboration`
//! end-to-end tests: fixed per-(method, path) JSON responses.
//!
//! Trimmed to this crate's own needs (no redirect/auth-recording support —
//! those are exercised at the adapter layer in `github`'s own mock server)
//! rather than shared as a crate, matching this workspace's precedent of a
//! purpose-built mock server per consumer.

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

pub struct MockServer {
    port: u16,
    shared: Arc<Shared>,
}

struct Shared {
    routes: Mutex<HashMap<(String, String), (u16, String)>>,
    stop: AtomicBool,
}

impl MockServer {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        let port = listener.local_addr().expect("addr").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let shared = Arc::new(Shared {
            routes: Mutex::new(HashMap::new()),
            stop: AtomicBool::new(false),
        });
        let server_shared = Arc::clone(&shared);
        thread::spawn(move || serve(&listener, &server_shared));
        Self { port, shared }
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn route(&self, method: &str, path: &str, status: u16, body: &str) {
        self.shared.routes.lock().expect("routes").insert(
            (method.to_owned(), path.to_owned()),
            (status, body.to_owned()),
        );
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
    let mut header = String::new();
    loop {
        header.clear();
        let read = reader.read_line(&mut header)?;
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
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
    let (status, body) = route
        .unwrap_or_else(|| (404, "{\"message\":\"Not Found\"}".to_owned()));
    let response = http_response(status, body.as_bytes());
    stream.write_all(&response)?;
    stream.flush()?;
    Ok(())
}

fn http_response(code: u16, body: &[u8]) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {code} status\r\n");
    response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    response.push_str("Content-Type: application/json\r\n");
    response.push_str("Connection: close\r\n\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}
