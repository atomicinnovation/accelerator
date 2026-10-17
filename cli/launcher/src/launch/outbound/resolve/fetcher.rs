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
// The help path never needs the manifest, so it gives up fast and fails open to
// the built-in listing rather than inheriting dispatch's retry-heavy budget.
const HELP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const HELP_TOTAL_TIMEOUT: Duration = Duration::from_secs(5);
// Whole-request deadline per attempt for the buffered small-asset path, sized
// for a multi-MB release binary over a slow link. On the streaming path it
// bounds one read rather than the attempt, so `StreamLimits` carries the
// attempt and whole-loop bounds instead.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(300);
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
/// a transient failure, so it is not retried either. Nor is a sink that cannot
/// be opened: the fault is local, so retrying the network cannot fix it.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchError {
    NotFound,
    Unreachable(String),
    TooLarge { limit: u64 },
    Sink(String),
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

/// A body streamed whole into a truncating sink, digested in the one pass.
#[derive(Debug)]
pub(super) struct Whole {
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SinkMode {
    Truncate,
    Append { from: u64 },
}

/// What a request ranged from an offset yielded.
#[derive(Debug)]
pub(super) enum Resumed {
    /// The server ignored the range and sent the whole body.
    Whole(Whole),
    Appended,
    AlreadyComplete,
    /// The server refused the range, so the bytes on disk cannot be trusted
    /// as a prefix of the resource.
    RangeRejected,
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

/// The per-instance bounds a fetcher runs under: the client's connect and
/// whole-request timeouts, the attempt cap, and the inter-attempt backoff.
#[derive(Clone, Copy)]
struct Limits {
    connect: Duration,
    total: Duration,
    max_attempts: u32,
    backoff: Duration,
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
        Self::build(
            Limits {
                connect: CONNECT_TIMEOUT,
                total: TOTAL_TIMEOUT,
                max_attempts: MAX_ATTEMPTS,
                backoff: Duration::from_millis(250),
            },
            true,
        )
    }

    /// Build the help-scoped fetcher: a single short attempt (https pinned) so a
    /// slow or unreachable release host fails open to the built-in listing
    /// within the help connect timeout rather than dispatch's retry budget.
    ///
    /// # Errors
    ///
    /// If the underlying client cannot be constructed.
    pub fn for_help() -> Result<Self, String> {
        Self::build(
            Limits {
                connect: HELP_CONNECT_TIMEOUT,
                total: HELP_TOTAL_TIMEOUT,
                max_attempts: 1,
                backoff: Duration::ZERO,
            },
            true,
        )
    }

    /// Build a test fetcher with a caller-chosen backoff, permitting `http` for
    /// a local mock server.
    ///
    /// # Errors
    ///
    /// If the underlying client cannot be constructed.
    pub fn with_backoff(backoff: Duration) -> Result<Self, String> {
        Self::build(
            Limits {
                connect: CONNECT_TIMEOUT,
                total: TOTAL_TIMEOUT,
                max_attempts: MAX_ATTEMPTS,
                backoff,
            },
            false,
        )
    }

    fn build(limits: Limits, require_https: bool) -> Result<Self, String> {
        // The ring provider must be installed before building a TLS client
        // (idempotent), keeping the Fetcher self-sufficient in tests.
        let _ = rustls::crypto::ring::default_provider().install_default();
        // `https_only` re-enforces the scheme on the post-redirect URL too, not
        // just the initial request the `get()` guard checks. Off for the test
        // fetcher so it can reach a local `http` mock.
        let client = Client::builder()
            .connect_timeout(limits.connect)
            .timeout(limits.total)
            .redirect(redirect_policy())
            .https_only(require_https)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            max_attempts: limits.max_attempts,
            backoff: limits.backoff,
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
                Err(terminal) => last = terminal.retryable_detail()?,
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

    /// GET `url` whole, copying the body into a sink obtained from
    /// `open_sink` and digesting it in the same pass, so nothing larger than
    /// the small-asset path is ever held in memory.
    ///
    /// `open_sink` is invoked once at the start of each attempt and must yield
    /// a freshly truncated destination: the retry loop depends on an attempt
    /// that failed partway leaving nothing behind.
    ///
    /// # Errors
    ///
    /// [`FetchError`] describing the terminal failure, including
    /// [`FetchError::TooLarge`] when the body exceeds `limits.max_bytes`.
    pub(super) fn get_whole<'sink>(
        &self,
        url: &str,
        limits: &StreamLimits,
        open_sink: &mut dyn FnMut() -> std::io::Result<
            Box<dyn StreamSink + 'sink>,
        >,
    ) -> Result<Whole, FetchError> {
        self.streaming_attempts(url, limits, |started| {
            let response = self.send_streaming(url, limits, None)?;
            if !response.status().is_success() {
                return Err(unexpected_status(&response));
            }
            let sink = open_sink()
                .map_err(|error| Terminal::Sink(error.to_string()))?;
            copy_body(response, sink, limits, started)
                .map(|sha256| Whole { sha256 })
        })
    }

    /// GET `url` ranged from byte `from`, appending the suffix to a sink
    /// opened with [`SinkMode::Append`], or truncating when the server ignores
    /// the range and answers with the whole body.
    ///
    /// # Errors
    ///
    /// [`FetchError`] describing the terminal failure, including
    /// [`FetchError::TooLarge`] when the body exceeds `limits.max_bytes`.
    pub(super) fn resume<'sink>(
        &self,
        url: &str,
        limits: &StreamLimits,
        from: u64,
        open_sink: &mut dyn FnMut(
            SinkMode,
        ) -> std::io::Result<
            Box<dyn StreamSink + 'sink>,
        >,
    ) -> Result<Resumed, FetchError> {
        self.streaming_attempts(url, limits, |started| {
            let response = self.send_streaming(url, limits, Some(from))?;
            let status = response.status();
            if status.as_u16() == 416 {
                return Ok(unsatisfiable(&response, from));
            }
            if status.as_u16() == 206
                && content_range(&response).and_then(satisfied_start)
                    != Some(from)
            {
                return Ok(Resumed::RangeRejected);
            }
            if !status.is_success() {
                return Err(unexpected_status(&response));
            }
            let ranged = status.as_u16() == 206;
            let mode = if ranged {
                SinkMode::Append { from }
            } else {
                SinkMode::Truncate
            };
            let sink = open_sink(mode)
                .map_err(|error| Terminal::Sink(error.to_string()))?;
            let sha256 = copy_body(response, sink, limits, started)?;
            Ok(if ranged {
                Resumed::Appended
            } else {
                Resumed::Whole(Whole { sha256 })
            })
        })
    }

    fn streaming_attempts<T>(
        &self,
        url: &str,
        limits: &StreamLimits,
        mut attempt: impl FnMut(Instant) -> Result<T, Terminal>,
    ) -> Result<T, FetchError> {
        if self.require_https && !is_https(url) {
            return Err(FetchError::Unreachable(format!(
                "refusing non-https URL (scheme not permitted): {url}"
            )));
        }
        let started = Instant::now();
        let mut last = String::new();
        for attempt_number in 0..self.max_attempts {
            if attempt_number > 0 {
                std::thread::sleep(self.backoff * attempt_number);
            }
            if started.elapsed() >= limits.total_deadline {
                break;
            }
            match attempt(started) {
                Ok(outcome) => return Ok(outcome),
                Err(terminal) => last = terminal.retryable_detail()?,
            }
        }
        Err(FetchError::Unreachable(format!(
            "{last} (gave up after {:?}, the bound on the whole retry loop)",
            limits.total_deadline
        )))
    }

    fn send_streaming(
        &self,
        url: &str,
        limits: &StreamLimits,
        range_from: Option<u64>,
    ) -> Result<reqwest::blocking::Response, Terminal> {
        let mut request = self.client.get(url).timeout(limits.idle_timeout);
        if let Some(offset) = range_from {
            request = request.header("Range", format!("bytes={offset}-"));
        }
        let response = request
            .send()
            .map_err(|error| Terminal::Retryable(error.to_string()))?;
        let status = response.status();
        if status.as_u16() == 404 {
            return Err(Terminal::NotFound);
        }
        if status.is_server_error() {
            return Err(Terminal::Retryable(format!("server error {status}")));
        }
        Ok(response)
    }
}

/// Every 416 to a ranged request is final: a total equal to the range start
/// means the bytes on disk are the whole resource, and anything else, a
/// missing header included, means they are not a prefix of it.
fn unsatisfiable(response: &reqwest::blocking::Response, from: u64) -> Resumed {
    if content_range(response).and_then(unsatisfied_length) == Some(from) {
        Resumed::AlreadyComplete
    } else {
        Resumed::RangeRejected
    }
}

fn content_range(response: &reqwest::blocking::Response) -> Option<&str> {
    response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()
}

fn unsatisfied_length(content_range: &str) -> Option<u64> {
    content_range.strip_prefix("bytes */")?.trim().parse().ok()
}

fn satisfied_start(content_range: &str) -> Option<u64> {
    let (start, _) = content_range.strip_prefix("bytes ")?.split_once('-')?;
    start.trim().parse().ok()
}

fn unexpected_status(response: &reqwest::blocking::Response) -> Terminal {
    Terminal::Retryable(format!("unexpected status {}", response.status()))
}

fn copy_body(
    mut response: reqwest::blocking::Response,
    mut sink: Box<dyn StreamSink + '_>,
    limits: &StreamLimits,
    started: Instant,
) -> Result<[u8; 32], Terminal> {
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
    Ok(hasher.finalize().into())
}

enum Terminal {
    NotFound,
    Retryable(String),
    TooLarge { limit: u64 },
    Sink(String),
}

impl Terminal {
    fn retryable_detail(self) -> Result<String, FetchError> {
        match self {
            Self::Retryable(detail) => Ok(detail),
            Self::NotFound => Err(FetchError::NotFound),
            Self::TooLarge { limit } => Err(FetchError::TooLarge { limit }),
            Self::Sink(detail) => Err(FetchError::Sink(detail)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use http_test_support::{MockHTTPServer, RequestKey, Route};

    use super::{
        is_allowed_redirect_host, is_https, FetchError, Fetcher, Resumed,
        SinkMode, StreamLimits, StreamSink, Whole,
    };

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
    ) -> Result<Whole, FetchError> {
        fetcher.get_whole(url, limits, &mut || {
            sink.borrow_mut().clear();
            Ok(Box::new(CollectingSink(Rc::clone(sink))))
        })
    }

    #[derive(Default)]
    struct Resumption {
        received: Collecting,
        modes: Rc<RefCell<Vec<SinkMode>>>,
    }

    fn resume(
        server: &MockHTTPServer,
        from: u64,
        resumption: &Resumption,
    ) -> Result<Resumed, FetchError> {
        test_fetcher().resume(
            &format!("{}/asset", server.base_url()),
            &limits(1 << 20),
            from,
            &mut |mode| {
                resumption.modes.borrow_mut().push(mode);
                Ok(Box::new(CollectingSink(Rc::clone(&resumption.received))))
            },
        )
    }

    fn answering(status: u16, content_range: Option<String>) -> MockHTTPServer {
        let server = MockHTTPServer::start();
        server.route(
            RequestKey::get("/asset"),
            Route::Headers {
                status,
                headers: content_range
                    .map(|range| ("Content-Range".to_owned(), range))
                    .into_iter()
                    .collect(),
                body: if status == 206 {
                    "suffix".to_owned()
                } else {
                    String::new()
                },
            },
        );
        server
    }

    const RESUME_FROM: u64 = 1000;

    #[test]
    fn a_range_starting_at_the_end_is_already_complete() {
        let server = answering(416, Some(format!("bytes */{RESUME_FROM}")));
        let resumption = Resumption::default();

        let outcome = resume(&server, RESUME_FROM, &resumption);

        assert!(
            matches!(outcome, Ok(Resumed::AlreadyComplete)),
            "got {outcome:?}"
        );
        assert!(resumption.modes.borrow().is_empty(), "a sink was opened");
    }

    fn assert_range_rejected(server: &MockHTTPServer) {
        let resumption = Resumption::default();

        let outcome = resume(server, RESUME_FROM, &resumption);

        assert!(
            matches!(outcome, Ok(Resumed::RangeRejected)),
            "got {outcome:?}"
        );
        assert!(resumption.modes.borrow().is_empty(), "a sink was opened");
        assert_eq!(server.hits(&RequestKey::get("/asset")), 1);
    }

    #[test]
    fn a_416_for_a_shorter_resource_rejects_the_range() {
        assert_range_rejected(&answering(
            416,
            Some(format!("bytes */{}", RESUME_FROM - 10)),
        ));
    }

    #[test]
    fn a_416_for_a_longer_resource_rejects_the_range() {
        assert_range_rejected(&answering(
            416,
            Some(format!("bytes */{}", RESUME_FROM + 10)),
        ));
    }

    #[test]
    fn a_416_without_a_content_range_rejects_the_range() {
        assert_range_rejected(&answering(416, None));
    }

    #[test]
    fn a_206_from_a_different_offset_rejects_the_range() {
        let start = RESUME_FROM + 1;
        assert_range_rejected(&answering(
            206,
            Some(format!("bytes {start}-{}/{}", start + 5, start + 6)),
        ));
    }

    #[test]
    fn a_server_ignoring_the_range_answers_whole() {
        let payload = vec![3_u8; 2048];
        let server = MockHTTPServer::start();
        server.route(
            RequestKey::get("/asset"),
            Route::Bytes {
                status: 200,
                body: payload.clone(),
            },
        );
        let resumption = Resumption::default();

        let outcome = resume(&server, RESUME_FROM, &resumption);

        assert!(matches!(outcome, Ok(Resumed::Whole(_))), "got {outcome:?}");
        assert_eq!(*resumption.modes.borrow(), [SinkMode::Truncate]);
        assert_eq!(*resumption.received.borrow(), payload);
    }

    #[test]
    fn a_sink_that_cannot_be_opened_is_not_retried() {
        let server = MockHTTPServer::start();
        let key = RequestKey::get("/asset");
        server.route(
            key.clone(),
            Route::Bytes {
                status: 200,
                body: vec![0_u8; 64],
            },
        );

        let outcome = test_fetcher().get_whole(
            &format!("{}/asset", server.base_url()),
            &limits(1 << 20),
            &mut || Err(std::io::Error::other("the sink is unwritable")),
        );

        assert!(
            matches!(&outcome, Err(FetchError::Sink(detail))
                if detail.contains("unwritable")),
            "got {outcome:?}"
        );
        assert_eq!(server.hits(&key), 1);
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
        let server = MockHTTPServer::start();
        server.route(
            RequestKey::get("/asset"),
            Route::Bytes {
                status: 200,
                body: payload.clone(),
            },
        );
        let sink = Collecting::default();
        let body = stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url()),
            &limits(1 << 20),
            &sink,
        )
        .expect("streamed");
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
        let server = MockHTTPServer::start();
        let key = RequestKey::get("/asset");
        server.route(
            key.clone(),
            Route::Sequence(vec![
                Route::Truncated {
                    body: payload.clone(),
                    sent: 1024,
                },
                Route::Bytes {
                    status: 200,
                    body: payload.clone(),
                },
            ]),
        );
        let sink = Collecting::default();
        stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url()),
            &limits(1 << 20),
            &sink,
        )
        .expect("streamed after retry");
        assert_eq!(
            *sink.borrow(),
            payload,
            "the retry restarts from a cleared sink"
        );
        assert_eq!(server.hits(&key), 2);
    }

    #[test]
    fn a_body_larger_than_the_cap_is_refused_without_retrying() {
        let server = MockHTTPServer::start();
        let key = RequestKey::get("/asset");
        server.route(
            key.clone(),
            Route::Bytes {
                status: 200,
                body: vec![0_u8; 8192],
            },
        );
        let sink = Collecting::default();
        let result = stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url()),
            &limits(4096),
            &sink,
        );
        assert!(
            matches!(result, Err(FetchError::TooLarge { limit: 4096 })),
            "expected a size refusal, got {result:?}"
        );
        assert_eq!(
            server.hits(&key),
            1,
            "an oversize body is a defect, not a transient failure"
        );
    }

    #[test]
    fn a_stalled_transfer_fails_within_the_total_deadline() {
        let server = MockHTTPServer::start();
        server.route(
            RequestKey::get("/asset"),
            Route::Stall(Duration::from_secs(5)),
        );
        let sink = Collecting::default();
        let started = std::time::Instant::now();
        let result = stream(
            &test_fetcher(),
            &format!("{}/asset", server.base_url()),
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

    #[test]
    fn the_help_fetcher_gives_up_within_its_connect_bound() {
        let Ok(fetcher) = Fetcher::for_help() else {
            return;
        };
        // A non-routable address that drops the SYN, so the connect stalls until
        // its timeout rather than being refused immediately.
        let started = std::time::Instant::now();
        let result = fetcher.get("https://10.255.255.1/manifest.json");
        let elapsed = started.elapsed();
        assert!(result.is_err(), "expected a fetch failure, got {result:?}");
        assert!(
            elapsed < Duration::from_secs(8),
            "the help fetch inherited a longer budget than one short attempt: \
             {elapsed:?}"
        );
    }
}
