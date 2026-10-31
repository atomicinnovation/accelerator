//! The work-item ID grammar: a validated `work.id_pattern`, a validated key,
//! and the readings and renderings of IDs under them.

// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#![allow(clippy::literal_string_with_formatting_args)]

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Write as _;

const TRACKER_TOKEN: &str = "{tracker}";

const TRACKER_INNER: &str = "tracker";

const DEFAULT_WIDTH: usize = 4;

const MAX_WIDTH: usize = 19;

/// No ID is longer, so a longer value is never read as one and every walk
/// stays bounded.
const MAX_ID_BYTES: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    Empty,
    UnmatchedBrace(usize),
    NestedBrace(usize),
    UnclosedToken(usize),
    AdjacentTokens,
    BadFormatSpec(String),
    UnknownToken(String),
    HostileChar(char),
    NoNumberToken,
    WidthOverflow(usize),
    RepeatedToken,
    DigitBesideToken,
    TrackerNotSoleToken,
}

impl Display for PatternError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "pattern is empty"),
            Self::UnmatchedBrace(offset) => {
                write!(formatter, "unmatched '}}' at offset {offset}")
            }
            Self::NestedBrace(offset) => write!(
                formatter,
                "nested '{{' in token starting at offset {offset}"
            ),
            Self::UnclosedToken(offset) => {
                write!(formatter, "unclosed token starting at offset {offset}")
            }
            Self::AdjacentTokens => write!(
                formatter,
                "dynamic tokens must be separated by literal text (rule 3)"
            ),
            Self::BadFormatSpec(spec) => write!(
                formatter,
                "{{number}} format spec '{}' must match 0Nd (rule 4)",
                escaped(spec)
            ),
            Self::UnknownToken(token) => write!(
                formatter,
                "unknown token '{{{}}}' in pattern",
                escaped(token)
            ),
            Self::HostileChar(character) => write!(
                formatter,
                "literal '{}' is forbidden in patterns (rule 2)",
                escaped(&character.to_string())
            ),
            Self::NoNumberToken => write!(
                formatter,
                "pattern must contain at least one {{number}} token (rule 1)"
            ),
            Self::WidthOverflow(width) => write!(
                formatter,
                "{{number}} width {width} is too wide: a number has at most \
                 {MAX_WIDTH} digits"
            ),
            Self::RepeatedToken => write!(
                formatter,
                "a pattern holds at most one {{number}} token and one {{key}} \
                 token"
            ),
            Self::DigitBesideToken => write!(
                formatter,
                "a literal digit may not sit beside a {{key}} or {{number}} \
                 token"
            ),
            Self::TrackerNotSoleToken => write!(
                formatter,
                "`{{tracker}}` must be the only token in `work.id_pattern`"
            ),
        }
    }
}

impl std::error::Error for PatternError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    BadKeyValue(String),
}

impl Display for KeyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadKeyValue(value) => write!(
                formatter,
                "key value '{}' must match [A-Za-z][A-Za-z0-9]* (rule 5)",
                escaped(value)
            ),
        }
    }
}

impl std::error::Error for KeyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingKey;

impl Display for MissingKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "pattern references the {{key}} prefix but no value supplied"
        )
    }
}

impl std::error::Error for MissingKey {}

/// `text` safe to quote in a message: `'` escaped, and anything outside
/// printable ASCII shown as its code point.
pub(crate) fn escaped(text: &str) -> String {
    let mut out = String::new();
    for character in text.chars() {
        match character {
            '\'' => out.push_str("\\'"),
            ' '..='~' => out.push(character),
            _ => {
                let _ = write!(out, "U+{:04X}", u32::from(character));
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Key,
    Number { width: usize },
    Tracker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdPattern {
    text: String,
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdKey(String);

impl IdKey {
    /// # Errors
    ///
    /// [`KeyError::BadKeyValue`] when `text` is not a letter followed by
    /// letters and digits.
    pub fn parse(text: &str) -> Result<Self, KeyError> {
        let mut characters = text.chars();
        let well_formed = characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && characters.all(|c| c.is_ascii_alphanumeric());
        if well_formed {
            Ok(Self(text.to_owned()))
        } else {
            Err(KeyError::BadKeyValue(text.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const fn is_literal_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
}

fn is_key_token(token: &str) -> bool {
    matches!(token, "key" | "project")
}

/// The inner text of each well-formed `{...}` token, skipping escaped braces
/// and malformed tokens.
fn tokens_of(characters: &[char]) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < characters.len() {
        let next = characters.get(i + 1).copied();
        if (characters[i] == '{' && next == Some('{'))
            || (characters[i] == '}' && next == Some('}'))
        {
            i += 2;
            continue;
        }
        if characters[i] == '{' {
            if let Some(offset) = characters[i + 1..]
                .iter()
                .position(|&c| c == '}' || c == '{')
            {
                let close = i + 1 + offset;
                if characters[close] == '}' {
                    tokens.push(characters[i + 1..close].iter().collect());
                    i = close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    tokens
}

fn token_at(
    characters: &[char],
    open: usize,
) -> Result<(String, usize), PatternError> {
    let mut close = open + 1;
    loop {
        match characters.get(close) {
            Some('}') => break,
            Some('{') => return Err(PatternError::NestedBrace(open)),
            Some(_) => close += 1,
            None => return Err(PatternError::UnclosedToken(open)),
        }
    }
    let token = characters[open + 1..close].iter().collect();
    Ok((token, close + 1))
}

fn number_width(token: &str) -> Result<Option<usize>, PatternError> {
    let spec = match token.strip_prefix("number:") {
        _ if token == "number" => return Ok(Some(DEFAULT_WIDTH)),
        Some(spec) if !spec.is_empty() => spec,
        _ => return Ok(None),
    };
    let bad_spec = || PatternError::BadFormatSpec(spec.to_owned());
    let digits = spec
        .strip_prefix('0')
        .and_then(|rest| rest.strip_suffix('d'))
        .filter(|digits| {
            !digits.is_empty()
                && !digits.starts_with('0')
                && digits.chars().all(|c| c.is_ascii_digit())
        })
        .ok_or_else(bad_spec)?;
    let width = digits.parse::<usize>().unwrap_or(usize::MAX);
    if width > MAX_WIDTH {
        return Err(PatternError::WidthOverflow(width));
    }
    Ok(Some(width))
}

fn segment_of(token: &str) -> Result<Segment, PatternError> {
    if is_key_token(token) {
        return Ok(Segment::Key);
    }
    number_width(token)?.map_or_else(
        || Err(PatternError::UnknownToken(token.to_owned())),
        |width| Ok(Segment::Number { width }),
    )
}

fn refuse_a_digit_beside_a_token(
    segments: &[Segment],
) -> Result<(), PatternError> {
    let digit_beside = segments.windows(2).any(|pair| match pair {
        [Segment::Key | Segment::Number { .. }, Segment::Literal(after)] => {
            after.starts_with(|c: char| c.is_ascii_digit())
        }
        [Segment::Literal(before), Segment::Number { .. }] => {
            before.ends_with(|c: char| c.is_ascii_digit())
        }
        _ => false,
    });
    if digit_beside {
        Err(PatternError::DigitBesideToken)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiteralCase {
    Exact,
    IgnoreAscii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DigitWidth {
    Any,
    Exact,
}

/// How a walk reads a value: how literals compare, and whether the number
/// must have exactly the pattern's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Walk {
    pub(crate) literals: LiteralCase,
    pub(crate) digits: DigitWidth,
}

pub(crate) struct Reading<'a> {
    pub(crate) key: Option<&'a str>,
    pub(crate) digits: &'a str,
    rest: &'a str,
}

fn take_literal<'a>(
    rest: &'a str,
    literal: &str,
    case: LiteralCase,
) -> Option<&'a str> {
    let head = rest.get(..literal.len())?;
    let same = match case {
        LiteralCase::Exact => head == literal,
        LiteralCase::IgnoreAscii => head.eq_ignore_ascii_case(literal),
    };
    same.then(|| &rest[literal.len()..])
}

/// Every split of `rest` after a run of `run_length` leading bytes, longest
/// first.
fn longest_first(
    rest: &str,
    run_length: usize,
) -> impl Iterator<Item = (&str, &str)> {
    (1..=run_length).rev().map(move |end| rest.split_at(end))
}

fn take_key_run(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let bytes = rest.as_bytes();
    let run_length = if bytes.first().is_some_and(u8::is_ascii_alphabetic) {
        1 + bytes[1..]
            .iter()
            .take_while(|byte| byte.is_ascii_alphanumeric())
            .count()
    } else {
        0
    };
    longest_first(rest, run_length)
}

fn take_digits(rest: &str) -> impl Iterator<Item = (&str, &str)> {
    let run_length = rest.bytes().take_while(u8::is_ascii_digit).count();
    longest_first(rest, run_length)
}

fn walk_from<'a>(
    segments: &[Segment],
    rest: &'a str,
    walk: Walk,
    finished: &dyn Fn(&str) -> bool,
    key: Option<&'a str>,
    digits: Option<&'a str>,
) -> Option<Reading<'a>> {
    let Some((segment, later)) = segments.split_first() else {
        return finished(rest).then_some(Reading {
            key,
            digits: digits?,
            rest,
        });
    };
    match segment {
        Segment::Literal(literal) => walk_from(
            later,
            take_literal(rest, literal, walk.literals)?,
            walk,
            finished,
            key,
            digits,
        ),
        Segment::Key => take_key_run(rest).find_map(|(run, after)| {
            walk_from(later, after, walk, finished, Some(run), digits)
        }),
        Segment::Number { width } => take_digits(rest)
            .filter(|(run, _)| {
                walk.digits == DigitWidth::Any || run.len() == *width
            })
            .find_map(|(run, after)| {
                walk_from(later, after, walk, finished, key, Some(run))
            }),
        Segment::Tracker => None,
    }
}

impl IdPattern {
    /// # Errors
    ///
    /// The first [`PatternError`] rule `text` breaks.
    pub fn parse(text: &str) -> Result<Self, PatternError> {
        if text.is_empty() {
            return Err(PatternError::Empty);
        }
        let characters: Vec<char> = text.chars().collect();
        if tokens_of(&characters)
            .iter()
            .any(|token| token == TRACKER_INNER)
        {
            if text != TRACKER_TOKEN {
                return Err(PatternError::TrackerNotSoleToken);
            }
            return Ok(Self {
                text: text.to_owned(),
                segments: vec![Segment::Tracker],
            });
        }
        let segments = segments_of(&characters)?;
        refuse_a_digit_beside_a_token(&segments)?;
        Ok(Self {
            text: text.to_owned(),
            segments,
        })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn references_key(&self) -> bool {
        self.segments.contains(&Segment::Key)
    }

    #[must_use]
    pub fn is_tracker_owned(&self) -> bool {
        self.segments == [Segment::Tracker]
    }

    fn width(&self) -> Option<usize> {
        self.segments.iter().find_map(|segment| match segment {
            Segment::Number { width } => Some(*width),
            _ => None,
        })
    }

    /// The highest number the pattern's width holds; `None` under
    /// `{tracker}`.
    #[must_use]
    pub fn cap(&self) -> Option<u64> {
        let width = u32::try_from(self.width()?).ok()?;
        10u64.checked_pow(width).map(|bound| bound - 1)
    }

    /// # Errors
    ///
    /// [`MissingKey`] when the pattern has a key token and `key` is `None`.
    pub fn render(
        &self,
        key: Option<&IdKey>,
        number: u64,
    ) -> Result<String, MissingKey> {
        let mut out = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(literal) => out.push_str(literal),
                Segment::Key => out.push_str(key.ok_or(MissingKey)?.as_str()),
                Segment::Number { width } => {
                    let _ = write!(out, "{number:0width$}");
                }
                Segment::Tracker => {
                    let _ = write!(out, "{number}");
                }
            }
        }
        Ok(out)
    }

    /// The whole of `text` walked under `walk`.
    pub(crate) fn walk<'a>(
        &self,
        text: &'a str,
        walk: Walk,
    ) -> Option<Reading<'a>> {
        if text.len() > MAX_ID_BYTES {
            return None;
        }
        walk_from(&self.segments, text, walk, &str::is_empty, None, None)
    }

    /// `text` in exactly the shape the pattern renders: literals as
    /// written, a key run, and exactly the pattern's width of digits.
    #[must_use]
    pub fn read_exact(&self, text: &str) -> Option<IdParts> {
        let walk = Walk {
            literals: LiteralCase::Exact,
            digits: DigitWidth::Exact,
        };
        IdParts::from_reading(&self.walk(text, walk)?)
    }
}

fn segments_of(characters: &[char]) -> Result<Vec<Segment>, PatternError> {
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut last_was_token = false;
    let mut i = 0;
    while i < characters.len() {
        let character = characters[i];
        let next = characters.get(i + 1).copied();
        let escaped_brace =
            matches!((character, next), ('{', Some('{')) | ('}', Some('}')));
        if escaped_brace {
            return Err(PatternError::HostileChar(character));
        }
        if character == '}' {
            return Err(PatternError::UnmatchedBrace(i));
        }
        if character == '{' {
            let (token, after) = token_at(characters, i)?;
            if last_was_token {
                return Err(PatternError::AdjacentTokens);
            }
            let segment = segment_of(&token)?;
            if segments.iter().any(|held| same_kind(held, &segment)) {
                return Err(PatternError::RepeatedToken);
            }
            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            }
            segments.push(segment);
            last_was_token = true;
            i = after;
            continue;
        }
        if !is_literal_character(character) {
            return Err(PatternError::HostileChar(character));
        }
        literal.push(character);
        last_was_token = false;
        i += 1;
    }
    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }
    if !segments
        .iter()
        .any(|segment| matches!(segment, Segment::Number { .. }))
    {
        return Err(PatternError::NoNumberToken);
    }
    Ok(segments)
}

const fn same_kind(left: &Segment, right: &Segment) -> bool {
    matches!(
        (left, right),
        (Segment::Key, Segment::Key)
            | (Segment::Number { .. }, Segment::Number { .. })
    )
}

/// An ID's key, its digits as written, and its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdParts {
    key: Option<String>,
    digits: String,
    number: u64,
}

impl IdParts {
    fn from_reading(reading: &Reading<'_>) -> Option<Self> {
        Some(Self {
            key: reading.key.map(str::to_owned),
            digits: reading.digits.to_owned(),
            number: reading.digits.parse().ok()?,
        })
    }

    /// `text` read leniently: literals ignoring ASCII case, and a number of
    /// any width.
    #[must_use]
    pub fn parse(pattern: &IdPattern, text: &str) -> Option<Self> {
        let walk = Walk {
            literals: LiteralCase::IgnoreAscii,
            digits: DigitWidth::Any,
        };
        Self::from_reading(&pattern.walk(text, walk)?)
    }

    /// The ID leading `filename`, read as [`Self::parse`] reads a value and
    /// followed by the `-` that starts the slug, with where the slug starts.
    #[must_use]
    pub fn parse_prefix(
        pattern: &IdPattern,
        filename: &str,
    ) -> Option<(Self, usize)> {
        let ascii_length = filename
            .bytes()
            .take(MAX_ID_BYTES)
            .take_while(u8::is_ascii)
            .count();
        let head = &filename[..ascii_length];
        let walk = Walk {
            literals: LiteralCase::IgnoreAscii,
            digits: DigitWidth::Any,
        };
        let reading = walk_from(
            &pattern.segments,
            head,
            walk,
            &|rest| rest.starts_with('-'),
            None,
            None,
        )?;
        let slug_start = head.len() - reading.rest.len() + 1;
        Some((Self::from_reading(&reading)?, slug_start))
    }

    #[must_use]
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    #[must_use]
    pub fn digits(&self) -> &str {
        &self.digits
    }

    #[must_use]
    pub const fn number(&self) -> u64 {
        self.number
    }

    /// Whether both name one ID: keys equal ignoring ASCII case, and equal
    /// numbers.
    #[must_use]
    pub fn same_id(&self, other: &Self) -> bool {
        let same_key = match (&self.key, &other.key) {
            (Some(mine), Some(theirs)) => mine.eq_ignore_ascii_case(theirs),
            (None, None) => true,
            _ => false,
        };
        same_key && self.number == other.number
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::time::Duration;
    use std::time::Instant;

    use super::IdKey;
    use super::IdParts;
    use super::IdPattern;
    use super::KeyError;
    use super::MissingKey;
    use super::PatternError;

    const PROMPTLY: Duration = Duration::from_millis(100);

    fn pattern(text: &str) -> IdPattern {
        IdPattern::parse(text).expect("a well-formed pattern")
    }

    fn key(text: &str) -> IdKey {
        IdKey::parse(text).expect("a well-formed key")
    }

    fn parts(text: &str, under: &str) -> Option<(Option<String>, u64)> {
        IdParts::parse(&pattern(under), text)
            .map(|parts| (parts.key().map(str::to_owned), parts.number()))
    }

    fn prefix_number(filename: &str, under: &str) -> Option<u64> {
        IdParts::parse_prefix(&pattern(under), filename)
            .map(|(parts, _)| parts.number())
    }

    #[test]
    fn a_bare_number_token_has_width_four() {
        assert_eq!(pattern("{number}").cap(), Some(9999));
        assert_eq!(pattern("{number:06d}").cap(), Some(999_999));
        assert_eq!(pattern("{number}").render(None, 7), Ok("0007".to_owned()));
    }

    #[test]
    fn a_malformed_pattern_cannot_be_parsed() {
        assert_eq!(
            IdPattern::parse("{number:4d}"),
            Err(PatternError::BadFormatSpec("4d".to_owned()))
        );
        assert_eq!(
            IdPattern::parse("{bogus}-{number}"),
            Err(PatternError::UnknownToken("bogus".to_owned()))
        );
        let overflow = IdPattern::parse("{number:020d}");
        assert_eq!(overflow, Err(PatternError::WidthOverflow(20)));
        assert!(overflow
            .unwrap_err()
            .to_string()
            .contains("at most 19 digits"),);
    }

    #[test]
    fn a_character_outside_the_literal_set_is_hostile() {
        for literal in [
            "'", "\t", " ", "$", "`", ";", "~", "\u{202E}", "\u{009B}",
            "\u{200B}", "é",
        ] {
            let text = format!("x{literal}{{number:04d}}");
            let hostile = literal.chars().next().unwrap();
            assert_eq!(
                IdPattern::parse(&text),
                Err(PatternError::HostileChar(hostile)),
                "{text:?}"
            );
        }
        assert_eq!(
            IdPattern::parse("{{x}}{number}"),
            Err(PatternError::HostileChar('{'))
        );
        assert_eq!(
            IdPattern::parse("{{tracker}}"),
            Err(PatternError::HostileChar('{'))
        );
        for text in ["x.{number:04d}", "x_{number:04d}", "x-{number:04d}"] {
            assert!(IdPattern::parse(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn a_hostile_character_is_shown_escaped() {
        let message =
            |text: &str| IdPattern::parse(text).unwrap_err().to_string();
        assert!(message("x'{number:04d}").contains("'\\''"));
        assert!(message("x\t{number:04d}").contains("U+0009"));
        assert!(message("x\0{number:04d}").contains("U+0000"));
        assert!(message("x\u{202E}{number:04d}").contains("U+202E"));
        let esc_messages = [
            KeyError::BadKeyValue("P\u{1b}".to_owned()).to_string(),
            IdKey::parse("P\u{1b}").unwrap_err().to_string(),
            message("{\u{1b}}-{number:04d}"),
            message("{number:\u{1b}}"),
        ];
        for text in esc_messages {
            assert!(text.contains("U+001B"), "{text}");
            assert!(text.chars().all(|c| matches!(c, ' '..='~')), "{text:?}");
        }
    }

    #[test]
    fn a_repeated_token_is_refused() {
        for text in ["{number:04d}-{number:04d}", "{key}-{key}-{number:04d}"] {
            assert_eq!(
                IdPattern::parse(text),
                Err(PatternError::RepeatedToken),
                "{text}"
            );
        }
    }

    #[test]
    fn a_digit_beside_a_token_is_refused() {
        for text in [
            "{key}1{number:04d}",
            "{key}0{number:04d}",
            "{number:04d}-{key}1",
            "v2{number:03d}",
            "{key}-1{number:04d}",
            "{number:04d}1",
        ] {
            assert_eq!(
                IdPattern::parse(text),
                Err(PatternError::DigitBesideToken),
                "{text}"
            );
        }
        for text in ["v_{number:03d}", "{key}-{number:04d}", "{number:04d}-x1"]
        {
            assert!(IdPattern::parse(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn a_key_value_must_match_rule_five() {
        for text in ["1PP", "P-P", "../x", ""] {
            assert_eq!(
                IdKey::parse(text),
                Err(KeyError::BadKeyValue(text.to_owned()))
            );
        }
        for text in ["PP", "pp", "X1"] {
            assert_eq!(
                IdKey::parse(text).map(|key| key.0),
                Ok(text.to_owned())
            );
        }
    }

    #[test]
    fn render_needs_a_key_only_under_a_key_pattern() {
        assert_eq!(
            pattern("{key}-{number:04d}").render(None, 1),
            Err(MissingKey)
        );
        assert_eq!(
            pattern("{key}-{number:04d}").render(Some(&key("PP")), 1),
            Ok("PP-0001".to_owned())
        );
        assert_eq!(
            pattern("{number:04d}").render(None, 1),
            Ok("0001".to_owned())
        );
    }

    #[test]
    fn id_parts_read_at_any_width_ignoring_case() {
        for (text, read_key) in
            [("pp-400", "pp"), ("PP-0400", "PP"), ("Pp-00400", "Pp")]
        {
            assert_eq!(
                parts(text, "{key}-{number:04d}"),
                Some((Some(read_key.to_owned()), 400)),
                "{text}"
            );
        }
    }

    #[test]
    fn a_value_that_cannot_be_walked_does_not_parse() {
        let keyed = "{key}-{number:04d}";
        assert_eq!(parts("PP\u{2013}0400", keyed), None);
        assert_eq!(parts("PP-", keyed), None);
        assert_eq!(parts("PP-99999999999999999999", keyed), None);
    }

    #[test]
    fn an_oversized_value_is_not_an_id_and_returns_promptly() {
        let keyed = pattern("{key}-{number:04d}");
        let oversized = format!("{}-{}", "P".repeat(200), "1".repeat(55));
        assert_eq!(oversized.len(), 256);
        let unmatched = "P".repeat(255);
        for text in [oversized.as_str(), unmatched.as_str()] {
            let started = Instant::now();
            assert_eq!(IdParts::parse(&keyed, text), None);
            assert_eq!(keyed.read_exact(text), None);
            assert!(started.elapsed() < PROMPTLY);
        }
    }

    #[test]
    fn a_long_filename_is_read_by_its_prefix() {
        let slug = "\u{65E5}".repeat(100);
        let filename = format!("0400-{slug}.md");
        assert_eq!(prefix_number(&filename, "{number:04d}"), Some(400));

        let started = Instant::now();
        assert_eq!(prefix_number(&"P".repeat(300), "{key}-{number:04d}"), None);
        assert!(started.elapsed() < PROMPTLY);
    }

    #[test]
    fn two_readings_are_the_same_id_when_key_and_number_agree() {
        let keyed = pattern("{key}-{number:04d}");
        let read = |text: &str| IdParts::parse(&keyed, text).unwrap();
        let held = read("PP-0400");
        assert!(read("pp-400").same_id(&held));
        assert!(read("PP-0400").same_id(&held));
        assert!(!read("XY-400").same_id(&held));
        assert!(!read("PP-401").same_id(&held));
    }

    #[test]
    fn a_filename_prefix_needs_the_slug_delimiter() {
        assert_eq!(
            IdParts::parse_prefix(&pattern("{number:04d}"), "0400-x.md").map(
                |(parts, slug_start)| (parts.digits().to_owned(), slug_start)
            ),
            Some(("0400".to_owned(), 5))
        );
        assert_eq!(prefix_number("0400.md", "{number:04d}"), None);
        assert_eq!(prefix_number("0400x-y.md", "{number:04d}"), None);
    }

    #[test]
    fn the_filename_reading_folds_only_ascii_letters() {
        assert_eq!(
            prefix_number("\u{212A}-0500-x.md", "{key}-{number:04d}"),
            None
        );
    }

    #[test]
    fn render_keeps_literal_text() {
        assert_eq!(
            pattern("v{number:03d}").render(None, 1),
            Ok("v001".to_owned())
        );
    }

    #[test]
    fn a_key_before_an_alphanumeric_literal_reads_as_the_regex_does() {
        let under = "{key}v{number:03d}";
        assert_eq!(parts("PPv001", under), Some((Some("PP".to_owned()), 1)));
        assert_eq!(parts("PPvv001", under), Some((Some("PPv".to_owned()), 1)));
    }

    #[test]
    fn read_exact_needs_the_literals_as_written_and_the_exact_width() {
        let versioned = pattern("v{number:03d}");
        assert!(versioned.read_exact("v001").is_some());
        assert!(versioned.read_exact("V001").is_none());
        assert!(versioned.read_exact("v0001").is_none());
        assert!(versioned.read_exact("v000").is_some());
    }

    #[test]
    fn the_tracker_pattern_stands_alone_and_has_no_cap() {
        let tracker = pattern("{tracker}");
        assert!(tracker.is_tracker_owned());
        assert_eq!(tracker.cap(), None);
        assert_eq!(
            IdPattern::parse("{tracker}-{number:04d}"),
            Err(PatternError::TrackerNotSoleToken)
        );
    }
}
