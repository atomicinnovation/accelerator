//! A document body's `## Sources` section: its cited sources, each with its
//! tier text and metadata, the lines it could not read, and the span of lines
//! it occupies, by which the body can be cut around it.

mod lines;
pub mod tier_text;

use crate::topic::sources_section::lines::Line;
use crate::topic::sources_section::tier_text::TierText;

const SOURCES_HEADING: &str = "## Sources";
const CLOSING_HEADINGS: [&str; 2] = ["# ", "## "];
const NONE_FOUND: &str = "None found.";
const ENTRY_OPENING: &str = "- [";
const TITLE_CLOSE: &str = "](";
const URL_CLOSE: char = ')';
const SEGMENT_SEPARATOR: &str = " — ";
const BLOCK_LINE_OPENING: &str = "  - ";
const KEY_SEPARATOR: &str = ": ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcesSection {
    Absent,
    DeclaredEmpty {
        span: LineSpan,
        unread: Vec<UnreadLine>,
    },
    /// The heading without `None found.`; `entries` may be empty.
    Present {
        span: LineSpan,
        entries: Vec<CitedSource>,
        unread: Vec<UnreadLine>,
    },
}

impl SourcesSection {
    /// Lines are numbered from 1 within `body`, frontmatter or not.
    #[must_use]
    pub fn parse(body: &str) -> Self {
        let mut lines = lines::numbered(body);
        let Some(heading) = lines.find(|line| line.text == SOURCES_HEADING)
        else {
            return Self::Absent;
        };
        let mut reading = SectionReading::opened_at(heading.number);
        lines
            .take_while(|line| !closes_the_section(line.text))
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

fn closes_the_section(text: &str) -> bool {
    CLOSING_HEADINGS
        .iter()
        .any(|heading| text.starts_with(heading))
}

struct SectionReading {
    start: usize,
    last: usize,
    entries: Vec<CitedSource>,
    unread: Vec<UnreadLine>,
    declares_none_found: bool,
    adjacency: BlockAdjacency,
}

enum BlockAdjacency {
    AfterEntry,
    Detached,
}

enum SectionLine {
    Ignored,
    NoneFound,
    Entry(CitedSource),
    Block(MetadataPair),
    Unread,
}

impl SectionLine {
    fn classify(text: &str) -> Self {
        if text.trim().is_empty() {
            return Self::Ignored;
        }
        if text == NONE_FOUND {
            return Self::NoneFound;
        }
        CitedSource::parse(text)
            .map(Self::Entry)
            .or_else(|| MetadataPair::parse(text).map(Self::Block))
            .unwrap_or(Self::Unread)
    }
}

impl SectionReading {
    const fn opened_at(start: usize) -> Self {
        Self {
            start,
            last: start,
            entries: Vec::new(),
            unread: Vec::new(),
            declares_none_found: false,
            adjacency: BlockAdjacency::Detached,
        }
    }

    fn read(&mut self, line: &Line<'_>) {
        self.last = line.number;
        match SectionLine::classify(line.text) {
            SectionLine::Ignored => {}
            SectionLine::NoneFound => {
                self.declares_none_found = true;
                self.adjacency = BlockAdjacency::Detached;
            }
            SectionLine::Entry(entry) => {
                self.entries.push(entry);
                self.adjacency = BlockAdjacency::AfterEntry;
            }
            SectionLine::Block(pair) => self.attach(pair, line),
            SectionLine::Unread => self.leave_unread(line),
        }
    }

    fn attach(&mut self, pair: MetadataPair, line: &Line<'_>) {
        let entry_above = match self.adjacency {
            BlockAdjacency::AfterEntry => self.entries.last_mut(),
            BlockAdjacency::Detached => None,
        };
        match entry_above {
            Some(entry) => entry.metadata.push(pair),
            None => self.leave_unread(line),
        }
    }

    fn leave_unread(&mut self, line: &Line<'_>) {
        self.unread.push(UnreadLine {
            line: line.number,
            text: line.text.to_owned(),
        });
        self.adjacency = BlockAdjacency::Detached;
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
        let mut readings =
            rest.rmatch_indices(TITLE_CLOSE).filter_map(|(close, _)| {
                let title = &rest[..close];
                let link_and_rest = &rest[close + TITLE_CLOSE.len()..];
                Self::entry_after_title(title, link_and_rest)
            });
        let rightmost = readings.next()?;
        if rightmost.has_tier_text() {
            return Some(rightmost);
        }
        Some(readings.find(Self::has_tier_text).unwrap_or(rightmost))
    }

    const fn has_tier_text(&self) -> bool {
        matches!(self.tier, TierReading::Parsed(_))
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

impl MetadataPair {
    fn parse(line: &str) -> Option<Self> {
        let (key, value) = line
            .strip_prefix(BLOCK_LINE_OPENING)?
            .split_once(KEY_SEPARATOR)?;
        let well_formed = is_block_key(key)
            && value.chars().any(|character| !character.is_whitespace());
        well_formed.then(|| Self {
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }
}

fn is_block_key(key: &str) -> bool {
    let mut characters = key.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TierReading {
    Parsed(TierText),
    /// The first segment after the link, as written: an entry that leaves
    /// out its tier has its venue read here.
    Rejected(String),
}

impl TierReading {
    #[must_use]
    pub fn of(tier_text: &str) -> Self {
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

    /// `body` must be the text the span was read from.
    #[must_use]
    pub fn split(self, body: &str) -> (&str, &str) {
        let before = &body[..lines::offset_of(body, self.start)];
        let after = &body[lines::offset_of(body, self.end)..];
        (before, after)
    }
}
