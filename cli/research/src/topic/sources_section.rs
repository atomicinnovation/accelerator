//! A document body's `## Sources` section: its cited sources, each with its
//! tier text, and the lines it could not read.

mod lines;
pub mod tier_text;

use crate::topic::sources_section::lines::Line;
use crate::topic::sources_section::tier_text::TierText;

const SOURCES_HEADING: &str = "## Sources";
const LEVEL_TWO_HEADING: &str = "## ";
const NONE_FOUND: &str = "None found.";
const ENTRY_OPENING: &str = "- [";
const TITLE_CLOSE: &str = "](";
const URL_CLOSE: char = ')';
const SEGMENT_SEPARATOR: &str = " — ";

/// Lines are numbered from 1 within the text parsed, frontmatter or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcesSection {
    Absent,
    DeclaredEmpty {
        span: LineSpan,
        unread: Vec<UnreadLine>,
    },
    Present {
        span: LineSpan,
        entries: Vec<CitedSource>,
        unread: Vec<UnreadLine>,
    },
}

impl SourcesSection {
    #[must_use]
    pub fn parse(body: &str) -> Self {
        let mut lines = lines::numbered(body);
        let Some(heading) = lines.find(|line| line.text == SOURCES_HEADING)
        else {
            return Self::Absent;
        };
        let mut reading = SectionReading::opened_at(heading.number);
        lines
            .take_while(|line| !line.text.starts_with(LEVEL_TWO_HEADING))
            .for_each(|line| reading.read(&line));
        reading.closed()
    }

    #[must_use]
    pub fn entries(&self) -> &[CitedSource] {
        match self {
            Self::Present { entries, .. } => entries,
            Self::Absent | Self::DeclaredEmpty { .. } => &[],
        }
    }

    #[must_use]
    pub fn unread(&self) -> &[UnreadLine] {
        match self {
            Self::Present { unread, .. }
            | Self::DeclaredEmpty { unread, .. } => unread,
            Self::Absent => &[],
        }
    }

    #[must_use]
    pub const fn span(&self) -> Option<LineSpan> {
        match self {
            Self::Present { span, .. } | Self::DeclaredEmpty { span, .. } => {
                Some(*span)
            }
            Self::Absent => None,
        }
    }
}

struct SectionReading {
    start: usize,
    last: usize,
    entries: Vec<CitedSource>,
    unread: Vec<UnreadLine>,
    declares_none_found: bool,
}

impl SectionReading {
    const fn opened_at(start: usize) -> Self {
        Self {
            start,
            last: start,
            entries: Vec::new(),
            unread: Vec::new(),
            declares_none_found: false,
        }
    }

    fn read(&mut self, line: &Line<'_>) {
        self.last = line.number;
        if line.text.trim().is_empty() {
            return;
        }
        if line.text == NONE_FOUND {
            self.declares_none_found = true;
        } else if let Some(entry) = CitedSource::parse(line.text) {
            self.entries.push(entry);
        } else {
            self.unread.push(UnreadLine {
                line: line.number,
                text: line.text.to_owned(),
            });
        }
    }

    fn closed(self) -> SourcesSection {
        let span = LineSpan {
            start: self.start,
            end: self.last + 1,
        };
        if self.entries.is_empty() && self.declares_none_found {
            SourcesSection::DeclaredEmpty {
                span,
                unread: self.unread,
            }
        } else {
            SourcesSection::Present {
                span,
                entries: self.entries,
                unread: self.unread,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitedSource {
    pub title: String,
    pub url: String,
    pub tier: TierReading,
    pub venue: Option<String>,
    pub metadata: Vec<MetadataPair>,
}

impl CitedSource {
    fn parse(line: &str) -> Option<Self> {
        let rest = line.strip_prefix(ENTRY_OPENING)?;
        rest.rmatch_indices(TITLE_CLOSE).find_map(|(close, _)| {
            let title = &rest[..close];
            let link_and_rest = &rest[close + TITLE_CLOSE.len()..];
            Self::entry_after_title(title, link_and_rest)
        })
    }

    fn entry_after_title(title: &str, link_and_rest: &str) -> Option<Self> {
        if title.is_empty() {
            return None;
        }
        let (url, rest) = url_and_rest(link_and_rest)?;
        let (tier_text, venue) = tier_and_venue(rest)?;
        Some(Self {
            title: title.to_owned(),
            url: url.to_owned(),
            tier: TierReading::of(tier_text),
            venue: venue.map(str::to_owned),
            metadata: Vec::new(),
        })
    }
}

fn url_and_rest(link_and_rest: &str) -> Option<(&str, &str)> {
    let (url, rest) = link_and_rest.split_once(URL_CLOSE)?;
    let well_formed = !url.is_empty() && !url.chars().any(char::is_whitespace);
    well_formed.then_some((url, rest))
}

fn tier_and_venue(rest: &str) -> Option<(&str, Option<&str>)> {
    let segments = rest
        .strip_prefix(SEGMENT_SEPARATOR)
        .filter(|segments| !segments.is_empty())?;
    Some(match segments.split_once(SEGMENT_SEPARATOR) {
        Some((tier_text, venue)) => {
            (tier_text, Some(venue).filter(|venue| !venue.is_empty()))
        }
        None => (segments, None),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierReading {
    Parsed(TierText),
    Rejected(String),
}

impl TierReading {
    fn of(tier_text: &str) -> Self {
        TierText::parse(tier_text).map_or_else(
            |rejection| Self::Rejected(rejection.into_text()),
            Self::Parsed,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadLine {
    pub line: usize,
    pub text: String,
}

/// From the heading's line up to, not including, `end`, the first line after
/// the section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineSpan {
    start: usize,
    end: usize,
}

impl LineSpan {
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
}
