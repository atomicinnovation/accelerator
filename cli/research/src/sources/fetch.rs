//! The fetch workflow: a call admitted to one serving turn, its attempts
//! retried on the schedule within the call's deadline, then decoded and
//! normalised into records.

pub use crate::sources::ports::Attempted;
pub use crate::sources::ports::Cause;
pub use crate::sources::ports::Clock;
pub use crate::sources::ports::Contention;
pub use crate::sources::ports::ContentionLog;
pub use crate::sources::ports::Diagnostics;
pub use crate::sources::ports::OutOfBudget;
pub use crate::sources::ports::PacingGate;
pub use crate::sources::ports::ServingTurn;
pub use crate::sources::ports::Transport;
pub use crate::sources::ports::Unavailable;
pub use crate::sources::ports::WaitTooLong;

use crate::sources::admission::admitted;
use crate::sources::arxiv;
use crate::sources::arxiv::Entry;
use crate::sources::arxiv::RawVersion;
use crate::sources::classify;
use crate::sources::classify::ClientRejection;
use crate::sources::classify::FetchError;
use crate::sources::classify::Reason;
use crate::sources::crossref;
use crate::sources::crossref::ContactAddress;
use crate::sources::crossref::CrossrefWork;
use crate::sources::crossref::DoiMetadata;
use crate::sources::metadata::date::PartialDate;
use crate::sources::openalex;
use crate::sources::openalex::Work;
use crate::sources::queue::Binding;
use crate::sources::queue::ServiceQueue;
use crate::sources::queue::Ticket;
use crate::sources::record::Record;
use crate::sources::request::ApiKey;
use crate::sources::request::ArxivId;
use crate::sources::request::ArxivRequest;
use crate::sources::request::Doi;
use crate::sources::request::Endpoint;
use crate::sources::request::Family;
use crate::sources::request::OpenAlexRequest;
use crate::sources::request::UpstreamRequest;
use crate::sources::schedule::Deadline;
use crate::sources::serving::queued;
use crate::sources::serving::Attempts;
use crate::sources::serving::QueuePorts;
use crate::sources::serving::Queued;
use crate::sources::serving::Served;
use crate::sources::serving::Settled;

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

pub trait CrossrefDecoder {
    /// # Errors
    ///
    /// A [`DecodeFailure`] when the body is not a Crossref work message.
    fn work(&self, body: &[u8]) -> Result<CrossrefWork, DecodeFailure>;
}

/// Withdrawal verdicts already confirmed, keyed by the identifier as the feed
/// reported it, so a newer version is confirmed afresh.
pub trait ConfirmationCache {
    fn recall(&self, entry: &ArxivId) -> Option<bool>;
    fn record(&self, entry: &ArxivId, withdrawn: bool);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchOutcome {
    Records(Vec<Record>),
    Unavailable(Unavailable),
    Failed(FetchError),
}

/// What an arXiv call that was served answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArxivAnswer {
    Records(Vec<Record>),
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
    pub queue: &'a dyn ServiceQueue,
    pub contention: &'a dyn ContentionLog,
}

pub struct ArxivFetch<'a> {
    pub request: &'a ArxivRequest,
    pub presented: Option<&'a Ticket>,
    pub api: &'a Endpoint,
    pub oai: &'a Endpoint,
}

pub struct DoiPorts<'a> {
    pub transport: &'a dyn Transport,
    pub decoder: &'a dyn CrossrefDecoder,
    pub clock: &'a dyn Clock,
    pub gate: &'a dyn PacingGate,
    pub queue: &'a dyn ServiceQueue,
    pub contention: &'a dyn ContentionLog,
    pub diagnostics: &'a dyn Diagnostics,
}

pub struct DoiFetch<'a> {
    pub doi: &'a Doi,
    pub presented: Option<&'a Ticket>,
    pub api: &'a Endpoint,
    pub contact: Option<&'a ContactAddress>,
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
    let attempts = Attempts::within(
        turn.as_ref(),
        ports.transport,
        ports.clock,
        ports.gate,
        deadline,
    );
    let verb = fetch.request.verb();
    let key = fetch.key.map(ApiKey::source);
    let settled = attempts.until_settled(
        &fetch.request.upstream(fetch.api, fetch.key),
        &|response| classify::openalex(response, verb, key),
        &mut |body| {
            match fetch.request {
                OpenAlexRequest::Search { .. } => ports.decoder.works(&body),
                OpenAlexRequest::Lookup(_) => {
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
    let accessed = PartialDate::utc_date_of(ports.clock.wall_now());
    FetchOutcome::Records(
        works
            .iter()
            .filter_map(|work| openalex::normalise(work, accessed))
            .take(fetch.request.limit().records())
            .collect(),
    )
}

/// Fetches arXiv entries, confirming every withdrawal candidate, once the
/// call's turn in arXiv's queue comes.
///
/// Confirmations share the query's deadline, and one that cannot be confirmed
/// makes the whole call unavailable, so an unconfirmed withdrawal is never
/// cited at the higher tier.
pub fn fetch_arxiv(
    fetch: &ArxivFetch<'_>,
    ports: &ArxivPorts<'_>,
    deadline: &Deadline,
) -> Queued<ArxivAnswer> {
    queued(
        &Binding::of(fetch.request),
        fetch.presented,
        &QueuePorts {
            transport: ports.transport,
            clock: ports.clock,
            gate: ports.gate,
            queue: ports.queue,
            contention: ports.contention,
        },
        deadline,
        &mut |attempts| served(attempts, fetch, ports),
    )
}

/// Fetches what Crossref holds for a DOI once the call's turn in its pool's
/// queue comes.
///
/// Only a usage error may fail a DOI call, so a record that cannot be read
/// leaves Crossref unavailable, and is reported so that a change to
/// Crossref's schema can be told apart from an outage.
pub fn fetch_doi(
    fetch: &DoiFetch<'_>,
    ports: &DoiPorts<'_>,
    deadline: &Deadline,
) -> Queued<DoiMetadata> {
    queued(
        &Binding::DoiMetadata(fetch.doi.clone()),
        fetch.presented,
        &QueuePorts {
            transport: ports.transport,
            clock: ports.clock,
            gate: ports.gate,
            queue: ports.queue,
            contention: ports.contention,
        },
        deadline,
        &mut |attempts| {
            let settled = attempts.until_settled(
                &UpstreamRequest::crossref_work(
                    fetch.api,
                    fetch.doi,
                    fetch.contact,
                ),
                &classify::crossref,
                &mut |body| Ok(ports.decoder.work(&body)),
            )?;
            Ok(match settled {
                Settled::Delivered(Ok(work)) => Ok(crossref::normalise(&work)),
                Settled::Delivered(Err(_)) => {
                    ports.diagnostics.report(&format!(
                        "research fetch: doi metadata {}: Crossref returned a \
                         record that could not be read",
                        fetch.doi.canonical()
                    ));
                    Err(Unavailable::new(Reason::UpstreamError))
                }
                Settled::Empty => Ok(DoiMetadata::default()),
                Settled::Unavailable(unavailable) => Err(unavailable),
                Settled::Failed(_) => {
                    Err(Unavailable::new(Reason::UpstreamError))
                }
            })
        },
    )
}

/// Runs the query and every confirmation it needs within one turn.
fn served(
    attempts: &Attempts<'_>,
    fetch: &ArxivFetch<'_>,
    ports: &ArxivPorts<'_>,
) -> Served<ArxivAnswer> {
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
        Settled::Unavailable(unavailable) => return Ok(Err(unavailable)),
        Settled::Failed(error) => return Ok(Ok(ArxivAnswer::Failed(error))),
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
                    return Ok(Err(unavailable))
                }
                Settled::Failed(error) => {
                    return Ok(Ok(ArxivAnswer::Failed(error)))
                }
            },
        };
        verdicts.note(id, withdrawn);
    }

    let accessed = PartialDate::utc_date_of(ports.clock.wall_now());
    Ok(Ok(ArxivAnswer::Records(
        listed
            .iter()
            .filter_map(|(id, entry)| {
                arxiv::normalise(entry, verdicts.withdrawn(id), accessed)
            })
            .collect(),
    )))
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
