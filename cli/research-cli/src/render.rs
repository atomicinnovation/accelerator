//! What a call prints: compact JSON on stdout for records, unavailability
//! and a waiting ticket, and one non-secret stderr line for a call that did
//! not deliver.

use research::sources::classify::FetchError;
use research::sources::crossref::DoiMetadata;
use research::sources::crossref::Venue;
use research::sources::fetch::ArxivAnswer;
use research::sources::fetch::FetchOutcome;
use research::sources::metadata::SourceMetadata;
use research::sources::ports::Cause;
use research::sources::ports::Unavailable;
use research::sources::queue::Position;
use research::sources::queue::RejectedTicket;
use research::sources::queue::Ticket;
use research::sources::record::Record;
use research::sources::record::VenueSignals;
use research::sources::request::Family;
use research::sources::request::Verb;
use research::sources::service::Service;
use research::sources::serving::Queued;
use serde::ser::SerializeMap;
use serde::Serialize;
use serde::Serializer;

use crate::fetch_command::FetchedDoi;
use crate::fetch_command::FetchedRecords;
use crate::fetch_command::RecordsOutcome;

/// How a call ended, read the same way whichever source it asked; `S` is
/// what a served call answered.
enum Ending<'a, S> {
    Served(S),
    Failed(&'a FetchError),
    Unavailable {
        source: &'static str,
        unavailable: Unavailable,
    },
    Waiting {
        source: &'static str,
        ticket: &'a Ticket,
        position: Position,
    },
    Rejected(RejectedTicket),
}

fn ending(fetched: &FetchedRecords) -> Ending<'_, &[Record]> {
    match &fetched.outcome {
        RecordsOutcome::Direct(FetchOutcome::Records(records)) => {
            Ending::Served(records)
        }
        RecordsOutcome::Direct(FetchOutcome::Failed(error)) => {
            Ending::Failed(error)
        }
        RecordsOutcome::Direct(FetchOutcome::Unavailable(unavailable)) => {
            Ending::Unavailable {
                source: fetched.family.code(),
                unavailable: *unavailable,
            }
        }
        RecordsOutcome::Queued { service, queued } => {
            queued_ending(*service, queued, |answer| match answer {
                ArxivAnswer::Records(records) => {
                    Ending::Served(records.as_slice())
                }
                ArxivAnswer::Failed(error) => Ending::Failed(error),
            })
        }
    }
}

fn doi_ending(fetched: &FetchedDoi) -> Ending<'_, &DoiMetadata> {
    queued_ending(fetched.service, &fetched.outcome, Ending::Served)
}

/// The one reading of every queue state, for any family a service serves.
fn queued_ending<'a, T, S>(
    service: Service,
    queued: &'a Queued<T>,
    served: impl FnOnce(&'a T) -> Ending<'a, S>,
) -> Ending<'a, S> {
    match queued {
        Queued::Served(answer) => served(answer),
        Queued::Unavailable(unavailable) => Ending::Unavailable {
            source: service.source(),
            unavailable: *unavailable,
        },
        Queued::Waiting { ticket, position } => Ending::Waiting {
            source: service.source(),
            ticket,
            position: *position,
        },
        Queued::Rejected(rejection) => Ending::Rejected(RejectedTicket {
            presented_to: service.ticket_family(),
            rejection: rejection.clone(),
        }),
    }
}

/// The stdout document, or `None` for a failed or rejected call, which
/// prints only its diagnostics.
pub fn document(fetched: &FetchedRecords) -> Option<String> {
    let authenticated =
        (fetched.family == Family::OpenAlex).then_some(fetched.authenticated);
    match ending(fetched) {
        Ending::Served(records) => serde_json::to_string(&Document::Ok {
            records: records.iter().map(RecordJson::from).collect(),
        })
        .ok(),
        other => unserved_document(&other, authenticated),
    }
}

/// The stdout document of a DOI call, or `None` for a rejected one.
pub fn doi_document(fetched: &FetchedDoi) -> Option<String> {
    match doi_ending(fetched) {
        Ending::Served(answer) => serde_json::to_string(&DoiDocument {
            status: "ok",
            metadata: MetadataJson(&answer.metadata),
            venue: answer.venue.as_ref().map(Venue::as_str),
        })
        .ok(),
        other => unserved_document(&other, None),
    }
}

/// The document of a call its source or queue did not serve.
fn unserved_document<S>(
    ending: &Ending<'_, S>,
    authenticated: Option<bool>,
) -> Option<String> {
    let document = match ending {
        Ending::Unavailable {
            source,
            unavailable,
        } => Document::Unavailable {
            source,
            reason: unavailable.reason().code(),
            authenticated,
            cause: unavailable.cause().map(Cause::code),
        },
        Ending::Waiting {
            source,
            ticket,
            position,
        } => Document::Waiting {
            source,
            ticket: ticket.to_string(),
            position: position.get(),
        },
        Ending::Served(_) | Ending::Failed(_) | Ending::Rejected(_) => {
            return None
        }
    };
    serde_json::to_string(&document).ok()
}

#[derive(Serialize)]
struct DoiDocument<'a> {
    status: &'static str,
    metadata: MetadataJson<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    venue: Option<&'a str>,
}

/// The upstream's own error line for a call that failed.
pub fn failure(fetched: &FetchedRecords) -> Option<String> {
    match ending(fetched) {
        Ending::Failed(error) => Some(error.to_string()),
        _ => None,
    }
}

pub fn web_document(metadata: &SourceMetadata) -> Option<String> {
    serde_json::to_string(&WebDocument {
        status: "ok",
        metadata: MetadataJson(metadata),
    })
    .ok()
}

#[derive(Serialize)]
struct WebDocument<'a> {
    status: &'static str,
    metadata: MetadataJson<'a>,
}

/// The usage line for a ticket presented in a way the queue refuses.
pub fn rejection(fetched: &FetchedRecords) -> Option<String> {
    rejection_of(ending(fetched))
}

pub fn doi_rejection(fetched: &FetchedDoi) -> Option<String> {
    rejection_of(doi_ending(fetched))
}

fn rejection_of<S>(ending: Ending<'_, S>) -> Option<String> {
    match ending {
        Ending::Rejected(rejected) => Some(rejected.to_string()),
        _ => None,
    }
}

/// The line naming the source, verb and final status of a call that did not
/// deliver records: with its attempts, or for a waiting call its position
/// and the ticket to re-present.
pub fn summary(fetched: &FetchedRecords) -> Option<String> {
    summary_of(
        fetched.family,
        fetched.verb,
        fetched.attempts,
        &ending(fetched),
    )
}

pub fn doi_summary(fetched: &FetchedDoi) -> Option<String> {
    summary_of(
        Family::Doi,
        Verb::Metadata,
        fetched.attempts,
        &doi_ending(fetched),
    )
}

fn summary_of<S>(
    family: Family,
    verb: Verb,
    attempts: usize,
    ending: &Ending<'_, S>,
) -> Option<String> {
    let status = match ending {
        Ending::Served(_) | Ending::Rejected(_) => return None,
        Ending::Waiting {
            ticket, position, ..
        } => {
            return Some(format!(
                "research fetch: {} {} waiting at position {}; re-present \
                 with --ticket {ticket}",
                family.code(),
                verb.code(),
                position.get(),
            ))
        }
        Ending::Unavailable { unavailable, .. } => {
            format!("unavailable ({})", unavailable.reason().code())
        }
        Ending::Failed(_) => "failed".to_owned(),
    };
    let attempts = match attempts {
        1 => "1 attempt".to_owned(),
        count => format!("{count} attempts"),
    };
    Some(format!(
        "research fetch: {} {} {status} after {attempts}",
        family.code(),
        verb.code(),
    ))
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Document<'a> {
    Ok {
        records: Vec<RecordJson<'a>>,
    },
    Unavailable {
        source: &'static str,
        reason: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        authenticated: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cause: Option<&'static str>,
    },
    Waiting {
        source: &'static str,
        ticket: String,
        position: u32,
    },
}

#[derive(Serialize)]
struct RecordJson<'a> {
    title: Option<&'a str>,
    authors: &'a [String],
    url: &'a str,
    venue: Option<&'a str>,
    venue_signals: SignalsJson<'a>,
    #[serde(rename = "abstract")]
    abstract_excerpt: Option<&'a str>,
    tier: String,
    retracted: bool,
    withdrawn: bool,
    metadata: MetadataJson<'a>,
}

impl<'a> From<&'a Record> for RecordJson<'a> {
    fn from(record: &'a Record) -> Self {
        Self {
            title: record.title(),
            authors: record.authors(),
            url: record.url(),
            venue: record.venue(),
            venue_signals: SignalsJson::from(record.venue_signals()),
            abstract_excerpt: record.abstract_excerpt(),
            tier: record.tier().to_string(),
            retracted: record.retracted(),
            withdrawn: record.withdrawn(),
            metadata: MetadataJson(record.metadata()),
        }
    }
}

/// Serialised entry by entry, because `serde_json` maps sort their keys and
/// the metadata's key order is part of its contract.
struct MetadataJson<'a>(&'a SourceMetadata);

impl Serialize for MetadataJson<'_> {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in self.0.entries() {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum SignalsJson<'a> {
    OpenAlex {
        source_type: Option<&'a str>,
        version: Option<&'a str>,
        is_core: Option<bool>,
        listed_in: &'a [String],
        #[serde(rename = "type")]
        work_type: &'a str,
        is_retracted: bool,
    },
    Arxiv {
        journal_ref: Option<&'a str>,
        doi: Option<&'a str>,
    },
}

impl<'a> From<&'a VenueSignals> for SignalsJson<'a> {
    fn from(signals: &'a VenueSignals) -> Self {
        match signals {
            VenueSignals::OpenAlex {
                source_type,
                version,
                is_core,
                listed_in,
                work_type,
                is_retracted,
            } => Self::OpenAlex {
                source_type: source_type.as_deref(),
                version: version.as_deref(),
                is_core: *is_core,
                listed_in,
                work_type,
                is_retracted: *is_retracted,
            },
            VenueSignals::Arxiv { journal_ref, doi } => Self::Arxiv {
                journal_ref: journal_ref.as_deref(),
                doi: doi.as_deref(),
            },
        }
    }
}
