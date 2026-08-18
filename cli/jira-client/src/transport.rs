//! The bounded HTTP transport, transcribed from `jira-request.sh:298-442`.
//!
//! Every bound is a constructor parameter, never a post-construction setter: a
//! `reqwest` client's timeout is fixed when the client is built, so a
//! `with_timeout` applied afterwards would be a silent no-op and the timeout
//! suites would quietly wait on the 30s default.
//!
//! `Sleeper` and `Jitter` reach this seam for the same reason — without them
//! the mock-backed retry suites would execute real exponential backoff.

use std::cell::RefCell;
use std::io::Read as _;
use std::time::Duration;
use std::time::Instant;

use reqwest::blocking::Client;
use reqwest::Method;
use reqwest::Url;
use tracker_support::Jitter;
use tracker_support::RetryPolicy;
use tracker_support::Sleeper;
use tracker_support::TransportConfig;

use crate::auth::Credentials;
use crate::error::ClientError;
use crate::path;

/// A response that arrived, whatever its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    pub status: u16,
    pub body: String,
}

/// Bounds a whole paginated operation, because the page cap bounds result size
/// rather than time.
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
    credentials: Credentials,
    config: TransportConfig,
    policy: RetryPolicy,
    sleeper: RefCell<Box<dyn Sleeper>>,
    jitter: RefCell<Box<dyn Jitter>>,
}

impl Transport {
    /// Builds the client every request runs through.
    ///
    /// # Errors
    ///
    /// [`ClientError::TlsUnavailable`] when the client cannot be built.
    pub fn new(
        credentials: Credentials,
        config: TransportConfig,
        sleeper: Box<dyn Sleeper>,
        jitter: Box<dyn Jitter>,
    ) -> Result<Self, ClientError> {
        install_crypto_provider();
        let client = Client::builder()
            .timeout(config.timeout)
            // Jira's API never legitimately redirects, and following one would
            // defeat the site validator: it inspects the initial host only.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ClientError::TlsUnavailable {
                detail: error.to_string(),
            })?;
        Ok(Self {
            client,
            credentials,
            config,
            policy: RetryPolicy::default(),
            sleeper: RefCell::new(sleeper),
            jitter: RefCell::new(jitter),
        })
    }

    #[must_use]
    pub const fn config(&self) -> &TransportConfig {
        &self.config
    }

    #[must_use]
    pub fn deadline(&self) -> Deadline {
        Deadline::starting_now(self.config.deadline)
    }

    /// Sends one request, retrying 429 and 5xx up to the policy's attempts.
    ///
    /// A transport failure resolves on the first attempt with no retry, as the
    /// bash exits 21 immediately: folding timeouts into the retry loop would
    /// put the wall clock at four times the timeout plus backoffs.
    ///
    /// # Errors
    ///
    /// [`ClientError::BadPath`] before anything is sent,
    /// [`ClientError::Transport`] for a connect, DNS or timeout failure, and
    /// [`ClientError::OversizedResponse`] for a body beyond the bound.
    pub fn send(
        &self,
        method: &Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&str>,
    ) -> Result<Received, ClientError> {
        path::validate(path)?;
        let url = self.url(path, query)?;

        for attempt in 1..=self.policy.max_attempts {
            let mut request =
                self.client.request(method.clone(), url.clone()).basic_auth(
                    &self.credentials.email,
                    Some(self.credentials.token.expose()),
                );
            if let Some(body) = body {
                request = request
                    .header("Content-Type", "application/json")
                    .body(body.to_owned());
            }

            let response =
                request.send().map_err(|error| ClientError::Transport {
                    detail: connect_detail(&error),
                })?;
            let status = response.status().as_u16();
            let retry_after = retry_after(&response);
            tracing::debug!(
                method = %method,
                path = %path,
                status,
                attempt,
                "jira request attempt"
            );

            if !is_retryable_status(status) {
                return Ok(Received {
                    status,
                    body: self.read_bounded(response)?,
                });
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
                    "jira request retries exhausted"
                );
                return Ok(Received {
                    status,
                    body: self.read_bounded(response)?,
                });
            };
            tracing::debug!(
                status,
                attempt,
                backoff_secs = delay.as_secs(),
                "jira request backing off"
            );
            self.sleeper.borrow_mut().sleep(delay);
        }

        Err(ClientError::Transport {
            detail: "the retry loop ended without a response".to_owned(),
        })
    }

    fn url(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Url, ClientError> {
        let mut url = self.credentials.base.join(path).map_err(|error| {
            ClientError::BadPath {
                path: path.to_owned(),
                reason: error.to_string(),
            }
        })?;
        if !query.is_empty() {
            let mut pairs = url.query_pairs_mut();
            for (name, value) in query {
                pairs.append_pair(name, value);
            }
        }
        Ok(url)
    }

    /// Reads a body through the response bound, so no endpoint can exhaust
    /// memory with an oversized response.
    fn read_bounded(
        &self,
        response: reqwest::blocking::Response,
    ) -> Result<String, ClientError> {
        let limit = self.config.max_response_bytes;
        let mut buffer = Vec::new();
        let mut bounded = response.take(limit as u64 + 1);
        bounded.read_to_end(&mut buffer).map_err(|error| {
            ClientError::Transport {
                detail: format!("the response body could not be read: {error}"),
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
/// https request and a handshake failure.
fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

const fn is_retryable_status(status: u16) -> bool {
    status == 429 || matches!(status, 500..600)
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

fn connect_detail(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "the request timed out".to_owned();
    }
    if error.is_connect() {
        // Named distinctly because webpki-roots replaces curl's system trust
        // store: a TLS-intercepting proxy or a private CA fails here, and the
        // cause has to be readable from the message.
        return format!(
            "could not connect — check the host, the proxy, and whether the \
             certificate chain is publicly trusted: {error}"
        );
    }
    error.to_string()
}
