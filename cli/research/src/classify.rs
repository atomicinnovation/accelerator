//! What each source's response means: deliver it, retry it, give up on the
//! source for now, or fail the call.

use std::fmt;
use std::time::Duration;

use crate::request::Family;
use crate::request::KeySource;
use crate::request::Verb;

/// One attempt's result as the transport observed it. A connection failure
/// or timeout is an outcome to classify, not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Received(Received),
    ConnectionFailed,
    TimedOut,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Received {
    pub status: u16,
    pub retry_after: Option<Duration>,
    pub rate_limit_remaining: Option<u64>,
    pub rate_limit_credits_required: Option<u64>,
    pub body: Vec<u8>,
}

impl Received {
    /// A response with this status and no headers or body.
    pub fn status(status: u16) -> Self {
        Self {
            status,
            ..Self::default()
        }
    }
}

/// Why a source could not answer now. A call ending here still succeeds: the
/// caller records the source as unavailable rather than failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    RateLimited,
    BudgetExhausted,
    UpstreamError,
}

impl Reason {
    pub const fn code(self) -> &'static str {
        match self {
            Self::RateLimited => "rate_limited",
            Self::BudgetExhausted => "budget_exhausted",
            Self::UpstreamError => "upstream_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientRejection {
    Status(u16),
    Redirect(u16),
    ErrorFeed(String),
}

/// A call that cannot succeed by retrying or waiting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    KeyRejected(KeySource),
    Unauthenticated,
    ClientError {
        family: Family,
        rejection: ClientRejection,
    },
    UndecodableResponse(Family),
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyRejected(source) => write!(
                formatter,
                "E_OPENALEX_KEY_REJECTED: the OpenAlex API key from {source} \
                 was rejected"
            ),
            Self::Unauthenticated => formatter.write_str(
                "E_OPENALEX_UNAUTHENTICATED: OpenAlex refused a keyless \
                 request — configure openalex.api_key",
            ),
            Self::ClientError {
                family,
                rejection: ClientRejection::Status(status),
            } => write!(
                formatter,
                "E_RESEARCH_CLIENT_ERROR: {family} rejected the request with \
                 HTTP {status}"
            ),
            Self::ClientError {
                family,
                rejection: ClientRejection::Redirect(status),
            } => write!(
                formatter,
                "E_RESEARCH_CLIENT_ERROR: {family} redirected the request \
                 with HTTP {status} to a location that is not followed"
            ),
            Self::ClientError {
                family,
                rejection: ClientRejection::ErrorFeed(message),
            } => write!(
                formatter,
                "E_RESEARCH_CLIENT_ERROR: {family} rejected the request: \
                 {message}"
            ),
            Self::UndecodableResponse(family) => write!(
                formatter,
                "E_RESEARCH_UNDECODABLE: {family} returned a response that \
                 could not be read"
            ),
        }
    }
}

impl std::error::Error for FetchError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Deliver(Vec<u8>),
    Empty,
    Retry {
        reason: Reason,
        retry_after: Option<Duration>,
    },
    Unavailable(Reason),
    Fail(FetchError),
}

impl Verdict {
    pub(crate) const fn is_throttling(&self) -> bool {
        matches!(
            self,
            Self::Retry {
                reason: Reason::RateLimited,
                ..
            }
        )
    }
}

/// `key` is the source of the key the request carried, if any.
pub fn openalex(
    response: Response,
    verb: Verb,
    key: Option<&KeySource>,
) -> Verdict {
    let received = match response {
        Response::Received(received) => received,
        Response::ConnectionFailed | Response::TimedOut => {
            return upstream_error(None)
        }
    };
    match received.status {
        200..=299 => Verdict::Deliver(received.body),
        404 if verb == Verb::Lookup => Verdict::Empty,
        409 => Verdict::Unavailable(Reason::BudgetExhausted),
        429 if is_budget_exhausted(&received) => {
            Verdict::Unavailable(Reason::BudgetExhausted)
        }
        429 => Verdict::Retry {
            reason: Reason::RateLimited,
            retry_after: received.retry_after,
        },
        401 | 403 => {
            Verdict::Fail(key.map_or(FetchError::Unauthenticated, |source| {
                FetchError::KeyRejected(source.clone())
            }))
        }
        status => shared(Family::OpenAlex, status, received.retry_after),
    }
}

pub fn arxiv(response: Response) -> Verdict {
    let received = match response {
        Response::Received(received) => received,
        Response::ConnectionFailed | Response::TimedOut => {
            return upstream_error(None)
        }
    };
    match received.status {
        200..=299 => Verdict::Deliver(received.body),
        403 | 406 | 429 => Verdict::Retry {
            reason: Reason::RateLimited,
            retry_after: received.retry_after,
        },
        status => shared(Family::Arxiv, status, received.retry_after),
    }
}

const fn shared(
    family: Family,
    status: u16,
    retry_after: Option<Duration>,
) -> Verdict {
    match status {
        300..=399 => client_error(family, ClientRejection::Redirect(status)),
        500..=599 => upstream_error(retry_after),
        _ => client_error(family, ClientRejection::Status(status)),
    }
}

/// OpenAlex spends credits per request; a `429` means the day's budget is
/// gone when fewer remain than this request needs.
fn is_budget_exhausted(received: &Received) -> bool {
    received.rate_limit_remaining.is_some_and(|remaining| {
        received
            .rate_limit_credits_required
            .map_or(remaining == 0, |required| remaining < required)
    })
}

const fn upstream_error(retry_after: Option<Duration>) -> Verdict {
    Verdict::Retry {
        reason: Reason::UpstreamError,
        retry_after,
    }
}

const fn client_error(family: Family, rejection: ClientRejection) -> Verdict {
    Verdict::Fail(FetchError::ClientError { family, rejection })
}
