//! What Crossref holds for a DOI: the venue a journal or proceedings article
//! was published in, and the publisher of any work.

use std::fmt;

use crate::sources::metadata::collapsed_whitespace;
use crate::sources::metadata::Publisher;
use crate::sources::metadata::SourceMetadata;

/// Crossref's types whose container is a peer-reviewed venue. A chapter's
/// container is its book, and a posted preprint's its server.
const VENUE_TYPES: [&str; 2] = ["journal-article", "proceedings-article"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossrefWork {
    pub kind: Option<String>,
    pub container_titles: Vec<String>,
    pub publisher: Option<String>,
}

/// Why a body Crossref delivered is not a work message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableWork(String);

impl UnreadableWork {
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

impl fmt::Display for UnreadableWork {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The journal or proceedings a work was published in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Venue(String);

impl Venue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What `doi metadata` reports; the default is a DOI Crossref holds no
/// record for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoiMetadata {
    pub metadata: SourceMetadata,
    pub venue: Option<Venue>,
}

pub fn normalise(work: &CrossrefWork) -> DoiMetadata {
    let mut metadata = SourceMetadata::default();
    if let Some(publisher) =
        work.publisher.as_deref().and_then(Publisher::parse)
    {
        metadata.record_publisher(&publisher);
    }
    DoiMetadata {
        metadata,
        venue: venue_of(work),
    }
}

fn venue_of(work: &CrossrefWork) -> Option<Venue> {
    let is_venue_type = work
        .kind
        .as_deref()
        .is_some_and(|kind| VENUE_TYPES.contains(&kind));
    if !is_venue_type {
        return None;
    }
    collapsed_whitespace(work.container_titles.first()?).map(Venue)
}

/// An address Crossref can reach the caller at, sent so that it serves the
/// call from its polite pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactAddress(String);

/// A configured contact that could not be a working address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Implausible;

impl ContactAddress {
    /// The address travels inside a `User-Agent` comment, so it is held to
    /// visible ASCII with nothing that could end or escape that comment.
    ///
    /// # Errors
    ///
    /// [`Implausible`] when the trimmed value is not a plausible address.
    pub fn parse(raw: &str) -> Result<Option<Self>, Implausible> {
        let address = raw.trim();
        if address.is_empty() {
            return Ok(None);
        }
        if is_plausible(address) {
            Ok(Some(Self(address.to_owned())))
        } else {
            Err(Implausible)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_plausible(address: &str) -> bool {
    let fits_a_comment = address.chars().all(|character| {
        character.is_ascii_graphic() && !matches!(character, '(' | ')' | '\\')
    });
    let Some((local, domain)) = address.split_once('@') else {
        return false;
    };
    fits_a_comment
        && !local.is_empty()
        && !local.contains(':')
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}
