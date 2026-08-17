//! Blocking reqwest fetch with timeouts, retry, and a redirect host-allowlist
//! matched at a dotted-label boundary (so `evil-githubusercontent.com` and
//! `githubusercontent.com.attacker.net` are refused).

use std::io::Read as _;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::redirect::{Attempt, Policy};
use sha2::{Digest as _, Sha256};

const MAX_ATTEMPTS: u32 = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
// Whole-request deadline per attempt for the buffered small-asset path, sized
// for a multi-MB release binary over a slow link. On the streaming path it
// bounds one read rather than the attempt, so `StreamLimits` carries the
// attempt and whole-loop bounds instead.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(300);
// The tree resolver is the only consumer of the streaming transport, and it
// lands separately from the transport so each is verified on its own.
// The stall bound on the streaming path. A per-request timeout bounds one
// *read* once a body is streamed rather than the whole attempt, which is
// exactly the idle bound wanted here: a slow-but-progressing transfer resets it
// on every chunk, while a connection that stops sending fails within it instead
// of waiting out a deadline sized for a ~120MB archive. The attempt and
// whole-loop deadlines are enforced by the copy loop, which can only run
// between reads and so depends on this bounding each one.
#[allow(dead_code)]
const READ_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 10;
const CDN_HOST_SUFFIX: &str = ".githubusercontent.com";
const RELEASE_ORIGIN_HOST: &str = "github.com";
#[allow(dead_code)]
const STREAM_CHUNK: usize = 64 * 1024;

/// Why a fetch did not yield bytes.
///
/// A definitive 404 is not retried; a transport error or exhausted 5xx retries
/// is `Unreachable`; a body exceeding its declared size is a defect rather than
/// a transient failure, so it is not retried either.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchError {
    NotFound,
    Unreachable(String),
    TooLarge { limit: u64 },
}

/// The bounds a streamed transfer runs under.
///
/// `total_deadline` exists because `attempt_timeout` alone multiplies by the
/// attempt cap: a transfer deadline sized for a large archive would otherwise
/// become a half-hour wait inside a tool call with no progress output and no
/// cancel.
#[allow(dead_code)]
pub(super) struct StreamLimits {
    pub attempt_timeout: Duration,
    pub total_deadline: Duration,
    pub idle_timeout: Duration,
    pub max_bytes: u64,
}

#[allow(dead_code)]
impl StreamLimits {
    /// The bounds a tree archive is fetched under: a per-attempt deadline sized
    /// for a ~120MB compressed archive at a 200 KB/s sustained floor, a whole-
    /// loop bound so three attempts cannot compound into a half-hour wait, and
    /// the stall bound.
    #[must_use]
    pub(super) const fn for_archive(max_bytes: u64) -> Self {
        Self {
            attempt_timeout: Duration::from_secs(600),
            total_deadline: Duration::from_secs(900),
            idle_timeout: READ_IDLE_TIMEOUT,
            max_bytes,
        }
    }
}

/// A per-attempt destination for a streamed body.
///
/// The fetcher requests a fresh sink at the start of every attempt, so an
/// attempt that failed partway cannot leave its bytes — or any digest state the
/// caller keeps alongside them — for the next attempt to append to.
#[allow(dead_code)]
pub(super) trait StreamSink {
    /// Persist the next chunk.
    ///
    /// # Errors
    ///
    /// Whatever the underlying destination reports; the fetcher treats it as a
    /// retryable failure of this attempt.
    fn accept(&mut self, chunk: &[u8]) -> std::io::Result<()>;
}

/// What a completed stream transferred, digested in the one pass.
#[derive(Debug)]
#[allow(dead_code)]
pub(super) struct StreamedBody {
    pub bytes: u64,
    pub sha256: [u8; 32],
}

/// Whether a redirect target host is permitted, matched at a dotted-label
/// boundary.
#[must_use]
pub fn is_allowed_redirect_host(host: &str) -> bool {
    host == RELEASE_ORIGIN_HOST || host.ends_with(CDN_HOST_SUFFIX)
}

/// Whether a URL uses the required `https` scheme.
#[must_use]
pub fn is_https(url: &str) -> bool {
    url.starts_with("https://")
}

fn redirect_policy() -> Policy {
    Policy::custom(|attempt: Attempt| {
        if attempt.previous().len() > MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        match attempt.url().host_str() {
            Some(host) if is_allowed_redirect_host(host) => attempt.follow(),
            _ => attempt.stop(),
        }
    })
}

/// A configured blocking HTTP client for asset/manifest fetches.
pub struct Fetcher {
    client: Client,
    max_attempts: u32,
    backoff: Duration,
    require_https: bool,
}

impl Fetcher {
    /// Build the production fetcher (https pinned).
    ///
    /// # Errors
    ///
    /// If the underlying client cannot be constructed.
    pub fn new() -> Result<Self, String> {
        Self::build(Duration::from_millis(250), true)
    }

    /// Build a test fetcher with a caller-chosen backoff, permitting `http` for
    /// a local mock server.
    ///
    /// # Errors
    ///
    /// If the underlying client cannot be constructed.
    pub fn with_backoff(backoff: Duration) -> Result<Self, String> {
        Self::build(backoff, false)
    }

    fn build(backoff: Duration, require_https: bool) -> Result<Self, String> {
        // The ring provider must be installed before building a TLS client
        // (idempotent), keeping the Fetcher self-sufficient in tests.
        let _ = rustls::crypto::ring::default_provider().install_default();
        // `https_only` re-enforces the scheme on the post-redirect URL too, not
        // just the initial request the `get()` guard checks. Off for the test
        // fetcher so it can reach a local `http` mock.
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .redirect(redirect_policy())
            .https_only(require_https)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            max_attempts: MAX_ATTEMPTS,
            backoff,
            require_https,
        })
    }

    /// GET `url`, retrying transient/5xx failures up to the attempt cap; a 404
    /// returns immediately and a production fetcher refuses non-https up front.
    ///
    /// # Errors
    ///
    /// [`FetchError`] describing the terminal failure.
    pub fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        if self.require_https && !is_https(url) {
            return Err(FetchError::Unreachable(format!(
                "refusing non-https URL (scheme not permitted): {url}"
            )));
        }
        let mut last = String::new();
        for attempt in 0..self.max_attempts {
            if attempt > 0 {
                std::thread::sleep(self.backoff * attempt);
            }
            match self.try_get(url) {
                Ok(bytes) => return Ok(bytes),
                Err(Terminal::NotFound) => return Err(FetchError::NotFound),
                Err(Terminal::TooLarge { limit }) => {
                    return Err(FetchError::TooLarge { limit })
                }
                Err(Terminal::Retryable(detail)) => last = detail,
            }
        }
        Err(FetchError::Unreachable(last))
    }

    fn try_get(&self, url: &str) -> Result<Vec<u8>, Terminal> {
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|error| Terminal::Retryable(error.to_string()))?;
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(Terminal::NotFound);
        }
        if status.is_server_error() {
            return Err(Terminal::Retryable(format!("server error {status}")));
        }
        if !status.is_success() {
            return Err(Terminal::Retryable(format!(
                "unexpected status {status}"
            )));
        }
        response
            .bytes()
            .map(|body| body.to_vec())
            .map_err(|error| Terminal::Retryable(error.to_string()))
    }

    /// GET `url`, copying the body into a sink obtained from `open_sink` and
    /// digesting it in the same pass, so nothing larger than the small-asset
    /// path is ever held in memory.
    ///
    /// `open_sink` is invoked once at the start of each attempt and must yield a
    /// freshly truncated destination: the retry loop depends on an attempt that
    /// failed partway leaving nothing behind.
    ///
    /// # Errors
    ///
    /// [`FetchError`] describing the terminal failure, including
    /// [`FetchError::TooLarge`] when the body exceeds `limits.max_bytes`.
    #[allow(dead_code)]
    pub(super) fn get_streaming<'sink>(
        &self,
        url: &str,
        limits: &StreamLimits,
        open_sink: &mut dyn FnMut() -> std::io::Result<
            Box<dyn StreamSink + 'sink>,
        >,
    ) -> Result<StreamedBody, FetchError> {
        if self.require_https && !is_https(url) {
            return Err(FetchError::Unreachable(format!(
                "refusing non-https URL (scheme not permitted): {url}"
            )));
        }
        let started = Instant::now();
        let mut last = String::new();
        for attempt in 0..self.max_attempts {
            if attempt > 0 {
                std::thread::sleep(self.backoff * attempt);
            }
            if started.elapsed() >= limits.total_deadline {
                break;
            }
            match self.try_get_streaming(url, limits, started, open_sink) {
                Ok(body) => return Ok(body),
                Err(Terminal::NotFound) => return Err(FetchError::NotFound),
                Err(Terminal::TooLarge { limit }) => {
                    return Err(FetchError::TooLarge { limit })
                }
                Err(Terminal::Retryable(detail)) => last = detail,
            }
        }
        Err(FetchError::Unreachable(format!(
            "{last} (gave up after {:?}, the bound on the whole retry loop)",
            limits.total_deadline
        )))
    }

    #[allow(dead_code)]
    fn try_get_streaming<'sink>(
        &self,
        url: &str,
        limits: &StreamLimits,
        started: Instant,
        open_sink: &mut dyn FnMut() -> std::io::Result<
            Box<dyn StreamSink + 'sink>,
        >,
    ) -> Result<StreamedBody, Terminal> {
        let mut response = self
            .client
            .get(url)
            .timeout(limits.idle_timeout)
            .send()
            .map_err(|error| Terminal::Retryable(error.to_string()))?;
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(Terminal::NotFound);
        }
        if status.is_server_error() {
            return Err(Terminal::Retryable(format!("server error {status}")));
        }
        if !status.is_success() {
            return Err(Terminal::Retryable(format!(
                "unexpected status {status}"
            )));
        }

        let mut sink = open_sink()
            .map_err(|error| Terminal::Retryable(error.to_string()))?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; STREAM_CHUNK];
        let mut bytes = 0_u64;
        let attempt_started = Instant::now();
        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| Terminal::Retryable(error.to_string()))?;
            if read == 0 {
                break;
            }
            bytes += read as u64;
            if bytes > limits.max_bytes {
                return Err(Terminal::TooLarge {
                    limit: limits.max_bytes,
                });
            }
            let chunk = &buffer[..read];
            hasher.update(chunk);
            sink.accept(chunk)
                .map_err(|error| Terminal::Retryable(error.to_string()))?;
            if attempt_started.elapsed() >= limits.attempt_timeout
                || started.elapsed() >= limits.total_deadline
            {
                return Err(Terminal::Retryable(format!(
                    "transfer deadline exceeded after {bytes} bytes"
                )));
            }
        }
        Ok(StreamedBody {
            bytes,
            sha256: hasher.finalize().into(),
        })
    }
}

enum Terminal {
    NotFound,
    Retryable(String),
    #[allow(dead_code)]
    TooLarge {
        limit: u64,
    },
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::cell::RefCell;
    use std::io::{BufRead as _, BufReader, Read as _, Write as _};
    use std::net::TcpListener;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use super::{
        is_allowed_redirect_host, is_https, FetchError, Fetcher, StreamLimits,
        StreamSink, StreamedBody,
    };

    /// What the scripted listener does with a connection, chosen per attempt.
    #[derive(Clone, Copy)]
    enum Reply {
        /// Send the whole body.
        Whole,
        /// Promise the whole body but send only this many bytes, then close.
        Truncated(usize),
        /// Send headers, then go idle without sending a body.
        Stall,
    }

    struct Scripted {
        base_url: String,
        attempts: Arc<AtomicUsize>,
    }

    /// Serve `replies[n]` to the nth connection, then `replies`' last entry for
    /// every connection after that.
    fn scripted(body: Vec<u8>, replies: Vec<Reply>) -> Scripted {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&attempts);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let index = counter.fetch_add(1, Ordering::SeqCst);
                let reply = replies
                    .get(index)
                    .or_else(|| replies.last())
                    .copied()
                    .unwrap_or(Reply::Whole);
                let body = body.clone();
                // One thread per connection: a client that keeps a poisoned
                // connection pooled after a broken reply must not stop the
                // retry's connection from being served.
                std::thread::spawn(move || {
                    let mut reader =
                        BufReader::new(stream.try_clone().expect("clone"));
                    let mut line = String::new();
                    loop {
                        line.clear();
                        let Ok(read) = reader.read_line(&mut line) else {
                            break;
                        };
                        if read == 0 || line == "\r\n" || line == "\n" {
                            break;
                        }
                    }
                    let headers = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\
                         Connection: close\r\n\r\n",
                        body.len()
                    );
                    if stream.write_all(headers.as_bytes()).is_err() {
                        return;
                    }
                    let _ = stream.flush();
                    match reply {
                        Reply::Whole => {
                            let _ = stream.write_all(&body);
                            let _ = stream.flush();
                            let mut drain = Vec::new();
                            let _ = reader.get_mut().read_to_end(&mut drain);
                        }
                        Reply::Truncated(prefix) => {
                            let _ = stream.write_all(&body[..prefix]);
                            let _ = stream.flush();
                        }
                        Reply::Stall => {
                            std::thread::sleep(Duration::from_secs(5));
                        }
                    }
                });
            }
        });
        Scripted {
            base_url: format!("http://127.0.0.1:{port}"),
            attempts,
        }
    }

    /// Collects what it is handed, so a test can assert a retry started from an
    /// empty sink rather than appending to the failed attempt's bytes. Opening
    /// a fresh sink truncates, exactly as the file sink does in production.
    type Collecting = Rc<RefCell<Vec<u8>>>;

    struct CollectingSink(Collecting);

    impl StreamSink for CollectingSink {
        fn accept(&mut self, chunk: &[u8]) -> std::io::Result<()> {
            self.0.borrow_mut().extend_from_slice(chunk);
            Ok(())
        }
    }

    fn limits(max_bytes: u64) -> StreamLimits {
        StreamLimits {
            attempt_timeout: Duration::from_secs(5),
            total_deadline: Duration::from_secs(20),
            idle_timeout: Duration::from_secs(5),
            max_bytes,
        }
    }

    fn stream(
        fetcher: &Fetcher,
        url: &str,
        limits: &StreamLimits,
        sink: &Collecting,
    ) -> Result<StreamedBody, FetchError> {
        fetcher.get_streaming(url, limits, &mut || {
            sink.borrow_mut().clear();
            Ok(Box::new(CollectingSink(Rc::clone(sink))))
        })
    }

    fn test_fetcher() -> Fetcher {
        Fetcher::with_backoff(Duration::from_millis(1)).expect("fetcher")
    }

    #[test]
    fn the_archive_bounds_hold_the_loop_below_three_full_attempts() {
        let limits = StreamLimits::for_archive(4096);
        assert_eq!(limits.max_bytes, 4096);
        assert!(
            limits.total_deadline > limits.attempt_timeout,
            "the loop bound must leave room for a retry"
        );
        assert!(
            limits.total_deadline
                < limits.attempt_timeout * super::MAX_ATTEMPTS,
            "an enlarged attempt deadline must not compound across attempts"
        );
        assert!(
            limits.idle_timeout < limits.attempt_timeout,
            "a stall must fail well inside the transfer deadline"
        );
    }

    #[test]
    fn streams_a_body_reporting_its_length_and_digest() {
        let payload = vec![7_u8; 4096];
        let server = scripted(payload.clone(), vec![Reply::Whole]);
        let sink = Collecting::default();
        let body = stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url),
            &limits(1 << 20),
            &sink,
        )
        .expect("streamed");
        assert_eq!(body.bytes, payload.len() as u64);
        assert_eq!(*sink.borrow(), payload);
        let expected: [u8; 32] = {
            use sha2::Digest as _;
            sha2::Sha256::digest(&payload).into()
        };
        assert_eq!(body.sha256, expected);
    }

    #[test]
    fn a_retry_starts_from_a_fresh_sink_rather_than_appending() {
        let payload: Vec<u8> =
            (0..8192_u32).map(|byte| byte.to_le_bytes()[0]).collect();
        let server = scripted(
            payload.clone(),
            vec![Reply::Truncated(1024), Reply::Whole],
        );
        let sink = Collecting::default();
        let body = stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url),
            &limits(1 << 20),
            &sink,
        )
        .expect("streamed after retry");
        assert_eq!(body.bytes, payload.len() as u64);
        assert_eq!(
            *sink.borrow(),
            payload,
            "the retry appended to the partial"
        );
        assert_eq!(server.attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_body_larger_than_the_cap_is_refused_without_retrying() {
        let server = scripted(vec![0_u8; 8192], vec![Reply::Whole]);
        let sink = Collecting::default();
        let result = stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url),
            &limits(4096),
            &sink,
        );
        assert!(
            matches!(result, Err(FetchError::TooLarge { limit: 4096 })),
            "expected a size refusal, got {result:?}"
        );
        assert_eq!(
            server.attempts.load(Ordering::SeqCst),
            1,
            "an oversize body is a defect, not a transient failure"
        );
    }

    #[test]
    fn a_stalled_transfer_fails_within_the_total_deadline() {
        let server = scripted(vec![0_u8; 4096], vec![Reply::Stall]);
        let sink = Collecting::default();
        let started = std::time::Instant::now();
        let result = stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url),
            &StreamLimits {
                attempt_timeout: Duration::from_secs(600),
                total_deadline: Duration::from_secs(900),
                idle_timeout: Duration::from_millis(300),
                max_bytes: 1 << 20,
            },
            &sink,
        );
        let elapsed = started.elapsed();
        assert!(
            matches!(&result, Err(FetchError::Unreachable(detail))
                if detail.contains("900s")),
            "expected the loop's bound named in the failure, got {result:?}"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "a stall waited out the attempt deadline rather than the idle \
             bound: {elapsed:?}"
        );
    }

    #[test]
    fn production_fetcher_refuses_non_https_urls() {
        let Ok(fetcher) = Fetcher::new() else {
            return;
        };
        let result = fetcher.get("http://127.0.0.1:1/asset");
        assert!(
            matches!(
                &result,
                Err(FetchError::Unreachable(detail))
                    if detail.contains("https")
            ),
            "expected an https scheme refusal, got {result:?}"
        );
    }

    #[test]
    fn cdn_suffix_and_origin_are_allowed_redirect_hosts() {
        assert!(is_allowed_redirect_host("github.com"));
        assert!(is_allowed_redirect_host(
            "objects.release.githubusercontent.com"
        ));
    }

    #[test]
    fn lookalike_hosts_are_refused_redirect_targets() {
        assert!(!is_allowed_redirect_host("evil.example.com"));
        assert!(!is_allowed_redirect_host("evil-githubusercontent.com"));
        assert!(!is_allowed_redirect_host(
            "githubusercontent.com.attacker.net"
        ));
    }

    #[test]
    fn https_is_required_by_the_production_pin() {
        assert!(is_https("https://github.com/x"));
        assert!(!is_https("http://github.com/x"));
    }
}
