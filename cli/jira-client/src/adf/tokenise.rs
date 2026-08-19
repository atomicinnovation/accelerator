//! Markdown to the block-token stream.
//!
//! Two guards are reproduced with their quirks intact, because a work item's
//! body already round-tripped through them: the table guard is narrow (a line
//! must both start and end with `|`), and the nested-list guard has no space
//! requirement after the marker, so an indented `-word` continuation line is
//! rejected as a nested list. `tests/fixtures/adf-fidelity-quirks.txt` records
//! why each is preserved.

use crate::adf::AdfError;

/// One block record from the tokeniser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Paragraph(String),
    Heading { level: u8, text: String },
    Bullet(String),
    Ordered { number: String, text: String },
    TaskTodo(String),
    TaskDone(String),
    CodeOpen(String),
    CodeLine(String),
    CodeClose,
    HardBreak,
}

/// The token stream plus any non-fatal notices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tokenised {
    pub tokens: Vec<Token>,
    pub notices: Vec<String>,
}

pub const UNDERSCORE_NOTICE: &str =
    "Notice: '__...__' is not emphasis in this subset; use **...** for bold";

const SPACE: [char; 6] = [' ', '\t', '\n', '\u{b}', '\u{c}', '\r'];

fn is_space(character: char) -> bool {
    SPACE.contains(&character)
}

#[derive(Default)]
struct State {
    tokens: Vec<Token>,
    notices: Vec<String>,
    in_code: bool,
    in_paragraph: bool,
    paragraph: String,
    pending_hard_break: bool,
}

impl State {
    fn emit_paragraph_segment(&mut self) {
        if !self.paragraph.is_empty() {
            let text = std::mem::take(&mut self.paragraph);
            self.tokens.push(Token::Paragraph(text));
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
/// [`AdfError`] for a blockquote, a pipe table, an indented list marker, or a
/// `\x1e`/`\x1f` byte in the input.
pub fn tokenise(markdown: &str) -> Result<Tokenised, AdfError> {
    let mut state = State::default();

    for raw in markdown.lines() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);

        if line.contains('\u{1e}') || line.contains('\u{1f}') {
            return Err(AdfError::BadInput);
        }

        if state.in_code {
            if line.starts_with("```") {
                state.tokens.push(Token::CodeClose);
                state.in_code = false;
            } else {
                state.tokens.push(Token::CodeLine(line.to_owned()));
            }
            continue;
        }

        if line.chars().all(is_space) {
            state.flush_paragraph();
            continue;
        }

        if line.starts_with('>') {
            state.flush_paragraph();
            return Err(AdfError::UnsupportedBlockquote);
        }

        if line.starts_with('|')
            && line.trim_end_matches(is_space).ends_with('|')
        {
            state.flush_paragraph();
            return Err(AdfError::UnsupportedTable);
        }

        if starts_indented_list(line) {
            state.flush_paragraph();
            return Err(AdfError::UnsupportedNestedList);
        }

        if let Some(language) = line.strip_prefix("```") {
            state.flush_paragraph();
            state.tokens.push(Token::CodeOpen(language.to_owned()));
            state.in_code = true;
            continue;
        }

        if let Some(level) = heading_level(line) {
            state.flush_paragraph();
            let text = line[usize::from(level)..].trim_start_matches(is_space);
            state.tokens.push(Token::Heading {
                level,
                text: text.to_owned(),
            });
            continue;
        }

        if let Some(rest) = list_item_text(line) {
            state.flush_paragraph();
            state.tokens.push(classify_list_item(rest));
            continue;
        }

        if let Some((number, text)) = ordered_item(line) {
            state.flush_paragraph();
            state.tokens.push(Token::Ordered { number, text });
            continue;
        }

        accumulate_paragraph(&mut state, line);
    }

    state.flush_paragraph();
    Ok(Tokenised {
        tokens: state.tokens,
        notices: state.notices,
    })
}

/// `^[[:space:]]+[-*+]` or `^[[:space:]]+[0-9]+\.` — deliberately without a
/// space requirement after the marker.
fn starts_indented_list(line: &str) -> bool {
    let trimmed = line.trim_start_matches(is_space);
    if trimmed.len() == line.len() {
        return false;
    }
    if trimmed.starts_with(['-', '*', '+']) {
        return true;
    }
    let digits: String =
        trimmed.chars().take_while(char::is_ascii_digit).collect();
    !digits.is_empty() && trimmed[digits.len()..].starts_with('.')
}

/// `^#{1,6} ` — seven hashes do not match, so the line becomes a paragraph.
fn heading_level(line: &str) -> Option<u8> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&hashes) || !line[hashes..].starts_with(' ') {
        return None;
    }
    u8::try_from(hashes).ok()
}

/// `^[-*+] `, with all following whitespace stripped from the text.
fn list_item_text(line: &str) -> Option<&str> {
    let rest = line.strip_prefix(['-', '*', '+'])?;
    if !rest.starts_with(' ') {
        return None;
    }
    Some(rest.trim_start_matches(is_space))
}

fn classify_list_item(text: &str) -> Token {
    if let Some(rest) = text.strip_prefix("[ ] ") {
        return Token::TaskTodo(rest.to_owned());
    }
    for marker in ["[x] ", "[X] "] {
        if let Some(rest) = text.strip_prefix(marker) {
            return Token::TaskDone(rest.to_owned());
        }
    }
    Token::Bullet(text.to_owned())
}

/// `^[0-9]+\. `, with the number taken from the digits before the first
/// `". "`.
fn ordered_item(line: &str) -> Option<(String, String)> {
    let digits: String =
        line.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = line[digits.len()..].strip_prefix(". ")?;
    Some((digits, rest.to_owned()))
}

fn accumulate_paragraph(state: &mut State, line: &str) {
    if line.contains("__") {
        state.notices.push(UNDERSCORE_NOTICE.to_owned());
    }

    if state.in_paragraph && state.pending_hard_break {
        state.emit_paragraph_segment();
        state.tokens.push(Token::HardBreak);
    }
    state.pending_hard_break = false;

    let mut line = line;
    if has_hard_break_marker(line) {
        line = line.trim_end_matches(is_space);
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
/// list item keeps them inside its text.
fn has_hard_break_marker(line: &str) -> bool {
    line.len() - line.trim_end_matches(' ').len() >= 2
}
