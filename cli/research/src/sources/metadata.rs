//! The `key: value` metadata a Sources entry may carry, in the value forms
//! consumers can rely on.

pub mod date;

use crate::sources::metadata::date::PartialDate;
use crate::sources::request::ArxivId;
use crate::sources::request::Doi;
use crate::sources::request::WorkId;

/// The keys consumers can rely on when present, in the order a map lists
/// them.
pub const RECOMMENDED_KEYS: [&str; 19] = [
    "doi",
    "arxiv_id",
    "openalex_id",
    "pmid",
    "issn",
    "published",
    "updated",
    "accessed",
    "type",
    "authors",
    "publisher",
    "volume",
    "issue",
    "pages",
    "license",
    "journal_ref",
    "primary_category",
    "oa_status",
    "pdf_url",
];

/// A source's metadata, recorded only through typed recorders so every value
/// is in its key's value form.
///
/// The first value recorded for a key wins, and entries list in
/// [`RECOMMENDED_KEYS`] order whatever order they were recorded in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMetadata {
    entries: Vec<(String, String)>,
}

impl SourceMetadata {
    pub fn record_doi(&mut self, doi: &Doi) {
        self.record("doi", doi.canonical());
    }

    pub fn record_arxiv_id(&mut self, id: &ArxivId) {
        self.record("arxiv_id", id.to_string());
    }

    pub fn record_openalex_id(&mut self, id: &WorkId) {
        self.record("openalex_id", id.as_str().to_owned());
    }

    pub fn record_published(&mut self, date: PartialDate) {
        self.record("published", date.to_string());
    }

    pub fn record_updated(&mut self, date: PartialDate) {
        self.record("updated", date.to_string());
    }

    pub fn record_accessed(&mut self, date: PartialDate) {
        self.record("accessed", date.to_string());
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    fn record(&mut self, key: &'static str, value: String) {
        if self.entries.iter().any(|(present, _)| present == key) {
            return;
        }
        let at = self
            .entries
            .partition_point(|(present, _)| rank(present) < rank(key));
        self.entries.insert(at, (key.to_owned(), value));
    }
}

fn rank(key: &str) -> usize {
    RECOMMENDED_KEYS
        .iter()
        .position(|recommended| *recommended == key)
        .unwrap_or(RECOMMENDED_KEYS.len())
}
