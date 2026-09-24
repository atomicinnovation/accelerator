//! arXiv feed entries and `arXivRaw` versions as the decoder hands them
//! over, the withdrawal rules, and the record each entry becomes.

use crate::record::excerpt;
use crate::record::Record;
use crate::record::VenueSignals;
use crate::request::ArxivId;
use crate::tier::arxiv_tier;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub authors: Vec<String>,
    pub comment: Option<String>,
    pub journal_ref: Option<String>,
    pub doi: Option<String>,
    pub primary_category: Option<String>,
}

impl Entry {
    /// The identifier the feed reported, version included, or `None` when
    /// the entry's ID is not an arXiv abstract URL.
    pub fn arxiv_id(&self) -> Option<ArxivId> {
        ArxivId::from_abstract_url(&self.id)
    }

    pub fn is_withdrawal_candidate(&self) -> bool {
        self.comment.as_deref().is_some_and(is_withdrawal_candidate)
    }
}

/// One `<version>` of an `arXivRaw` record, in document order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawVersion {
    pub size: String,
    pub source_type: Option<String>,
}

/// Whether a comment opens by declaring the work withdrawn. A candidate is
/// only a suspicion: comments are free text, so confirmation needs the
/// `arXivRaw` record.
///
/// Runs of whitespace compare as one space, because feed comments are
/// line-wrapped.
pub fn is_withdrawal_candidate(comment: &str) -> bool {
    let normalised = comment
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if normalised.starts_with("withdrawn") {
        return true;
    }
    let Some(rest) = normalised.strip_prefix("this ") else {
        return false;
    };
    let Some(rest) = ["paper ", "article ", "submission ", "manuscript "]
        .iter()
        .find_map(|noun| rest.strip_prefix(noun))
    else {
        return false;
    };
    ["has been withdrawn", "is withdrawn"]
        .iter()
        .any(|phrase| rest.starts_with(phrase))
}

/// arXiv marks a withdrawal as a new, empty version of source type `I`.
pub fn latest_version_withdrawn(versions: &[RawVersion]) -> bool {
    versions.last().is_some_and(|latest| {
        latest.size == "0kb" && latest.source_type.as_deref() == Some("I")
    })
}

/// The record an entry becomes, or `None` when its ID is not an arXiv
/// abstract URL and so cannot be cited.
pub fn normalise(entry: &Entry, withdrawn: bool) -> Option<Record> {
    let id = entry.arxiv_id()?;
    Some(Record {
        title: Some(entry.title.clone()),
        authors: entry.authors.clone(),
        url: id.abstract_url(),
        venue: Some(entry.primary_category.as_ref().map_or_else(
            || "arXiv".to_owned(),
            |category| format!("arXiv ({category})"),
        )),
        venue_signals: VenueSignals::Arxiv {
            journal_ref: entry.journal_ref.clone(),
            doi: entry.doi.clone(),
        },
        abstract_excerpt: entry
            .summary
            .as_deref()
            .filter(|summary| !summary.is_empty())
            .map(excerpt),
        tier: arxiv_tier(withdrawn),
        retracted: false,
        withdrawn,
    })
}
