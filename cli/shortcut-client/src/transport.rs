//! The bounded HTTP transport over Shortcut's REST API v4.
//!
//! The bounds match Jira's and Linear's: 30s per request, four attempts, the
//! same jittered backoff, no redirects, and a bounded body read. Resending
//! follows Jira's rule exactly — only a status earns a resend, never a
//! timeout or a dropped connection, so a request is never sent again after an
//! answer that could not be read.

use std::cell::RefCell;
use std::io::Read as _;
use std::time::Duration;
use std::time::Instant;

use reqwest::blocking::Client;
use reqwest::header::HeaderValue;
use reqwest::Method;
use reqwest::Url;
use serde_json::Value;
use tracker_support::Jitter;
use tracker_support::RetryPolicy;
use tracker_support::Sleeper;
use tracker_support::TransportConfig;

use crate::auth::Credentials;
use crate::error::ClientError;
use crate::paths::Paths;

/// A response that arrived, whatever its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Received {
    pub status: u16,
    pub body: String,
}

impl Received {
    /// The body as JSON, or `None` when it is not JSON at all.
    #[must_use]
    pub fn json(&self) -> Option<Value> {
        serde_json::from_str(&self.body).ok()
    }

    #[must_use]
    pub const fn succeeded(&self) -> bool {
        matches!(self.status, 200..300)
    }
}

/// What one request came to, across every attempt the policy allowed.
///
/// `may_have_applied` covers the whole retry loop, not the last attempt alone:
/// a write answered 5xx and then 429 may still have applied on the first
/// attempt, whatever the last one says.
#[derive(Debug)]
pub struct Delivery {
    pub result: Result<Received, ClientError>,
    pub may_have_applied: bool,
}

/// Bounds a whole operation, because the page cap bounds result size rather
/// than time.
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

/// Which answers a request may be sent again after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resend {
    WhileRetryable,
    /// Only a 429 proves the request was not applied.
    OnlyWhenThrottled,
}

impl Resend {
    const fn allows(self, status: u16) -> bool {
        match self {
            Self::WhileRetryable => status == 429 || matches!(status, 500..600),
            Self::OnlyWhenThrottled => status == 429,
        }
    }
}

pub struct Transport {
    client: Client,
    authorization: HeaderValue,
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
    /// [`ClientError::TlsUnavailable`] when the client cannot be built, and
    /// [`ClientError::MalformedToken`] for a token no header can carry.
    pub fn new(
        credentials: &Credentials,
        config: TransportConfig,
        sleeper: Box<dyn Sleeper>,
        jitter: Box<dyn Jitter>,
    ) -> Result<Self, ClientError> {
        install_crypto_provider();
        let mut authorization = HeaderValue::from_str(&format!(
            "Bearer {}",
            credentials.token.expose()
        ))
        .map_err(|_| ClientError::MalformedToken {
            found: "a character a header cannot carry".to_owned(),
        })?;
        authorization.set_sensitive(true);
        let client = Client::builder()
            .tls_certs_only(bundled_trust_roots())
            .timeout(config.timeout)
            // Shortcut's API never legitimately redirects, and following one
            // would carry the bearer token past the origin checks.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ClientError::TlsUnavailable {
                detail: error.to_string(),
            })?;
        Ok(Self {
            client,
            authorization,
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

    #[must_use]
    pub fn get(&self, url: &Url, deadline: &Deadline) -> Delivery {
        self.execute(&Method::GET, url, None, Resend::WhileRetryable, deadline)
    }

    #[must_use]
    pub fn send(
        &self,
        method: &Method,
        url: &Url,
        body: &Value,
        resend: Resend,
        deadline: &Deadline,
    ) -> Delivery {
        match serde_json::to_string(body) {
            Ok(payload) => {
                self.execute(method, url, Some(&payload), resend, deadline)
            }
            Err(error) => Delivery {
                result: Err(ClientError::RequestInvalid {
                    detail: format!(
                        "the request body could not be built: {error}"
                    ),
                }),
                may_have_applied: false,
            },
        }
    }

    /// Follows a `next_page_url` only when `paths` admits it, so the bearer
    /// token never leaves Shortcut's origin.
    #[must_use]
    pub fn follow(
        &self,
        next: &str,
        paths: &Paths,
        deadline: &Deadline,
    ) -> Delivery {
        let admitted = paths
            .admits_next_page(next)
            .then(|| Url::parse(next).ok())
            .flatten();
        admitted.map_or_else(
            || {
                tracing::warn!(next, "shortcut next_page_url refused");
                Delivery {
                    result: Err(ClientError::ForeignPage {
                        url: next.to_owned(),
                    }),
                    may_have_applied: false,
                }
            },
            |url| self.get(&url, deadline),
        )
    }

    fn execute(
        &self,
        method: &Method,
        url: &Url,
        body: Option<&str>,
        resend: Resend,
        deadline: &Deadline,
    ) -> Delivery {
        let mut may_have_applied = false;
        for attempt in 1..=self.policy.max_attempts {
            let answer = self.attempt(method, url, body, attempt);
            may_have_applied |= answer.may_have_applied();
            let (received, retry_after) = match answer {
                Answer::Arrived {
                    received,
                    retry_after,
                } => (received, retry_after),
                Answer::Failed(error) => {
                    return Delivery {
                        result: Err(error),
                        may_have_applied,
                    };
                }
            };
            if !resend.allows(received.status) || deadline.expired() {
                return delivered(received, may_have_applied);
            }
            let Some(delay) =
                self.backoff(attempt, retry_after, received.status)
            else {
                return delivered(received, may_have_applied);
            };
            self.sleeper.borrow_mut().sleep(delay);
        }
        Delivery {
            result: Err(ClientError::Transport {
                detail: "the retry loop ended without a response".to_owned(),
            }),
            may_have_applied,
        }
    }

    fn attempt(
        &self,
        method: &Method,
        url: &Url,
        body: Option<&str>,
        attempt: usize,
    ) -> Answer {
        let mut request = self
            .client
            .request(method.clone(), url.clone())
            .header(reqwest::header::AUTHORIZATION, self.authorization.clone());
        if let Some(body) = body {
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body.to_owned());
        }
        let response = match request.send() {
            Ok(response) => response,
            Err(error) => return Answer::Failed(send_failure(&error, attempt)),
        };
        let status = response.status().as_u16();
        let retry_after = retry_after(&response);
        tracing::debug!(%method, status, attempt, "shortcut request attempt");
        match self.read_bounded(response) {
            Ok(body) => Answer::Arrived {
                received: Received { status, body },
                retry_after,
            },
            Err(error) => Answer::Failed(error),
        }
    }

    fn backoff(
        &self,
        attempt: usize,
        retry_after: Option<Duration>,
        status: u16,
    ) -> Option<Duration> {
        let delay = self.policy.delay_for(
            attempt,
            retry_after,
            &mut **self.jitter.borrow_mut(),
        );
        if let Some(delay) = delay {
            tracing::debug!(
                status,
                attempt,
                backoff_secs = delay.as_secs(),
                "shortcut request backing off"
            );
        } else {
            tracing::warn!(
                status,
                attempts = attempt,
                "shortcut request retries exhausted"
            );
        }
        delay
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

/// What one attempt came to.
enum Answer {
    Arrived {
        received: Received,
        retry_after: Option<Duration>,
    },
    Failed(ClientError),
}

impl Answer {
    /// A 5xx, or a failure after the request left the client, may have
    /// applied it.
    const fn may_have_applied(&self) -> bool {
        match self {
            Self::Arrived { received, .. } => {
                matches!(received.status, 500..600)
            }
            Self::Failed(error) => {
                matches!(error, ClientError::Transport { .. })
            }
        }
    }
}

/// A 2xx whose body is not JSON may still have applied the request, so it
/// counts towards `may_have_applied`.
fn delivered(received: Received, may_have_applied: bool) -> Delivery {
    let may_have_applied =
        may_have_applied || (received.succeeded() && received.json().is_none());
    Delivery {
        result: Ok(received),
        may_have_applied,
    }
}

/// `rustls-no-provider` installs no crypto provider, and the offline harness
/// is cleartext, so nothing but this call stands between an https request and
/// a handshake failure.
fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn bundled_trust_roots() -> impl Iterator<Item = reqwest::Certificate> {
    webpki_root_certs::TLS_SERVER_ROOT_CERTS
        .iter()
        .filter_map(|root| reqwest::Certificate::from_der(root).ok())
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
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = source {
        if let Some(transport) = current.downcast_ref::<reqwest::Error>() {
            return connect_detail(transport);
        }
        source = current.source();
    }
    if error.kind() == std::io::ErrorKind::TimedOut {
        return "the request timed out".to_owned();
    }
    format!(
        "the response body could not be read — a stalled, truncated or \
         dropped body: {error}"
    )
}

/// Only a first-attempt connect failure is provably unsent: once an earlier
/// attempt drew a retryable response, that request may have been applied.
fn send_failure(error: &reqwest::Error, attempt: usize) -> ClientError {
    let detail = connect_detail(error);
    if attempt == 1 && error.is_connect() {
        ClientError::NotSent { detail }
    } else {
        ClientError::Transport { detail }
    }
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
