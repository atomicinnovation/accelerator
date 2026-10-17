//! A minimal std-only HTTP/1.1 mock server for the resolution tests: fixed
//! per-path responses, hit counts, and 404/5xx/redirect/stall shapes.

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
use std::sync::mpsc::{self, RecvTimeoutError};
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
    /// As `Ok`, except that the first ranged request is answered with a 206
    /// promising the whole suffix, half of which is sent before the
    /// connection closes.
    FailFirstRange(Vec<u8>),
}

struct Shared {
    routes: Mutex<HashMap<String, Route>>,
    hits: Mutex<HashMap<String, Arc<AtomicUsize>>>,
    ranges: Mutex<HashMap<String, Arc<AtomicUsize>>>,
    stop: AtomicBool,
}

pub struct MockHTTPServer {
    port: u16,
    shared: Arc<Shared>,
}

impl MockHTTPServer {
    /// Bind an ephemeral loopback port and start serving in a background thread.
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
        let port = listener.local_addr().expect("addr").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let shared = Arc::new(Shared {
            routes: Mutex::new(HashMap::new()),
            hits: Mutex::new(HashMap::new()),
            ranges: Mutex::new(HashMap::new()),
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
        self.shared
            .ranges
            .lock()
            .expect("ranges")
            .entry(path.to_owned())
            .or_insert_with(|| Arc::new(AtomicUsize::new(0)));
    }

    /// How many `Range` requests `path` received.
    pub fn range_hits(&self, path: &str) -> usize {
        self.shared
            .ranges
            .lock()
            .expect("ranges")
            .get(path)
            .map_or(0, |count| count.load(Ordering::SeqCst))
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

impl Drop for MockHTTPServer {
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
    // Capture any `Range: bytes=N-` so the Ok route can serve a 206 partial,
    // or a 416 past the end, as a real server does.
    let mut range_from: Option<u64> = None;
    let mut header = String::new();
    loop {
        header.clear();
        let read = reader.read_line(&mut header)?;
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some(spec) = header
            .to_ascii_lowercase()
            .strip_prefix("range:")
            .map(str::trim)
            .and_then(|value| value.strip_prefix("bytes="))
            .and_then(|value| value.split('-').next())
        {
            range_from = spec.trim().parse().ok();
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

    let earlier_ranges = if range_from.is_some() {
        let ranges = shared.ranges.lock().expect("ranges");
        ranges
            .get(&path)
            .map(|count| count.fetch_add(1, Ordering::SeqCst))
    } else {
        None
    };

    let route = shared.routes.lock().expect("routes").get(&path).cloned();
    let route = match (route, range_from, earlier_ranges) {
        (Some(Route::FailFirstRange(body)), Some(offset), Some(0)) => {
            return send_half_of_the_suffix(&mut stream, &body, offset);
        }
        (Some(Route::FailFirstRange(body)), _, _) => Some(Route::Ok(body)),
        (route, _, _) => route,
    };
    let response = match route {
        Some(Route::Ok(body)) => ranged_response(&body, range_from),
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
        Some(Route::FailFirstRange(_)) => {
            unreachable!("a first-range failure resolves before dispatch")
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

fn ranged_response(body: &[u8], range_from: Option<u64>) -> Vec<u8> {
    match range_from.and_then(|offset| usize::try_from(offset).ok()) {
        Some(offset) if offset < body.len() => {
            let total = body.len();
            let suffix = body[offset..].to_vec();
            http_response(
                206,
                "Partial Content",
                &[(
                    "Content-Range",
                    &format!("bytes {offset}-{}/{total}", total - 1),
                )],
                &suffix,
            )
        }
        Some(_) => http_response(
            416,
            "Range Not Satisfiable",
            &[("Content-Range", &format!("bytes */{}", body.len()))],
            &[],
        ),
        None => http_response(200, "OK", &[], body),
    }
}

fn send_half_of_the_suffix(
    stream: &mut TcpStream,
    body: &[u8],
    offset: u64,
) -> std::io::Result<()> {
    let offset = usize::try_from(offset).unwrap();
    let suffix = &body[offset..];
    let head = format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\n\
         Content-Range: bytes {offset}-{}/{}\r\n\
         Connection: close\r\n\r\n",
        suffix.len(),
        body.len() - 1,
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&suffix[..suffix.len() / 2])?;
    stream.flush()
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

/// Run `run` on a detached thread, failing rather than hanging if it outlasts
/// `bound`; a panic inside it is re-raised as itself.
#[allow(clippy::panic)]
pub fn within<T: Send + 'static>(
    bound: Duration,
    what: &str,
    run: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let _ = sender.send(run());
    });
    match receiver.recv_timeout(bound) {
        Ok(outcome) => outcome,
        Err(RecvTimeoutError::Timeout) => panic!("{what} blocked"),
        Err(RecvTimeoutError::Disconnected) => match worker.join() {
            Err(panic) => std::panic::resume_unwind(panic),
            Ok(()) => panic!("{what} ended without an outcome"),
        },
    }
}

#[allow(clippy::panic)]
mod tests {
    use std::time::Duration;

    use super::within;

    #[test]
    #[should_panic(expected = "the run blocked")]
    fn a_run_that_outlasts_its_bound_fails_instead_of_hanging() {
        within(Duration::from_millis(100), "the run", || {
            std::thread::sleep(Duration::from_secs(2));
        });
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn a_panic_within_a_bounded_run_is_reraised_not_reported_as_blocked() {
        within::<()>(Duration::from_secs(10), "the run", || panic!("boom"));
    }
}
