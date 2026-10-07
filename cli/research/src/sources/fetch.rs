//! The fetch workflow: a call admitted to one serving turn, its attempts
//! retried on the schedule within the call's deadline, then decoded and
//! normalised into records.

use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use crate::sources::arxiv;
use crate::sources::arxiv::Entry;
use crate::sources::arxiv::RawVersion;
use crate::sources::classify;
use crate::sources::classify::ClientRejection;
use crate::sources::classify::FetchError;
use crate::sources::classify::Reason;
use crate::sources::classify::Response;
use crate::sources::classify::Verdict;
use crate::sources::openalex;
use crate::sources::openalex::Work;
use crate::sources::queue::ArxivQueue;
use crate::sources::queue::Binding;
use crate::sources::queue::Joining;
use crate::sources::queue::Position;
use crate::sources::queue::Ticket;
use crate::sources::queue::TicketRejection;
use crate::sources::record::Record;
use crate::sources::request::ApiKey;
use crate::sources::request::ArxivId;
use crate::sources::request::ArxivRequest;
use crate::sources::request::Endpoint;
use crate::sources::request::Family;
use crate::sources::request::OpenAlexRequest;
use crate::sources::request::UpstreamRequest;
use crate::sources::request::Verb;
use crate::sources::schedule::Deadline;
use crate::sources::schedule::RetryNumber;
use crate::sources::schedule::RetrySchedule;

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

/// Admits calls to a source that tolerates one request at a time.
pub trait PacingGate {
    /// The least a request must wait after the one before it.
    fn spacing(&self) -> Duration;

    /// A turn holding the source, or `None` while another call holds it.
    fn try_serve(&self) -> Option<Box<dyn ServingTurn + '_>>;
}

/// Holds the source for as long as it lives, so every attempt and backoff
/// of a call runs without another call's requests between them.
pub trait ServingTurn {
    /// Runs `attempt` exactly once, or refuses without running it when the
    /// deadline cannot admit the wait the source still asks for.
    ///
    /// # Errors
    ///
    /// [`WaitTooLong`] when the attempt could not be admitted in time.
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        deadline: &Deadline,
    ) -> Result<(), WaitTooLong>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTooLong;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfBudget {
    pub last_retryable: Option<Reason>,
}

/// Why an arXiv call ended in lock contention with no upstream failure
/// behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Contention {
    TicketPastCap(Ticket),
    /// The call ran out of budget with no place in the queue to keep.
    QueueUnusable,
}

/// Where a call that lost its turn to contention is recorded.
pub trait ContentionLog {
    fn record(&self, contention: Contention);
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

    /// The call's ticket passed its cap, or the queue could not be used, and
    /// no upstream failure was left to report instead.
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
    /// The call ran out of budget but kept its place; re-presenting the
    /// ticket resumes it.
    Waiting {
        ticket: Ticket,
        position: Position,
    },
    Rejected(TicketRejection),
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
    pub queue: &'a dyn ArxivQueue,
    pub contention: &'a dyn ContentionLog,
}

pub struct ArxivFetch<'a> {
    pub request: &'a ArxivRequest,
    pub presented: Option<&'a Ticket>,
    pub api: &'a Endpoint,
    pub oai: &'a Endpoint,
}

pub fn fetch_openalex(
    fetch: &OpenAlexFetch<'_>,
    ports: &OpenAlexPorts<'_>,
    deadline: &Deadline,
) -> FetchOutcome {
    let Ok(turn) = admitted(ports.gate, ports.clock, deadline, &|| true) else {
        return FetchOutcome::Unavailable(Unavailable::new(
            Reason::RateLimited,
        ));
    };
    let attempts = Attempts {
        transport: ports.transport,
        clock: ports.clock,
        turn: turn.as_ref(),
        spacing: ports.gate.spacing(),
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
        Ok(Settled::Delivered(works)) => works,
        Ok(Settled::Empty) => Vec::new(),
        Ok(Settled::Unavailable(unavailable)) => {
            return FetchOutcome::Unavailable(unavailable)
        }
        Ok(Settled::Failed(error)) => return FetchOutcome::Failed(error),
        Err(out_of_budget) => {
            return FetchOutcome::Unavailable(Unavailable::new(
                out_of_budget.last_retryable.unwrap_or(Reason::RateLimited),
            ))
        }
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
/// The call waits its turn in the queue. One that runs out of budget keeps
/// its place and returns [`FetchOutcome::Waiting`]; one that never got a
/// place reports its last upstream failure or lock contention instead.
///
/// Confirmations share the query's deadline, and one that cannot be confirmed
/// makes the whole call unavailable, so an unconfirmed withdrawal is never
/// cited at the higher tier.
pub fn fetch_arxiv(
    fetch: &ArxivFetch<'_>,
    ports: &ArxivPorts<'_>,
    deadline: &Deadline,
) -> FetchOutcome {
    let binding = Binding::of(fetch.request);
    let joining = ports.queue.join(
        &binding,
        fetch.presented,
        deadline,
        ports.gate.spacing(),
    );
    match joining {
        Joining::Rejected(rejection) => FetchOutcome::Rejected(rejection),
        Joining::OverCap {
            ticket,
            last_retryable,
        } => upstream_reason_or_contention(
            ports,
            last_retryable,
            Contention::TicketPastCap(ticket),
        ),
        Joining::Unqueued => served_when(&|| true, fetch, ports, deadline)
            .unwrap_or_else(|out_of_budget| {
                upstream_reason_or_contention(
                    ports,
                    out_of_budget.last_retryable,
                    Contention::QueueUnusable,
                )
            }),
        Joining::Queued(place) => {
            match served_when(&|| place.is_front(), fetch, ports, deadline) {
                Ok(outcome) => {
                    place.leave(deadline);
                    outcome
                }
                Err(out_of_budget) => {
                    let ticket = place.ticket().clone();
                    let position = place
                        .step_aside(out_of_budget.last_retryable, deadline);
                    FetchOutcome::Waiting { ticket, position }
                }
            }
        }
    }
}

/// The only place the domain records contention: a call that ended with no
/// upstream failure to report instead.
fn upstream_reason_or_contention(
    ports: &ArxivPorts<'_>,
    last_retryable: Option<Reason>,
    contention: Contention,
) -> FetchOutcome {
    FetchOutcome::Unavailable(last_retryable.map_or_else(
        || {
            ports.contention.record(contention);
            Unavailable::lock_contention()
        },
        Unavailable::new,
    ))
}

const ADMISSION_POLL: Duration = Duration::from_millis(100);

/// Waits for a turn while `ready` holds back and the deadline still covers a
/// serving window: the source's spacing and one whole request.
fn admitted<'g>(
    gate: &'g dyn PacingGate,
    clock: &dyn Clock,
    deadline: &Deadline,
    ready: &dyn Fn() -> bool,
) -> Result<Box<dyn ServingTurn + 'g>, OutOfBudget> {
    loop {
        if !deadline.admits_attempt_after(clock.now(), gate.spacing()) {
            return Err(OutOfBudget {
                last_retryable: None,
            });
        }
        if ready() {
            if let Some(turn) = gate.try_serve() {
                return Ok(turn);
            }
        }
        clock.sleep(ADMISSION_POLL);
    }
}

fn served_when(
    ready: &dyn Fn() -> bool,
    fetch: &ArxivFetch<'_>,
    ports: &ArxivPorts<'_>,
    deadline: &Deadline,
) -> Result<FetchOutcome, OutOfBudget> {
    let turn = admitted(ports.gate, ports.clock, deadline, ready)?;
    serve_call(turn, fetch, ports, deadline)
}

/// Runs the query and every confirmation it needs within one turn, which is
/// released before this returns.
fn serve_call(
    turn: Box<dyn ServingTurn + '_>,
    fetch: &ArxivFetch<'_>,
    ports: &ArxivPorts<'_>,
    deadline: &Deadline,
) -> Result<FetchOutcome, OutOfBudget> {
    let attempts = Attempts {
        transport: ports.transport,
        clock: ports.clock,
        turn: turn.as_ref(),
        spacing: ports.gate.spacing(),
        deadline,
    };
    let served = served(&attempts, fetch, ports);
    drop(turn);
    served
}

fn served(
    attempts: &Attempts<'_>,
    fetch: &ArxivFetch<'_>,
    ports: &ArxivPorts<'_>,
) -> Result<FetchOutcome, OutOfBudget> {
    let settled = attempts.until_settled(
        &fetch.request.upstream(fetch.api),
        &classify::arxiv,
        &mut |body| {
            ports
                .decoder
                .entries(&body)
                .map_err(|failure| decode_error(Family::Arxiv, failure))
        },
    )?;
    let entries = match settled {
        Settled::Delivered(entries) => entries,
        Settled::Empty => Vec::new(),
        Settled::Unavailable(unavailable) => {
            return Ok(FetchOutcome::Unavailable(unavailable))
        }
        Settled::Failed(error) => return Ok(FetchOutcome::Failed(error)),
    };
    let listed = listed_entries(fetch.request, entries);

    let mut verdicts = Withdrawals::default();
    for (id, entry) in &listed {
        if !entry.is_withdrawal_candidate() || verdicts.knows(id) {
            continue;
        }
        let withdrawn = match ports.confirmations.recall(id) {
            Some(withdrawn) => withdrawn,
            None => match confirm_withdrawal(attempts, fetch.oai, ports, id)? {
                Settled::Delivered(withdrawn) => withdrawn,
                Settled::Empty => false,
                Settled::Unavailable(unavailable) => {
                    return Ok(FetchOutcome::Unavailable(unavailable))
                }
                Settled::Failed(error) => {
                    return Ok(FetchOutcome::Failed(error))
                }
            },
        };
        verdicts.note(id, withdrawn);
    }

    Ok(FetchOutcome::Records(
        listed
            .iter()
            .filter_map(|(id, entry)| {
                arxiv::normalise(entry, verdicts.withdrawn(id))
            })
            .collect(),
    ))
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

/// The verdict is recorded inside the turn that fetched it, so a concurrent
/// caller waiting for a turn finds it already cached.
fn confirm_withdrawal(
    attempts: &Attempts<'_>,
    oai: &Endpoint,
    ports: &ArxivPorts<'_>,
    id: &ArxivId,
) -> Result<Settled<bool>, OutOfBudget> {
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
    turn: &'a dyn ServingTurn,
    spacing: Duration,
    deadline: &'a Deadline,
}

impl Attempts<'_> {
    /// Attempts `request` until it settles or the retries run out, the last
    /// retryable attempt then deciding the reason. Each attempt must leave
    /// room for a serving window after its wait, or the call is out of
    /// budget. A delivered body is accepted inside the turn.
    fn until_settled<T>(
        &self,
        request: &UpstreamRequest,
        classify: &dyn Fn(Response) -> Verdict,
        accept: &mut dyn FnMut(Vec<u8>) -> Result<T, FetchError>,
    ) -> Result<Settled<T>, OutOfBudget> {
        let mut next_retry = RetryNumber::first();
        let mut wait = Duration::ZERO;
        let mut last_retryable = None;
        loop {
            let serving_window = wait + self.spacing;
            if !self
                .deadline
                .admits_attempt_after(self.clock.now(), serving_window)
            {
                return Err(OutOfBudget { last_retryable });
            }
            if !wait.is_zero() {
                self.clock.sleep(wait);
            }

            let mut step = None;
            let passed = self.turn.paced(
                &mut || {
                    let verdict = classify(self.transport.send(request));
                    let defer_until = self.deferral(&verdict, next_retry);
                    step = Some(Self::step(verdict, accept));
                    Attempted { defer_until }
                },
                self.deadline,
            );
            if passed.is_err() {
                return Err(OutOfBudget { last_retryable });
            }

            match step {
                Some(Step::Settled(settled)) => return Ok(settled),
                Some(Step::Retry {
                    reason,
                    retry_after,
                }) => {
                    last_retryable = Some(reason);
                    let Some(next_wait) =
                        RetrySchedule::wait_before(next_retry, retry_after)
                    else {
                        return Ok(Settled::Unavailable(Unavailable::new(
                            reason,
                        )));
                    };
                    wait = next_wait;
                    next_retry = next_retry.next();
                }
                None => return Err(OutOfBudget { last_retryable }),
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
