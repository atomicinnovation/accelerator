//! One endpoint, one method: `POST https://api.linear.app/graphql`.
//!
//! The bounds are the same shape as Jira's and asserted here rather than
//! inherited by inspection, because this crate is independently mergeable: 30s
//! per request, four attempts, the same jittered backoff, no redirects, and a
//! bounded body read.
//!
//! Query documents are hand-rolled strings with `serde_json` variables. Codegen
//! was declined: `cynic` is MPL-2.0, `graphql_client` needs a 1.28 MB committed
//! schema, and neither catches Linear's non-functioning-stub deprecation mode,
//! where a stub matches a committed schema, generates, type-checks and returns
//! nothing.

use std::cell::RefCell;
use std::io::Read as _;
use std::time::Duration;
use std::time::Instant;

use reqwest::blocking::Client;
// Re-exported so a composition root (the `*-cli` seam) can name the endpoint
// type without importing reqwest itself — the provider-isolation tripwire keeps
// reqwest inside the client crates.
pub use reqwest::Url;
use serde_json::json;
use serde_json::Value;
use tracker_support::Jitter;
use tracker_support::RetryPolicy;
use tracker_support::Sleeper;
use tracker_support::TransportConfig;

use crate::auth::Credentials;
use crate::error::ClientError;

pub const ENDPOINT: &str = "https://api.linear.app/graphql";

/// A response that arrived, whatever its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    pub status: u16,
    pub body: String,
}

impl Received {
    /// The body as JSON, or `None` when it is not JSON at all — which is bash
    /// code 16 rather than a transport failure.
    #[must_use]
    pub fn json(&self) -> Option<Value> {
        serde_json::from_str(&self.body).ok()
    }
}

/// Bounds a whole paginated operation.
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    started: Instant,
    budget: Duration,
}

impl Deadline {
    #[must_use]
    pub fn starting_now(budget: Duration) -> Self {
        Self {
            started: Instant::now(),
            budget,
        }
    }

    #[must_use]
    pub fn expired(&self) -> bool {
        self.started.elapsed() >= self.budget
    }
}

pub struct Transport {
    client: Client,
    endpoint: Url,
    credentials: Credentials,
    config: TransportConfig,
    policy: RetryPolicy,
    sleeper: RefCell<Box<dyn Sleeper>>,
    jitter: RefCell<Box<dyn Jitter>>,
}

impl Transport {
    /// Builds the client every request runs through.
    ///
    /// `endpoint` is a parameter so a test can point the client at a cleartext
    /// mock on loopback without any process state — the same seam Jira's
    /// transport takes, and for the same reason.
    ///
    /// # Errors
    ///
    /// [`ClientError::TlsUnavailable`] when the client cannot be built.
    pub fn new(
        endpoint: Url,
        credentials: Credentials,
        config: TransportConfig,
        sleeper: Box<dyn Sleeper>,
        jitter: Box<dyn Jitter>,
    ) -> Result<Self, ClientError> {
        install_crypto_provider();
        let client = Client::builder()
            .timeout(config.timeout)
            // Linear's API never legitimately redirects, and an unfollowed 30x
            // must surface rather than move an authenticated request.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ClientError::TlsUnavailable {
                detail: error.to_string(),
            })?;
        Ok(Self {
            client,
            endpoint,
            credentials,
            config,
            policy: RetryPolicy::default(),
            sleeper: RefCell::new(sleeper),
            jitter: RefCell::new(jitter),
        })
    }

    /// The production endpoint.
    ///
    /// # Errors
    ///
    /// [`ClientError::TlsUnavailable`] when the client cannot be built.
    pub fn to_linear(
        credentials: Credentials,
        config: TransportConfig,
        sleeper: Box<dyn Sleeper>,
        jitter: Box<dyn Jitter>,
    ) -> Result<Self, ClientError> {
        let endpoint = Url::parse(ENDPOINT).map_err(|error| {
            ClientError::TlsUnavailable {
                detail: error.to_string(),
            }
        })?;
        Self::new(endpoint, credentials, config, sleeper, jitter)
    }

    #[must_use]
    pub const fn config(&self) -> &TransportConfig {
        &self.config
    }

    #[must_use]
    pub const fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    #[must_use]
    pub fn deadline(&self) -> Deadline {
        Deadline::starting_now(self.config.deadline)
    }

    /// Sends one GraphQL request, retrying a 5xx and a 400 `RATELIMITED`.
    ///
    /// A 200 carrying `errors[]` is **never** retried: re-issuing a
    /// non-idempotent mutation because its response reported an error is how a
    /// duplicate gets created.
    ///
    /// A transport failure resolves on the first attempt with no retry.
    ///
    /// # Errors
    ///
    /// [`ClientError::Transport`] for a connect, DNS or timeout failure, and
    /// [`ClientError::OversizedResponse`] for a body beyond the bound.
    pub fn send(
        &self,
        document: &str,
        variables: &Value,
    ) -> Result<Received, ClientError> {
        let payload = serde_json::to_string(
            &json!({"query": document, "variables": variables}),
        )
        .map_err(|error| ClientError::Transport {
            detail: format!("the request body could not be built: {error}"),
        })?;

        for attempt in 1..=self.policy.max_attempts {
            let response = self
                .client
                .post(self.endpoint.clone())
                .header("Content-Type", "application/json")
                .header("Authorization", self.credentials.token.expose())
                .body(payload.clone())
                .send()
                .map_err(|error| ClientError::Transport {
                    detail: connect_detail(&error),
                })?;
            let status = response.status().as_u16();
            let retry_after = retry_after(&response);
            let body = self.read_bounded(response)?;
            tracing::debug!(status, attempt, "linear request attempt");

            if !Self::retryable(status, &body) {
                return Ok(Received { status, body });
            }

            let delay = self.policy.delay_for(
                attempt,
                retry_after,
                &mut **self.jitter.borrow_mut(),
            );
            let Some(delay) = delay else {
                tracing::warn!(
                    status,
                    attempts = attempt,
                    "linear request retries exhausted"
                );
                return Ok(Received { status, body });
            };
            tracing::debug!(
                status,
                attempt,
                backoff_secs = delay.as_secs(),
                "linear request backing off"
            );
            self.sleeper.borrow_mut().sleep(delay);
        }

        Err(ClientError::Transport {
            detail: "the retry loop ended without a response".to_owned(),
        })
    }

    /// A 5xx retries, and so does a 400 whose body classifies as
    /// `RATELIMITED`. Nothing else does — least of all a 200.
    fn retryable(status: u16, body: &str) -> bool {
        if matches!(status, 500..600) {
            return true;
        }
        if status != 400 {
            return false;
        }
        serde_json::from_str::<Value>(body).is_ok_and(|parsed| {
            crate::classify::classify_errors(&parsed)
                == crate::classify::GraphQlError::RateLimited
        })
    }

    fn read_bounded(
        &self,
        response: reqwest::blocking::Response,
    ) -> Result<String, ClientError> {
        let limit = self.config.max_response_bytes;
        let mut buffer = Vec::new();
        let mut bounded = response.take(limit as u64 + 1);
        bounded.read_to_end(&mut buffer).map_err(|error| {
            ClientError::Transport {
                detail: body_read_detail(&error),
            }
        })?;
        if buffer.len() > limit {
            return Err(ClientError::OversizedResponse { limit });
        }
        Ok(String::from_utf8_lossy(&buffer).into_owned())
    }
}

/// `rustls-tls-webpki-roots-no-provider` installs no crypto provider, and the
/// offline harness is cleartext, so nothing but this call stands between an
/// https request and a handshake failure. Omitting it is likelier here than in
/// Jira's transport, this being a copy of it.
fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn retry_after(response: &reqwest::blocking::Response) -> Option<Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

fn body_read_detail(error: &std::io::Error) -> String {
    if error.kind() == std::io::ErrorKind::TimedOut {
        return "the request timed out".to_owned();
    }
    format!(
        "the response body could not be read — a stalled, truncated or \
         dropped body: {error}"
    )
}

fn connect_detail(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "the request timed out".to_owned();
    }
    if error.is_connect() {
        return format!(
            "could not connect — check the proxy, and whether the certificate \
             chain is publicly trusted: {error}"
        );
    }
    error.to_string()
}
