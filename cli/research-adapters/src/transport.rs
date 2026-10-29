//! The blocking HTTP transport every source's requests run through, each
//! bounded as a whole, body included, by its per-request budget.
//!
//! Everything a request carries comes from its [`UpstreamRequest`], so the
//! transport holds no per-source knowledge. Error text never reaches a
//! caller: a failure is reported only as its [`Response`] outcome, so no URL,
//! and no key inside one, can leak through it.

use std::fmt;
use std::time::Duration;

use reqwest::header::HeaderMap;
use reqwest::redirect;
use reqwest::Client;
use reqwest::Url;
use research::sources::classify::Received;
use research::sources::classify::Response;
use research::sources::fetch::Transport;
use research::sources::request::UpstreamRequest;
use tokio::runtime::Runtime;

use crate::bounded::current_thread_runtime;
use crate::bounded::read_capped;
use crate::bounded::within;

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

/// The client stays on the one runtime it was first driven by, because the
/// resolver it builds lazily is bound to that runtime.
pub struct HttpTransport {
    client: Client,
    runtime: Runtime,
    per_request: Duration,
}

impl HttpTransport {
    /// A transport whose every request, body included, finishes within
    /// `per_request`.
    ///
    /// No connection outlives its request, so nothing reaches a paced source
    /// outside the attempt its serving turn admits.
    ///
    /// # Errors
    ///
    /// [`TransportUnavailable`] when the TLS-backed client, or the runtime
    /// that drives it, cannot be built.
    pub fn new(per_request: Duration) -> Result<Self, TransportUnavailable> {
        install_crypto_provider();
        let client = Client::builder()
            .pool_max_idle_per_host(0)
            .redirect(redirect::Policy::custom(follow_within_origin))
            .build()
            .map_err(|_| TransportUnavailable)?;
        let runtime =
            current_thread_runtime().map_err(|_| TransportUnavailable)?;
        Ok(Self {
            client,
            runtime,
            per_request,
        })
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
        let exchange = async {
            match outgoing.send().await {
                Ok(response) => receive(response).await,
                Err(_) => Response::ConnectionFailed,
            }
        };
        within(
            &self.runtime,
            self.per_request,
            exchange,
            Response::TimedOut,
        )
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

/// A body past the cap is withheld whole: a cut JSON or XML document would
/// not parse.
async fn receive(response: reqwest::Response) -> Response {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let Ok(capped) = read_capped(response, MAX_BODY_BYTES).await else {
        return Response::ConnectionFailed;
    };
    let body = if capped.overflowed {
        Vec::new()
    } else {
        capped.head
    };
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

/// `rustls-tls-webpki-roots-no-provider` installs no crypto provider, so
/// nothing but this call stands between an `https` request and a handshake
/// failure.
pub(crate) fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
