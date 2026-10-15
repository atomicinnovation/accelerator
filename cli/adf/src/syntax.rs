//! The Markdown markers the tokeniser reads, in one place, so the renderer's
//! escapes cannot drift from what the tokeniser recognises.

pub const ESCAPE: char = '\\';

/// Escaped wherever they appear in text. `_` is not emphasis to this
/// tokeniser, but it is to `CommonMark`, which reads the same local files.
pub const INLINE_MARKERS: [char; 5] = ['`', '*', '_', '[', '~'];

pub const STRIKE: &str = "~~";
pub const FENCE: &str = "```";
pub const RULE: &str = "---";

const SPACE: [char; 6] = [' ', '\t', '\n', '\u{b}', '\u{c}', '\r'];

pub fn is_space(character: char) -> bool {
    SPACE.contains(&character)
}

pub fn is_blank(line: &str) -> bool {
    line.chars().all(is_space)
}

/// The characters a backslash escapes, as in `CommonMark`.
pub const fn is_escapable(character: char) -> bool {
    character.is_ascii_punctuation()
}

pub fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

pub fn fence_language(line: &str) -> Option<&str> {
    line.strip_prefix(FENCE)
}

pub fn is_quote(line: &str) -> bool {
    line.starts_with('>')
}

pub fn unquoted(line: &str) -> &str {
    line.strip_prefix("> ")
        .or_else(|| line.strip_prefix('>'))
        .unwrap_or(line)
}

/// A line that both starts and ends with `|`. A table also needs a delimiter
/// row beneath its first such line.
pub fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim_end_matches(is_space);
    trimmed.len() >= 2 && trimmed.starts_with('|') && trimmed.ends_with('|')
}

/// A row's cells, split on every unescaped `|` and trimmed.
pub fn table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim_end_matches(is_space);
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);

    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        match character {
            '|' => cells.push(std::mem::take(&mut cell)),
            ESCAPE => {
                cell.push(character);
                if let Some(next) = characters.next() {
                    cell.push(next);
                }
            }
            _ => cell.push(character),
        }
    }
    cells.push(cell);
    cells
        .iter()
        .map(|cell| cell.trim_matches(is_space).to_owned())
        .collect()
}

/// `| --- | :-: |`, with exactly `columns` cells.
pub fn is_delimiter_row(line: &str, columns: usize) -> bool {
    if !is_table_row(line) {
        return false;
    }
    let cells = table_cells(line);
    cells.len() == columns
        && cells.iter().all(|cell| {
            let dashes = cell.strip_prefix(':').unwrap_or(cell);
            let dashes = dashes.strip_suffix(':').unwrap_or(dashes);
            !dashes.is_empty() && dashes.chars().all(|dash| dash == '-')
        })
}

pub fn is_rule(line: &str) -> bool {
    let trimmed = line.trim_end_matches(is_space);
    trimmed.len() >= RULE.len() && trimmed.chars().all(|dash| dash == '-')
}

/// `^#{1,6} ` — seven hashes do not match, so the line becomes a paragraph.
pub fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&hashes) || !line[hashes..].starts_with(' ') {
        return None;
    }
    let level = u8::try_from(hashes).ok()?;
    Some((level, line[hashes..].trim_start_matches(is_space)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListKind<'a> {
    Bullet,
    Ordered { number: &'a str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListMarker<'a> {
    pub kind: ListKind<'a>,
    pub content_column: usize,
    pub text: &'a str,
}

/// `[-*+] ` with all following whitespace stripped from the text, or
/// `[0-9]+\. ` with the text kept verbatim, after any indentation.
pub fn list_marker(line: &str) -> Option<ListMarker<'_>> {
    let indent = indentation(line);
    let content = &line[indent..];

    if let Some(rest) = content.strip_prefix(['-', '*', '+']) {
        let text = rest.strip_prefix(' ')?;
        return Some(ListMarker {
            kind: ListKind::Bullet,
            content_column: indent + 2,
            text: text.trim_start_matches(is_space),
        });
    }

    let digits = content.len()
        - content
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .len();
    if digits == 0 {
        return None;
    }
    let text = content[digits..].strip_prefix(". ")?;
    Some(ListMarker {
        kind: ListKind::Ordered {
            number: &content[..digits],
        },
        content_column: indent + digits + 2,
        text,
    })
}

/// Whether the tokeniser would read this line as anything but paragraph text.
pub fn opens_block(line: &str) -> bool {
    fence_language(line).is_some()
        || is_quote(line)
        || is_table_row(line)
        || is_rule(line)
        || heading(line).is_some()
        || list_marker(line).is_some()
}

/// The line with one backslash where it stops the tokeniser reading a block
/// marker, so it reads back as paragraph text.
pub fn escape_line_start(line: &str) -> String {
    if !opens_block(line) {
        return line.to_owned();
    }
    let at = match list_marker(line) {
        Some(ListMarker {
            kind: ListKind::Ordered { number },
            ..
        }) => indentation(line) + number.len(),
        _ => indentation(line),
    };
    format!("{}{ESCAPE}{}", &line[..at], &line[at..])
}

/// Text with every inline marker and every `also` character escaped, and a
/// backslash escaped wherever it would otherwise escape what follows it.
pub fn escape_inline(text: &str, also: &[char]) -> String {
    let mut escaped = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        let reads_as_syntax = if character == ESCAPE {
            characters.peek().is_none_or(|next| is_escapable(*next))
        } else {
            INLINE_MARKERS.contains(&character) || also.contains(&character)
        };
        if reads_as_syntax {
            escaped.push(ESCAPE);
        }
        escaped.push(character);
    }
    escaped
}

pub fn unescape(text: &str) -> String {
    let mut unescaped = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == ESCAPE {
            if let Some(next) = characters.next_if(|next| is_escapable(*next)) {
                unescaped.push(next);
                continue;
            }
        }
        unescaped.push(character);
    }
    unescaped
}

pub fn find_unescaped(text: &str, pattern: &str) -> Option<usize> {
    let mut offset = 0;
    while offset < text.len() {
        let rest = &text[offset..];
        if rest.starts_with(pattern) {
            return Some(offset);
        }
        let mut characters = rest.chars();
        let character = characters.next()?;
        offset += character.len_utf8();
        if character == ESCAPE {
            if let Some(next) =
                characters.next().filter(|next| is_escapable(*next))
            {
                offset += next.len_utf8();
            }
        }
    }
    None
}
