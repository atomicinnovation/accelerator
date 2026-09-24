//! The blocking HTTP transport every source's requests run through.
//!
//! Everything a request carries comes from its [`UpstreamRequest`], so the
//! transport holds no per-source knowledge. Error text never reaches a
//! caller: a failure is reported only as its [`Response`] outcome, so no URL,
//! and no key inside one, can leak through it.

use std::fmt;
use std::io::Read as _;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use reqwest::redirect;
use reqwest::Url;
use research::classify::Received;
use research::classify::Response;
use research::fetch::Transport;
use research::request::UpstreamRequest;

const MAX_REDIRECTS: usize = 3;
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

/// The HTTP client could not be built, so no request can be sent.
#[derive(Debug)]
pub struct TransportUnavailable;

impl fmt::Display for TransportUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "E_RESEARCH_TRANSPORT: the HTTP client could not be initialised",
        )
    }
}

impl std::error::Error for TransportUnavailable {}

pub struct HttpTransport {
    client: Client,
}

impl HttpTransport {
    /// A transport whose every request, body included, finishes within
    /// `per_request`.
    ///
    /// No connection outlives its request, so nothing reaches a paced source
    /// between the gate passes that admit each request.
    ///
    /// # Errors
    ///
    /// [`TransportUnavailable`] when the TLS-backed client cannot be built.
    pub fn new(per_request: Duration) -> Result<Self, TransportUnavailable> {
        install_crypto_provider();
        let client = Client::builder()
            .timeout(per_request)
            .pool_max_idle_per_host(0)
            .redirect(redirect::Policy::custom(follow_within_origin))
            .build()
            .map_err(|_| TransportUnavailable)?;
        Ok(Self { client })
    }
}

impl Transport for HttpTransport {
    fn send(&self, request: &UpstreamRequest) -> Response {
        let mut outgoing = self.client.get(request.url());
        for (name, value) in request.headers() {
            outgoing = outgoing.header(*name, value);
        }
        if let Some(token) = request.bearer() {
            outgoing = outgoing.bearer_auth(token);
        }
        match outgoing.send() {
            Ok(response) => receive(response),
            Err(error) => failure(&error),
        }
    }
}

/// Merged works answer with a redirect to the surviving ID. Only a hop that
/// stays on the requested origin is followed, so a key is never offered to a
/// host it was not issued for; any other redirect is returned as received.
fn follow_within_origin(attempt: redirect::Attempt<'_>) -> redirect::Action {
    let hops = attempt.previous().len();
    let stays_on_origin = attempt
        .previous()
        .first()
        .is_some_and(|original| same_origin(original, attempt.url()));
    if hops <= MAX_REDIRECTS && stays_on_origin {
        attempt.follow()
    } else {
        attempt.stop()
    }
}

fn same_origin(original: &Url, next: &Url) -> bool {
    original.scheme() == next.scheme()
        && original.host_str() == next.host_str()
        && original.port_or_known_default() == next.port_or_known_default()
}

fn receive(response: reqwest::blocking::Response) -> Response {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let mut body = Vec::new();
    let read = response
        .take(MAX_BODY_BYTES as u64 + 1)
        .read_to_end(&mut body);
    if let Err(error) = read {
        return if is_timeout(&error) {
            Response::TimedOut
        } else {
            Response::ConnectionFailed
        };
    }
    if body.len() > MAX_BODY_BYTES {
        body.clear();
    }
    Response::Received(Received {
        status,
        retry_after: seconds(&headers, "retry-after").map(Duration::from_secs),
        rate_limit_remaining: seconds(&headers, "x-ratelimit-remaining"),
        rate_limit_credits_required: seconds(
            &headers,
            "x-ratelimit-credits-required",
        ),
        body,
    })
}

fn seconds(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers.get(name)?.to_str().ok()?.trim().parse().ok()
}

fn failure(error: &reqwest::Error) -> Response {
    if error.is_timeout() {
        Response::TimedOut
    } else {
        Response::ConnectionFailed
    }
}

/// A body read that outlives the budget surfaces as an `io::Error` wrapping
/// reqwest's own error, which alone knows it was a timeout.
fn is_timeout(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::TimedOut
        || error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<reqwest::Error>())
            .is_some_and(reqwest::Error::is_timeout)
}

/// `rustls-tls-webpki-roots-no-provider` installs no crypto provider, so
/// nothing but this call stands between an `https` request and a handshake
/// failure.
fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
