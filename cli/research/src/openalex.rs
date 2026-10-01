//! OpenAlex works as the decoder hands them over, and the record each
//! becomes.
//!
//! Work types and source types are open sets upstream, so they stay strings
//! and are only ever read through the predicates here.

use crate::record::excerpt;
use crate::record::Record;
use crate::record::VenueSignals;
use crate::request::Doi;
use crate::request::WorkId;
use crate::tier::openalex_tier;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Work {
    pub id: String,
    pub doi: Option<String>,
    pub title: Option<String>,
    pub work_type: String,
    pub authors: Vec<String>,
    pub primary_location: Option<Location>,
    pub is_retracted: bool,
    pub abstract_inverted_index: Option<Vec<(String, Vec<usize>)>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Location {
    pub version: Option<String>,
    pub source: Option<Source>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    pub display_name: Option<String>,
    pub source_type: Option<String>,
    pub is_core: bool,
    pub listed_in: Vec<String>,
}

impl Work {
    pub(crate) fn is_peer_reviewed_publication(&self) -> bool {
        !self.is_preprint()
            && self.primary_source().is_some_and(|source| {
                (source.is_scholarly_venue()
                    && self.has_peer_reviewed_version())
                    || source.is_curated()
            })
    }

    pub(crate) fn is_early_version(&self) -> bool {
        self.is_preprint()
            || self.primary_source().is_some_and(|source| {
                source.is_repository()
                    || (source.is_scholarly_venue()
                        && self.version() == Some("submittedVersion"))
            })
    }

    fn is_preprint(&self) -> bool {
        self.work_type == "preprint"
    }

    fn has_peer_reviewed_version(&self) -> bool {
        matches!(self.version(), Some("publishedVersion" | "acceptedVersion"))
    }

    fn version(&self) -> Option<&str> {
        self.primary_location.as_ref()?.version.as_deref()
    }

    fn primary_source(&self) -> Option<&Source> {
        self.primary_location.as_ref()?.source.as_ref()
    }

    fn citation_url(&self, work: &WorkId) -> String {
        self.doi
            .as_deref()
            .map(|doi| doi.strip_prefix("https://doi.org/").unwrap_or(doi))
            .and_then(Doi::parse)
            .map_or_else(|| work.url(), |doi| doi.url())
    }

    fn venue_signals(&self) -> VenueSignals {
        let source = self.primary_source();
        VenueSignals::OpenAlex {
            source_type: source.and_then(|source| source.source_type.clone()),
            version: self.version().map(str::to_owned),
            is_core: source.map(|source| source.is_core),
            listed_in: source
                .map(|source| source.listed_in.clone())
                .unwrap_or_default(),
            work_type: self.work_type.clone(),
            is_retracted: self.is_retracted,
        }
    }
}

impl Source {
    fn is_scholarly_venue(&self) -> bool {
        matches!(self.source_type.as_deref(), Some("journal" | "conference"))
    }

    fn is_repository(&self) -> bool {
        self.source_type.as_deref() == Some("repository")
    }

    fn is_curated(&self) -> bool {
        self.is_core || self.listed_in.iter().any(|list| list == "medline")
    }
}

/// OpenAlex ships abstracts as `{word: [positions]}`; this lays the words
/// back out in position order.
pub fn abstract_from_inverted_index(index: &[(String, Vec<usize>)]) -> String {
    let mut placed = index
        .iter()
        .flat_map(|(word, positions)| {
            positions
                .iter()
                .map(move |position| (*position, word.as_str()))
        })
        .collect::<Vec<_>>();
    placed.sort_by_key(|(position, _)| *position);
    placed
        .into_iter()
        .map(|(_, word)| word)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The record a work becomes, or `None` when its ID is not an OpenAlex work
/// and so cannot be cited.
pub fn normalise(work: &Work) -> Option<Record> {
    let id = WorkId::from_openalex(&work.id)?;
    Some(Record {
        title: work.title.clone(),
        authors: work.authors.clone(),
        url: work.citation_url(&id),
        venue: work
            .primary_source()
            .and_then(|source| source.display_name.clone()),
        venue_signals: work.venue_signals(),
        abstract_excerpt: work
            .abstract_inverted_index
            .as_deref()
            .map(abstract_from_inverted_index)
            .filter(|text| !text.is_empty())
            .map(|text| excerpt(&text)),
        tier: openalex_tier(work),
        retracted: work.is_retracted,
        withdrawn: false,
    })
}
