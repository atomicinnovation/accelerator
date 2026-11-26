//! `markdown::linear::escape_for_push` against what Linear did to the bodies
//! in `tests/fixtures/linear-rewrites` and `linear-autolink-probes`: every
//! token Linear linked and every lone number it listed is escaped, and every
//! other escape is one the probes show Linear stores as plain text.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod captures;

use std::collections::BTreeSet;
use std::ops::Range;

use captures::{fixture, fragments, lines, read_back, Fragments, TestError};
use document::markdown::{events, MarkdownEvent, MarkdownTag};
use markdown::linear::{dotted_tokens, escape_for_push};
use tracker_test_support::linear_leaves_bare;

struct Corpus {
    name: &'static str,
    pushed: Fragments,
    read_back: Fragments,
}

impl Corpus {
    fn capture() -> Result<Self, TestError> {
        Ok(Self {
            name: "linear-rewrites",
            pushed: fragments(&fixture("linear-rewrites/pushed.md")?, "## "),
            read_back: fragments(
                &read_back("linear-rewrites/read-back.json")?,
                "## ",
            ),
        })
    }

    fn probes() -> Result<Vec<Self>, TestError> {
        let probes = |name, pushed, read_back_path| -> Result<_, TestError> {
            Ok(Self {
                name,
                pushed: fragments(&fixture(pushed)?, "#### "),
                read_back: fragments(&read_back(read_back_path)?, "#### "),
            })
        };
        Ok(vec![
            probes(
                "probes, created",
                "linear-autolink-probes/probes.md",
                "linear-autolink-probes/probes.created.json",
            )?,
            probes(
                "probes, updated",
                "linear-autolink-probes/probes.md",
                "linear-autolink-probes/probes.updated.json",
            )?,
            probes(
                "follow-ups",
                "linear-autolink-probes/follow-ups.md",
                "linear-autolink-probes/follow-ups.read-back.json",
            )?,
            probes(
                "split tokens",
                "linear-autolink-probes/split-tokens.md",
                "linear-autolink-probes/split-tokens.read-back.json",
            )?,
            Self {
                name: "tld-sweep",
                pushed: lines(&fixture("linear-autolink-probes/tld-sweep.md")?),
                read_back: lines(&read_back(
                    "linear-autolink-probes/tld-sweep.read-back.json",
                )?),
            },
        ])
    }

    fn pairs(&self) -> impl Iterator<Item = Pair<'_>> {
        self.pushed.iter().map(|(code, pushed)| Pair {
            code: format!("{} {code}", self.name),
            pushed,
            escaped: escape_for_push(pushed),
            read_back: self.read_back.get(code).unwrap_or_else(|| {
                panic!("{} {code}: no read-back", self.name)
            }),
        })
    }
}

struct Pair<'corpus> {
    code: String,
    pushed: &'corpus str,
    escaped: String,
    read_back: &'corpus str,
}

impl Pair<'_> {
    fn inserted_before(&self) -> Vec<usize> {
        let mut inserted = Vec::new();
        let mut pushed = self.pushed.bytes().enumerate().peekable();
        for byte in self.escaped.bytes() {
            match pushed.peek() {
                Some(&(_, expected)) if expected == byte => {
                    pushed.next();
                }
                Some(&(offset, _)) if byte == b'\\' => inserted.push(offset),
                _ => panic!("{}: not an insertion of backslashes", self.code),
            }
        }
        inserted
    }

    fn dotted_escapes(&self) -> Vec<usize> {
        self.inserted_before()
            .into_iter()
            .filter(|&offset| is_dotted_escape(self.pushed, offset))
            .collect()
    }

    fn digit_escapes(&self) -> Vec<usize> {
        self.inserted_before()
            .into_iter()
            .filter(|&offset| !is_dotted_escape(self.pushed, offset))
            .collect()
    }

    fn escaped_tokens(&self) -> Vec<Range<usize>> {
        let escapes = self.dotted_escapes();
        let tokens: Vec<Range<usize>> = ReadText::of(self.pushed)
            .iter()
            .flat_map(ReadText::dotted_tokens)
            .filter(|token| escapes.iter().any(|dot| token.contains(dot)))
            .collect();
        for dot in escapes {
            assert!(
                tokens.iter().any(|token| token.contains(&dot)),
                "{}: an escape at {dot} in no token",
                self.code
            );
        }
        tokens
    }

    fn linked_tokens(&self) -> BTreeSet<String> {
        let already_linked: Vec<String> = links(self.pushed)
            .into_iter()
            .map(|link| link.text)
            .collect();
        links(self.read_back)
            .into_iter()
            .filter(|link| {
                link.kind == LinkKind::DottedToken
                    && !already_linked.contains(&link.text)
            })
            .flat_map(|link| linked_hosts(link.text))
            .collect()
    }

    fn scheme_urls(&self) -> Vec<String> {
        links(self.read_back)
            .into_iter()
            .filter(|link| link.kind == LinkKind::SchemeUrl)
            .map(|link| link.text)
            .collect()
    }

    fn lone_lines_linear_listed(&self) -> Vec<usize> {
        if !self.read_back.lines().any(is_empty_ordered_item) {
            return Vec::new();
        }
        let item_starts: Vec<usize> = events(self.pushed)
            .into_iter()
            .filter_map(|(event, range)| {
                matches!(event, MarkdownEvent::Start(MarkdownTag::Item))
                    .then_some(range.start)
            })
            .collect();
        lone_number_delimiters(self.pushed)
            .into_iter()
            .filter(|&(line, _)| !item_starts.contains(&line))
            .map(|(_, delimiter)| delimiter)
            .collect()
    }
}

fn is_dotted_escape(pushed: &str, offset: usize) -> bool {
    pushed[offset..].starts_with('.')
        && pushed[offset + 1..]
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() || "-_".contains(next))
}

/// The hosts of the dotted tokens in `text`, or `text` itself when it holds
/// none, so a link the recogniser cannot see is reported as missed.
fn linked_hosts(text: String) -> Vec<String> {
    let hosts: Vec<String> = dotted_tokens(&text)
        .map(|token| host(&text[token]).to_owned())
        .collect();
    if hosts.is_empty() {
        vec![text]
    } else {
        hosts
    }
}

struct LinearLink {
    text: String,
    kind: LinkKind,
}

#[derive(PartialEq, Eq)]
enum LinkKind {
    DottedToken,
    SchemeUrl,
}

fn links(markdown: &str) -> Vec<LinearLink> {
    let mut links = Vec::new();
    let mut open: Option<(String, String)> = None;
    for (event, _) in events(markdown) {
        match (event, open.as_mut()) {
            (
                MarkdownEvent::Start(MarkdownTag::Link { destination, .. }),
                _,
            ) => {
                open = Some((destination, String::new()));
            }
            (
                MarkdownEvent::Text(text) | MarkdownEvent::Code(text),
                Some((_, link_text)),
            ) => link_text.push_str(&text),
            (MarkdownEvent::End(MarkdownTag::Link { .. }), _) => {
                if let Some((destination, text)) = open.take() {
                    links.extend(LinearLink::classify(&destination, text));
                }
            }
            _ => {}
        }
    }
    links
}

impl LinearLink {
    fn classify(destination: &str, text: String) -> Option<Self> {
        let host = destination
            .strip_prefix("http://")
            .or_else(|| destination.strip_prefix("mailto:"));
        let names_the_text =
            |host: &str| host == text || host.is_empty() || !text.is_ascii();
        let kind = if destination == text {
            LinkKind::SchemeUrl
        } else if host.is_some_and(names_the_text) {
            LinkKind::DottedToken
        } else {
            return None;
        };
        Some(Self { text, kind })
    }
}

/// Adjacent text events of a body, each written as it reads, so only
/// escapes fall between them.
#[derive(Default)]
struct ReadText {
    text: String,
    sources: Vec<usize>,
}

impl ReadText {
    fn of(markdown: &str) -> Vec<Self> {
        let mut texts = vec![Self::default()];
        for (event, range) in events(markdown) {
            match event {
                MarkdownEvent::Text(text)
                    if markdown[range.clone()] == text =>
                {
                    let current = texts.last_mut().unwrap();
                    current.text.push_str(&text);
                    current.sources.extend(range);
                }
                _ => texts.push(Self::default()),
            }
        }
        texts
    }

    /// The source range of each dotted token in the text.
    fn dotted_tokens(&self) -> Vec<Range<usize>> {
        dotted_tokens(&self.text)
            .map(|token| {
                self.sources[token.start]..self.sources[token.end - 1] + 1
            })
            .collect()
    }
}

/// How Linear stores a lone number it turned into a list item: the number,
/// `. ` and nothing after.
fn is_empty_ordered_item(line: &str) -> bool {
    let content = line.trim_start_matches(['>', ' ', '\t']);
    let digits = content.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && &content[digits..] == ". "
}

/// For each line holding only a number and `.` or `)`, the offset where its
/// content starts and the offset of its delimiter.
fn lone_number_delimiters(markdown: &str) -> Vec<(usize, usize)> {
    let mut delimiters = Vec::new();
    let mut line_start = 0;
    for line in markdown.split_inclusive('\n') {
        let content = line.trim_start_matches(['>', ' ', '\t']);
        let content_start = line_start + line.len() - content.len();
        let digits = content.bytes().take_while(u8::is_ascii_digit).count();
        let rest = &content[digits..];
        if digits > 0
            && rest.starts_with(['.', ')'])
            && rest[1..].trim().is_empty()
        {
            delimiters.push((content_start, content_start + digits));
        }
        line_start += line.len();
    }
    delimiters
}

fn host(token: &str) -> &str {
    token.split(':').next().unwrap_or(token)
}

fn token_host(pushed: &str, token: &Range<usize>) -> String {
    host(&pushed[token.clone()]).replace('\\', "")
}

/// The classes of lone number the probe corpus shows Linear leaves as text.
fn is_recorded_digit_over_escape(pushed: &str, delimiter: usize) -> bool {
    let line_start = pushed[..delimiter].rfind('\n').map_or(0, |end| end + 1);
    let line_end = pushed[delimiter..]
        .find('\n')
        .map_or(pushed.len(), |end| delimiter + end);
    let digits = pushed[line_start..delimiter]
        .bytes()
        .rev()
        .take_while(u8::is_ascii_digit)
        .count();
    digits >= 10
        || !pushed[delimiter + 1..line_end].trim().is_empty()
        || matches!(
            Container::around(pushed, delimiter),
            Container::BlockQuote
                | Container::TopLevel
                | Container::LaterParagraphOfItem
        )
}

#[derive(Debug, PartialEq, Eq)]
enum Container {
    BlockQuote,
    TopLevel,
    FirstParagraphOfItem,
    LaterParagraphOfItem,
}

impl Container {
    fn around(markdown: &str, offset: usize) -> Self {
        let mut open: Vec<(MarkdownTag, usize)> = Vec::new();
        for (event, range) in events(markdown) {
            match event {
                MarkdownEvent::Start(MarkdownTag::Paragraph) => {
                    if let Some((MarkdownTag::Item, paragraphs)) =
                        open.last_mut()
                    {
                        *paragraphs += 1;
                    }
                    open.push((MarkdownTag::Paragraph, 0));
                }
                MarkdownEvent::Start(tag) => open.push((tag, 0)),
                MarkdownEvent::End(_) => {
                    open.pop();
                }
                MarkdownEvent::Text(_) if range.contains(&offset) => {
                    return Self::innermost(&open);
                }
                _ => {}
            }
        }
        Self::TopLevel
    }

    fn innermost(open: &[(MarkdownTag, usize)]) -> Self {
        if open.iter().any(|(tag, _)| *tag == MarkdownTag::BlockQuote) {
            return Self::BlockQuote;
        }
        for (tag, paragraphs) in open.iter().rev() {
            match tag {
                MarkdownTag::Item if *paragraphs > 1 => {
                    return Self::LaterParagraphOfItem;
                }
                MarkdownTag::Item => return Self::FirstParagraphOfItem,
                _ => {}
            }
        }
        Self::TopLevel
    }
}

#[track_caller]
fn assert_none(failures: &[String]) {
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn assert_every_token_linear_linked_is_escaped(corpus: &Corpus) {
    let mut failures = Vec::new();
    for pair in corpus.pairs() {
        let escaped: BTreeSet<String> = pair
            .escaped_tokens()
            .iter()
            .map(|token| token_host(pair.pushed, token))
            .collect();
        let linked = pair.linked_tokens();
        let missed: Vec<&String> = linked.difference(&escaped).collect();
        if !missed.is_empty() {
            failures.push(format!(
                "{}: Linear linked {missed:?}, left unescaped in {:?}",
                pair.code, pair.escaped
            ));
        }
    }
    assert_none(&failures);
}

fn assert_every_unneeded_escape_is_a_recorded_over_escape(corpus: &Corpus) {
    let mut failures = Vec::new();
    for pair in corpus.pairs() {
        let linked = pair.linked_tokens();
        for token in pair.escaped_tokens() {
            let text = token_host(pair.pushed, &token);
            if !linked.contains(&text)
                && !linear_leaves_bare(pair.pushed, token.clone())
            {
                failures.push(format!(
                    "{}: {text:?} escaped, though Linear left it bare",
                    pair.code
                ));
            }
        }
    }
    assert_none(&failures);
}

fn assert_every_scheme_url_linear_linked_is_sent_unchanged(corpus: &Corpus) {
    for pair in corpus.pairs() {
        for url in pair.scheme_urls() {
            assert!(
                pair.escaped.contains(&url),
                "{}: {url:?} changed in {:?}",
                pair.code,
                pair.escaped
            );
        }
    }
}

#[test]
fn each_fragment_escapes_to_its_hand_escaped_capture() -> Result<(), TestError>
{
    let pushed = Corpus::capture()?.pushed;
    let escaped =
        fragments(&fixture("linear-rewrites/escaped/pushed.md")?, "## ");
    assert_eq!(
        pushed.keys().collect::<Vec<_>>(),
        escaped.keys().collect::<Vec<_>>()
    );
    for (code, fragment) in &pushed {
        assert_eq!(escape_for_push(fragment), escaped[code], "fragment {code}");
    }
    Ok(())
}

#[test]
fn the_whole_capture_escapes_to_the_hand_escaped_capture(
) -> Result<(), TestError> {
    assert_eq!(
        escape_for_push(&fixture("linear-rewrites/pushed.md")?),
        fixture("linear-rewrites/escaped/pushed.md")?
    );
    Ok(())
}

#[test]
fn every_token_linear_linked_is_escaped() -> Result<(), TestError> {
    assert_every_token_linear_linked_is_escaped(&Corpus::capture()?);
    Ok(())
}

#[test]
fn every_unneeded_escape_is_a_recorded_over_escape() -> Result<(), TestError> {
    assert_every_unneeded_escape_is_a_recorded_over_escape(&Corpus::capture()?);
    Ok(())
}

#[test]
fn every_scheme_url_linear_linked_is_sent_unchanged() -> Result<(), TestError> {
    let capture = Corpus::capture()?;
    let urls: Vec<String> = capture
        .pairs()
        .flat_map(|pair| pair.scheme_urls())
        .collect();
    assert!(urls.len() >= 3, "{urls:?}");
    assert_every_scheme_url_linear_linked_is_sent_unchanged(&capture);
    Ok(())
}

#[test]
fn every_token_linear_linked_in_a_probe_is_escaped() -> Result<(), TestError> {
    for corpus in Corpus::probes()? {
        assert_every_token_linear_linked_is_escaped(&corpus);
    }
    Ok(())
}

#[test]
fn every_unneeded_escape_in_a_probe_is_a_recorded_over_escape(
) -> Result<(), TestError> {
    for corpus in Corpus::probes()? {
        assert_every_unneeded_escape_is_a_recorded_over_escape(&corpus);
    }
    Ok(())
}

#[test]
fn every_scheme_url_linear_linked_in_a_probe_is_sent_unchanged(
) -> Result<(), TestError> {
    for corpus in Corpus::probes()? {
        assert_every_scheme_url_linear_linked_is_sent_unchanged(&corpus);
    }
    Ok(())
}

#[test]
fn every_lone_line_linear_listed_is_escaped() -> Result<(), TestError> {
    let mut listed = 0;
    for corpus in Corpus::probes()? {
        for pair in corpus.pairs() {
            let escapes = pair.digit_escapes();
            for delimiter in pair.lone_lines_linear_listed() {
                listed += 1;
                assert!(
                    escapes.contains(&delimiter),
                    "{}: the lone number before {delimiter} is unescaped in \
                     {:?}",
                    pair.code,
                    pair.escaped
                );
            }
        }
    }
    assert!(listed >= 10, "{listed} lone lines listed");
    Ok(())
}

#[test]
fn every_unneeded_digit_escape_is_a_recorded_over_escape(
) -> Result<(), TestError> {
    for corpus in Corpus::probes()? {
        for pair in corpus.pairs() {
            let listed = pair.lone_lines_linear_listed();
            for delimiter in pair.digit_escapes() {
                assert!(
                    listed.contains(&delimiter)
                        || is_recorded_digit_over_escape(
                            pair.pushed,
                            delimiter
                        ),
                    "{}: the number before {delimiter} escaped, though \
                     Linear left it as text, in {:?}",
                    pair.code,
                    pair.pushed
                );
            }
        }
    }
    Ok(())
}
