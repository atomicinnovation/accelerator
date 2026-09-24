//! What a call prints: compact JSON on stdout for records and unavailability,
//! and one non-secret stderr line for a call that did not deliver.

use research::fetch::Cause;
use research::fetch::FetchOutcome;
use research::record::Record;
use research::record::VenueSignals;
use research::request::Family;
use serde::Serialize;

use crate::fetch_command::Fetched;

/// The stdout document, or `None` for a failed call, which prints only its
/// diagnostics.
pub fn document(fetched: &Fetched) -> Option<String> {
    let document = match &fetched.outcome {
        FetchOutcome::Records(records) => Document::Ok {
            records: records.iter().map(RecordJson::from).collect(),
        },
        FetchOutcome::Unavailable(unavailable) => Document::Unavailable {
            source: fetched.family.code(),
            reason: unavailable.reason().code(),
            authenticated: (fetched.family == Family::OpenAlex)
                .then_some(fetched.authenticated),
            cause: unavailable.cause().map(Cause::code),
        },
        FetchOutcome::Failed(_) => return None,
    };
    serde_json::to_string(&document).ok()
}

/// The line naming the source, verb, final status and attempts of a call
/// that did not deliver records.
pub fn summary(fetched: &Fetched) -> Option<String> {
    let status = match &fetched.outcome {
        FetchOutcome::Records(_) => return None,
        FetchOutcome::Unavailable(unavailable) => {
            format!("unavailable ({})", unavailable.reason().code())
        }
        FetchOutcome::Failed(_) => "failed".to_owned(),
    };
    let attempts = match fetched.attempts {
        1 => "1 attempt".to_owned(),
        count => format!("{count} attempts"),
    };
    Some(format!(
        "research fetch: {} {} {status} after {attempts}",
        fetched.family.code(),
        fetched.verb.code(),
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
        }
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
