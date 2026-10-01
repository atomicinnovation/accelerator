//! The fetch workflow: one attempt per gate pass, retried on the schedule
//! within the call's deadline, then decoded and normalised into records.

use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use crate::arxiv;
use crate::arxiv::Entry;
use crate::arxiv::RawVersion;
use crate::classify;
use crate::classify::ClientRejection;
use crate::classify::FetchError;
use crate::classify::Reason;
use crate::classify::Response;
use crate::classify::Verdict;
use crate::openalex;
use crate::openalex::Work;
use crate::record::Record;
use crate::request::ApiKey;
use crate::request::ArxivId;
use crate::request::ArxivRequest;
use crate::request::Endpoint;
use crate::request::Family;
use crate::request::OpenAlexRequest;
use crate::request::UpstreamRequest;
use crate::request::Verb;
use crate::schedule::Deadline;
use crate::schedule::RetryNumber;
use crate::schedule::RetrySchedule;

pub trait Transport {
    fn send(&self, request: &UpstreamRequest) -> Response;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeFailure {
    Undecodable,
    /// A well-formed body in which the source reports an error of its own.
    ErrorFeed(String),
}

pub trait OpenAlexDecoder {
    /// # Errors
    ///
    /// A [`DecodeFailure`] when the body is not a page of works.
    fn works(&self, body: &[u8]) -> Result<Vec<Work>, DecodeFailure>;

    /// # Errors
    ///
    /// A [`DecodeFailure`] when the body is not a single work.
    fn work(&self, body: &[u8]) -> Result<Work, DecodeFailure>;
}

pub trait ArxivDecoder {
    /// # Errors
    ///
    /// A [`DecodeFailure`] when the body is not an Atom feed of entries.
    fn entries(&self, body: &[u8]) -> Result<Vec<Entry>, DecodeFailure>;

    /// # Errors
    ///
    /// A [`DecodeFailure`] when the body is not an `arXivRaw` record.
    fn raw_versions(
        &self,
        body: &[u8],
    ) -> Result<Vec<RawVersion>, DecodeFailure>;
}

/// `now` serves in-process arithmetic; `wall_now` stamps values persisted
/// for other processes to read.
pub trait Clock {
    fn now(&self) -> Instant;
    fn wall_now(&self) -> SystemTime;
    fn sleep(&self, duration: Duration);
}

/// Withdrawal verdicts already confirmed, keyed by the identifier as the feed
/// reported it, so a newer version is confirmed afresh.
pub trait ConfirmationCache {
    fn recall(&self, entry: &ArxivId) -> Option<bool>;
    fn record(&self, entry: &ArxivId, withdrawn: bool);
}

/// What the gate needs to know once an attempt has run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempted {
    /// When a throttled source asks every caller to wait until.
    pub defer_until: Option<SystemTime>,
}

/// Paces attempts against a source that tolerates one request at a time.
pub trait PacingGate {
    /// Runs `attempt` exactly once while holding the source, or refuses
    /// without running it when the deadline cannot admit the wait.
    ///
    /// # Errors
    ///
    /// [`Unavailable`] when the attempt could not be admitted in time.
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        deadline: &Deadline,
    ) -> Result<(), Unavailable>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    LockContention,
}

impl Cause {
    pub const fn code(self) -> &'static str {
        match self {
            Self::LockContention => "lock_contention",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailable {
    reason: Reason,
    cause: Option<Cause>,
}

impl Unavailable {
    pub const fn new(reason: Reason) -> Self {
        Self {
            reason,
            cause: None,
        }
    }

    /// Another process held the source for longer than the deadline allowed.
    pub const fn lock_contention() -> Self {
        Self {
            reason: Reason::RateLimited,
            cause: Some(Cause::LockContention),
        }
    }

    pub const fn reason(self) -> Reason {
        self.reason
    }

    pub const fn cause(self) -> Option<Cause> {
        self.cause
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchOutcome {
    Records(Vec<Record>),
    Unavailable(Unavailable),
    Failed(FetchError),
}

pub struct OpenAlexPorts<'a> {
    pub transport: &'a dyn Transport,
    pub decoder: &'a dyn OpenAlexDecoder,
    pub clock: &'a dyn Clock,
    pub gate: &'a dyn PacingGate,
}

pub struct OpenAlexFetch<'a> {
    pub request: &'a OpenAlexRequest,
    pub api: &'a Endpoint,
    pub key: Option<&'a ApiKey>,
}

pub struct ArxivPorts<'a> {
    pub transport: &'a dyn Transport,
    pub decoder: &'a dyn ArxivDecoder,
    pub clock: &'a dyn Clock,
    pub gate: &'a dyn PacingGate,
    pub confirmations: &'a dyn ConfirmationCache,
}

pub struct ArxivFetch<'a> {
    pub request: &'a ArxivRequest,
    pub api: &'a Endpoint,
    pub oai: &'a Endpoint,
}

pub fn fetch_openalex(
    fetch: &OpenAlexFetch<'_>,
    ports: &OpenAlexPorts<'_>,
    deadline: &Deadline,
) -> FetchOutcome {
    let attempts = Attempts {
        transport: ports.transport,
        clock: ports.clock,
        gate: ports.gate,
        deadline,
    };
    let verb = fetch.request.verb();
    let key = fetch.key.map(ApiKey::source);
    let settled = attempts.until_settled(
        &fetch.request.upstream(fetch.api, fetch.key),
        &|response| classify::openalex(response, verb, key),
        &mut |body| {
            match verb {
                Verb::Search => ports.decoder.works(&body),
                Verb::Lookup => {
                    ports.decoder.work(&body).map(|work| vec![work])
                }
            }
            .map_err(|failure| decode_error(Family::OpenAlex, failure))
        },
    );
    let works = match settled {
        Settled::Delivered(works) => works,
        Settled::Empty => Vec::new(),
        Settled::Unavailable(unavailable) => {
            return FetchOutcome::Unavailable(unavailable)
        }
        Settled::Failed(error) => return FetchOutcome::Failed(error),
    };
    FetchOutcome::Records(
        works
            .iter()
            .filter_map(openalex::normalise)
            .take(fetch.request.limit().records())
            .collect(),
    )
}

/// Fetches arXiv entries, confirming every withdrawal candidate.
///
/// Confirmations share the query's deadline, and one that cannot be confirmed
/// makes the whole call unavailable, so an unconfirmed withdrawal is never
/// cited at the higher tier.
pub fn fetch_arxiv(
    fetch: &ArxivFetch<'_>,
    ports: &ArxivPorts<'_>,
    deadline: &Deadline,
) -> FetchOutcome {
    let attempts = Attempts {
        transport: ports.transport,
        clock: ports.clock,
        gate: ports.gate,
        deadline,
    };
    let settled = attempts.until_settled(
        &fetch.request.upstream(fetch.api),
        &classify::arxiv,
        &mut |body| {
            ports
                .decoder
                .entries(&body)
                .map_err(|failure| decode_error(Family::Arxiv, failure))
        },
    );
    let entries = match settled {
        Settled::Delivered(entries) => entries,
        Settled::Empty => Vec::new(),
        Settled::Unavailable(unavailable) => {
            return FetchOutcome::Unavailable(unavailable)
        }
        Settled::Failed(error) => return FetchOutcome::Failed(error),
    };
    let listed = listed_entries(fetch.request, entries);

    let mut verdicts = Withdrawals::default();
    for (id, entry) in &listed {
        if !entry.is_withdrawal_candidate() || verdicts.knows(id) {
            continue;
        }
        let withdrawn = match ports.confirmations.recall(id) {
            Some(withdrawn) => withdrawn,
            None => match confirm_withdrawal(&attempts, fetch.oai, ports, id) {
                Settled::Delivered(withdrawn) => withdrawn,
                Settled::Empty => false,
                Settled::Unavailable(unavailable) => {
                    return FetchOutcome::Unavailable(unavailable)
                }
                Settled::Failed(error) => return FetchOutcome::Failed(error),
            },
        };
        verdicts.note(id, withdrawn);
    }

    FetchOutcome::Records(
        listed
            .iter()
            .filter_map(|(id, entry)| {
                arxiv::normalise(entry, verdicts.withdrawn(id))
            })
            .collect(),
    )
}

/// The citable entries a request asked for, in feed order and within its
/// limit. A lookup keeps only the entry it named: arXiv answers a miss with
/// some other entry or none at all.
fn listed_entries(
    request: &ArxivRequest,
    entries: Vec<Entry>,
) -> Vec<(ArxivId, Entry)> {
    entries
        .into_iter()
        .filter_map(|entry| Some((entry.arxiv_id()?, entry)))
        .filter(|(id, _)| match request {
            ArxivRequest::Search { .. } => true,
            ArxivRequest::Lookup(wanted) => {
                id.unversioned() == wanted.unversioned()
            }
        })
        .take(request.limit().records())
        .collect()
}

/// The verdict is recorded inside the gate pass that fetched it, so a
/// concurrent caller waiting on the gate finds it already cached.
fn confirm_withdrawal(
    attempts: &Attempts<'_>,
    oai: &Endpoint,
    ports: &ArxivPorts<'_>,
    id: &ArxivId,
) -> Settled<bool> {
    attempts.until_settled(
        &UpstreamRequest::withdrawal_confirmation(oai, id),
        &classify::arxiv,
        &mut |body| {
            let versions = ports
                .decoder
                .raw_versions(&body)
                .map_err(|failure| decode_error(Family::Arxiv, failure))?;
            let withdrawn = arxiv::latest_version_withdrawn(&versions);
            ports.confirmations.record(id, withdrawn);
            Ok(withdrawn)
        },
    )
}

#[derive(Default)]
struct Withdrawals(Vec<(ArxivId, bool)>);

impl Withdrawals {
    fn knows(&self, id: &ArxivId) -> bool {
        self.0.iter().any(|(known, _)| known == id)
    }

    fn note(&mut self, id: &ArxivId, withdrawn: bool) {
        self.0.push((id.clone(), withdrawn));
    }

    fn withdrawn(&self, id: &ArxivId) -> bool {
        self.0
            .iter()
            .find(|(known, _)| known == id)
            .is_some_and(|(_, withdrawn)| *withdrawn)
    }
}

fn decode_error(family: Family, failure: DecodeFailure) -> FetchError {
    match failure {
        DecodeFailure::Undecodable => FetchError::UndecodableResponse(family),
        DecodeFailure::ErrorFeed(message) => FetchError::ClientError {
            family,
            rejection: ClientRejection::ErrorFeed(message),
        },
    }
}

enum Settled<T> {
    Delivered(T),
    Empty,
    Unavailable(Unavailable),
    Failed(FetchError),
}

enum Step<T> {
    Settled(Settled<T>),
    Retry {
        reason: Reason,
        retry_after: Option<Duration>,
    },
}

struct Attempts<'a> {
    transport: &'a dyn Transport,
    clock: &'a dyn Clock,
    gate: &'a dyn PacingGate,
    deadline: &'a Deadline,
}

impl Attempts<'_> {
    /// Attempts `request` until it settles, the retries run out, or the
    /// deadline cannot admit the next wait; the last retryable attempt then
    /// decides the reason. A delivered body is accepted inside the gate pass.
    fn until_settled<T>(
        &self,
        request: &UpstreamRequest,
        classify: &dyn Fn(Response) -> Verdict,
        accept: &mut dyn FnMut(Vec<u8>) -> Result<T, FetchError>,
    ) -> Settled<T> {
        let mut next_retry = RetryNumber::first();
        let mut wait = Duration::ZERO;
        let mut last_retryable = None;
        loop {
            if !self.deadline.admits_attempt_after(self.clock.now(), wait) {
                return Settled::Unavailable(Unavailable::new(
                    last_retryable.unwrap_or(Reason::RateLimited),
                ));
            }
            if !wait.is_zero() {
                self.clock.sleep(wait);
            }

            let mut step = None;
            let passed = self.gate.paced(
                &mut || {
                    let verdict = classify(self.transport.send(request));
                    let defer_until = self.deferral(&verdict, next_retry);
                    step = Some(Self::step(verdict, accept));
                    Attempted { defer_until }
                },
                self.deadline,
            );
            if let Err(refused) = passed {
                return Settled::Unavailable(Unavailable {
                    reason: last_retryable.unwrap_or(refused.reason),
                    cause: refused.cause,
                });
            }

            match step {
                Some(Step::Settled(settled)) => return settled,
                Some(Step::Retry {
                    reason,
                    retry_after,
                }) => {
                    last_retryable = Some(reason);
                    let Some(next_wait) =
                        RetrySchedule::wait_before(next_retry, retry_after)
                    else {
                        return Settled::Unavailable(Unavailable::new(reason));
                    };
                    wait = next_wait;
                    next_retry = next_retry.next();
                }
                None => {
                    return Settled::Unavailable(Unavailable::new(
                        last_retryable.unwrap_or(Reason::RateLimited),
                    ))
                }
            }
        }
    }

    fn deferral(
        &self,
        verdict: &Verdict,
        next_retry: RetryNumber,
    ) -> Option<SystemTime> {
        let Verdict::Retry { retry_after, .. } = verdict else {
            return None;
        };
        verdict.is_throttling().then(|| {
            self.clock.wall_now()
                + RetrySchedule::deferral_after(next_retry, *retry_after)
        })
    }

    fn step<T>(
        verdict: Verdict,
        accept: &mut dyn FnMut(Vec<u8>) -> Result<T, FetchError>,
    ) -> Step<T> {
        Step::Settled(match verdict {
            Verdict::Deliver(body) => {
                accept(body).map_or_else(Settled::Failed, Settled::Delivered)
            }
            Verdict::Empty => Settled::Empty,
            Verdict::Unavailable(reason) => {
                Settled::Unavailable(Unavailable::new(reason))
            }
            Verdict::Fail(error) => Settled::Failed(error),
            Verdict::Retry {
                reason,
                retry_after,
            } => {
                return Step::Retry {
                    reason,
                    retry_after,
                }
            }
        })
    }
}
