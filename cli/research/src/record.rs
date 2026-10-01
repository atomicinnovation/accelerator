//! The normalised shape every source's results are reported in.

use std::fmt;

/// The standing of the venue a record was published through. Derived here so
/// no researcher ever judges it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    One,
    Two,
    Three,
}

impl fmt::Display for Tier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::One => "tier-1",
            Self::Two => "tier-2",
            Self::Three => "tier-3",
        })
    }
}

/// The raw inputs a record's tier was derived from, reported so a reader can
/// check the derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VenueSignals {
    OpenAlex {
        source_type: Option<String>,
        version: Option<String>,
        is_core: Option<bool>,
        listed_in: Vec<String>,
        work_type: String,
        is_retracted: bool,
    },
    Arxiv {
        journal_ref: Option<String>,
        doi: Option<String>,
    },
}

/// One scholarly work, citable by its `url`.
///
/// Built only by a source's normaliser, so every record's tier is the one the
/// rules derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub(crate) title: Option<String>,
    pub(crate) authors: Vec<String>,
    pub(crate) url: String,
    pub(crate) venue: Option<String>,
    pub(crate) venue_signals: VenueSignals,
    pub(crate) abstract_excerpt: Option<String>,
    pub(crate) tier: Tier,
    pub(crate) retracted: bool,
    pub(crate) withdrawn: bool,
}

impl Record {
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn authors(&self) -> &[String] {
        &self.authors
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn venue(&self) -> Option<&str> {
        self.venue.as_deref()
    }

    pub const fn venue_signals(&self) -> &VenueSignals {
        &self.venue_signals
    }

    pub fn abstract_excerpt(&self) -> Option<&str> {
        self.abstract_excerpt.as_deref()
    }

    pub const fn tier(&self) -> Tier {
        self.tier
    }

    pub const fn retracted(&self) -> bool {
        self.retracted
    }

    pub const fn withdrawn(&self) -> bool {
        self.withdrawn
    }
}

const EXCERPT_CHARACTERS: usize = 600;
const ELLIPSIS: char = '…';

/// Shortens an abstract to at most 600 characters, ellipsis included,
/// breaking at the last word boundary that fits.
pub(crate) fn excerpt(text: &str) -> String {
    if text.chars().count() <= EXCERPT_CHARACTERS {
        return text.to_owned();
    }
    let kept = EXCERPT_CHARACTERS - 1;
    let head_end = text
        .char_indices()
        .nth(kept)
        .map_or(text.len(), |(index, _)| index);
    let head = &text[..head_end];
    let breaks_after_head = text[head_end..].starts_with(char::is_whitespace);
    let cut = if breaks_after_head {
        head
    } else {
        head.rfind(char::is_whitespace)
            .map_or(head, |boundary| &head[..boundary])
    };
    let mut excerpt = cut.trim_end().to_owned();
    excerpt.push(ELLIPSIS);
    excerpt
}
