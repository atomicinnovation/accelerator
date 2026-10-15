//! Markdown to the block-token stream.
//!
//! Containers — blockquotes and list items — are read recursively: their lines
//! are stripped of the `> ` prefix or the item's indentation and tokenised
//! again. A list item's first line is always paragraph text, and the
//! following indented lines that open no block continue that paragraph.
//!
//! The table guard is narrow by design: a row must both start and end with
//! `|`, and a table needs a delimiter row beneath its first row.
//! `tests/fixtures/adf-fidelity-quirks.txt` records why.

use crate::syntax;
use crate::syntax::{is_blank, ListKind, ListMarker};
use crate::AdfError;

/// One block record from the tokeniser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Paragraph(String),
    Heading { level: u8, text: String },
    Bullet(Vec<Token>),
    Ordered { number: String, item: Vec<Token> },
    TaskTodo(String),
    TaskDone(String),
    CodeOpen(String),
    CodeLine(String),
    CodeClose,
    HardBreak,
    BlockQuote(Vec<Token>),
    Table(Vec<Vec<String>>),
    Rule,
}

/// The token stream plus any non-fatal notices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tokenised {
    pub tokens: Vec<Token>,
    pub notices: Vec<String>,
}

impl Tokenised {
    fn absorb(&mut self, nested: Self) -> Vec<Token> {
        self.notices.extend(nested.notices);
        nested.tokens
    }
}

pub const UNDERSCORE_NOTICE: &str =
    "Notice: '__...__' is not emphasis in this subset; use **...** for bold";

#[derive(Default)]
struct State {
    tokenised: Tokenised,
    in_code: bool,
    in_paragraph: bool,
    paragraph: String,
    pending_hard_break: bool,
}

impl State {
    fn push(&mut self, token: Token) {
        self.tokenised.tokens.push(token);
    }

    fn emit_paragraph_segment(&mut self) {
        if !self.paragraph.is_empty() {
            let text = std::mem::take(&mut self.paragraph);
            self.push(Token::Paragraph(text));
        }
    }

    fn flush_paragraph(&mut self) {
        self.emit_paragraph_segment();
        self.in_paragraph = false;
        self.pending_hard_break = false;
    }
}

/// Tokenises a markdown body.
///
/// # Errors
///
/// [`AdfError::BadInput`] for a `\x1e`/`\x1f` byte in the input.
pub fn tokenise(markdown: &str) -> Result<Tokenised, AdfError> {
    if markdown.contains(['\u{1e}', '\u{1f}']) {
        return Err(AdfError::BadInput);
    }
    let lines: Vec<&str> = markdown
        .lines()
        .map(|raw| raw.strip_suffix('\r').unwrap_or(raw))
        .collect();
    Ok(read_blocks(&lines))
}

fn read_blocks(lines: &[&str]) -> Tokenised {
    let mut state = State::default();
    let mut index = 0;

    while let Some(&line) = lines.get(index) {
        index += 1;

        if state.in_code {
            if line.starts_with(syntax::FENCE) {
                state.push(Token::CodeClose);
                state.in_code = false;
            } else {
                state.push(Token::CodeLine(line.to_owned()));
            }
            continue;
        }

        if is_blank(line) {
            state.flush_paragraph();
            continue;
        }

        if syntax::is_quote(line) {
            state.flush_paragraph();
            let quoted: Vec<&str> = lines[index - 1..]
                .iter()
                .take_while(|line| syntax::is_quote(line))
                .map(|line| syntax::unquoted(line))
                .collect();
            index += quoted.len() - 1;
            let content = state.tokenised.absorb(read_blocks(&quoted));
            state.push(Token::BlockQuote(content));
            continue;
        }

        if starts_table(line, lines.get(index)) {
            state.flush_paragraph();
            let rows: Vec<Vec<String>> = std::iter::once(line)
                .chain(
                    lines[index + 1..]
                        .iter()
                        .copied()
                        .take_while(|line| syntax::is_table_row(line)),
                )
                .map(syntax::table_cells)
                .collect();
            index += rows.len();
            state.push(Token::Table(rows));
            continue;
        }

        if syntax::is_rule(line) {
            state.flush_paragraph();
            state.push(Token::Rule);
            continue;
        }

        if let Some(language) = syntax::fence_language(line) {
            state.flush_paragraph();
            state.push(Token::CodeOpen(language.to_owned()));
            state.in_code = true;
            continue;
        }

        if let Some((level, text)) = syntax::heading(line) {
            state.flush_paragraph();
            state.push(Token::Heading {
                level,
                text: text.to_owned(),
            });
            continue;
        }

        if let Some(marker) = syntax::list_marker(line) {
            state.flush_paragraph();
            if let Some(task) = task(&marker) {
                state.push(task);
                continue;
            }
            let continuation =
                continuation(&lines[index..], marker.content_column);
            index += continuation.len();
            let item = state
                .tokenised
                .absorb(read_item(marker.text, &continuation));
            state.push(match marker.kind {
                ListKind::Bullet => Token::Bullet(item),
                ListKind::Ordered { number } => Token::Ordered {
                    number: number.to_owned(),
                    item,
                },
            });
            continue;
        }

        accumulate_paragraph(&mut state, line);
    }

    state.flush_paragraph();
    state.tokenised
}

fn starts_table(line: &str, next: Option<&&str>) -> bool {
    syntax::is_table_row(line)
        && next.is_some_and(|next| {
            syntax::is_delimiter_row(next, syntax::table_cells(line).len())
        })
}

fn task(marker: &ListMarker<'_>) -> Option<Token> {
    if marker.kind != ListKind::Bullet {
        return None;
    }
    if let Some(rest) = marker.text.strip_prefix("[ ] ") {
        return Some(Token::TaskTodo(rest.to_owned()));
    }
    ["[x] ", "[X] "].into_iter().find_map(|done| {
        marker
            .text
            .strip_prefix(done)
            .map(|rest| Token::TaskDone(rest.to_owned()))
    })
}

/// The lines after an item's first that belong to it, with the item's
/// indentation removed: every line indented to its content column, and the
/// blank lines between them.
fn continuation<'a>(
    following: &[&'a str],
    content_column: usize,
) -> Vec<&'a str> {
    let mut belonging = 0;
    for (index, line) in following.iter().enumerate() {
        if is_blank(line) {
            continue;
        }
        if syntax::indentation(line) < content_column {
            break;
        }
        belonging = index + 1;
    }
    following[..belonging]
        .iter()
        .map(|line| {
            if is_blank(line) {
                ""
            } else {
                &line[content_column..]
            }
        })
        .collect()
}

/// A list item's first line keeps its trailing spaces as text when nothing
/// continues it, as a heading does.
fn read_item(first: &str, continuation: &[&str]) -> Tokenised {
    let continues = continuation
        .iter()
        .take_while(|line| !is_blank(line) && !syntax::opens_block(line))
        .count();

    let mut state = State::default();
    if continues == 0 {
        state.push(Token::Paragraph(first.to_owned()));
    } else {
        for line in std::iter::once(&first).chain(&continuation[..continues]) {
            accumulate_paragraph(&mut state, line);
        }
        state.flush_paragraph();
    }

    let rest = read_blocks(&continuation[continues..]);
    let mut tokenised = state.tokenised;
    let mut blocks = tokenised.absorb(rest);
    tokenised.tokens.append(&mut blocks);
    tokenised
}

fn accumulate_paragraph(state: &mut State, line: &str) {
    if line.contains("__") {
        state.tokenised.notices.push(UNDERSCORE_NOTICE.to_owned());
    }

    if state.in_paragraph && state.pending_hard_break {
        state.emit_paragraph_segment();
        state.push(Token::HardBreak);
    }
    state.pending_hard_break = false;

    let mut line = line;
    if has_hard_break_marker(line) {
        line = line.trim_end_matches(syntax::is_space);
        state.pending_hard_break = true;
    }

    if state.in_paragraph && !state.paragraph.is_empty() {
        state.paragraph.push(' ');
        state.paragraph.push_str(line);
    } else {
        line.clone_into(&mut state.paragraph);
        state.in_paragraph = true;
    }
}

/// `  +$` — two or more trailing spaces, and only in a paragraph. A heading or
/// a list item's lone line keeps them inside its text.
fn has_hard_break_marker(line: &str) -> bool {
    line.len() - line.trim_end_matches(' ').len() >= 2
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::path::Path;

    use super::{tokenise, Token, UNDERSCORE_NOTICE};

    fn notice_case(name: &str) -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/adf/assemble-notice-underscores")
                .join(name),
        )
        .expect("the notice case is committed")
    }

    #[test]
    fn the_tokeniser_emits_the_notice_without_failing() {
        let tokenised =
            tokenise("__not bold__\n").expect("a notice is not fatal");

        assert_eq!(tokenised.notices, vec![UNDERSCORE_NOTICE.to_owned()]);
        assert_eq!(
            tokenised.tokens,
            vec![Token::Paragraph("__not bold__".to_owned())]
        );
    }

    #[test]
    fn the_underscore_notice_is_emitted_without_failing() {
        let expected = notice_case("expected-notice.txt");
        let tokenised = tokenise(&notice_case("input.md"))
            .expect("a notice is not a failure");

        assert_eq!(
            notice_case("oracle-status.txt").trim(),
            "0",
            "the notice leaves the exit status at zero"
        );
        let oracle_stderr = notice_case("oracle.err");
        assert!(oracle_stderr.contains(expected.trim()), "{oracle_stderr}");
        assert_eq!(tokenised.notices, vec![expected.trim().to_owned()]);
    }
}
