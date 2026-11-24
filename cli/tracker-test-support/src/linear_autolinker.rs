//! Which dotted tokens Linear's autolinker leaves bare, and how far a link
//! it makes reaches, as the probes in
//! `work-adapters/tests/fixtures/linear-autolink-probes` record them.

use std::ops::Range;

use document::markdown::events;
use document::markdown::MarkdownEvent;
use document::markdown::MarkdownTag;

const BARE_LAST_LABELS: [&str; 2] = ["md", "merck"];

/// Whether Linear leaves the dotted token at `token` in `text` as plain
/// text, though its last label names a top-level domain.
#[must_use]
pub fn linear_leaves_bare(text: &str, token: Range<usize>) -> bool {
    let token_text = &text[token.clone()];
    if token_text.contains('\\') {
        return true;
    }
    let host = host(token_text);
    let has_port = host.len() < token_text.len();
    let labels: Vec<&str> = host.split('.').collect();
    let last_label = labels.last().copied().unwrap_or_default();
    let before = &text[..token.start];
    let after = &text[token.end..];
    let preceding = before.chars().next_back();
    let following = after.chars().next();
    let names_a_bare_file = BARE_LAST_LABELS
        .iter()
        .any(|bare| last_label.eq_ignore_ascii_case(bare))
        && !has_port
        && linked_span(text, token.clone()).end == token.end;
    let follows_a_cancelling_character = preceding
        .is_some_and(|character| "-_:@/\\.".contains(character))
        || before.chars().rev().nth(1) == Some('\\');
    let precedes_a_cancelling_character = following
        .is_some_and(|character| "_~-+=$|^<`".contains(character))
        || (has_port && following == Some(':'));
    names_a_bare_file
        || !last_label.is_ascii()
        || follows_a_cancelling_character
        || precedes_a_cancelling_character
        || labels.iter().any(|label| !is_linkable_label(label))
        || ends_with_html_tag(before)
        || ends_with_entity(before)
        || starts_with_entity(after)
        || is_struck_through(text, token.start)
}

/// The span Linear links for a dotted token it does not leave bare: the
/// token, a single `~` before it, and any path, query or fragment after it.
pub fn linked_span(text: &str, token: Range<usize>) -> Range<usize> {
    let before = &text[..token.start];
    let start = if before.ends_with('~') && !before.ends_with("~~") {
        token.start - 1
    } else {
        token.start
    };
    start..token.end + path_length(&text[token.end..])
}

fn path_length(after: &str) -> usize {
    if !after.starts_with(['/', '?', '#']) {
        return 0;
    }
    let mut path = after
        .find(|character: char| {
            character.is_whitespace() || "<>".contains(character)
        })
        .map_or(after, |end| &after[..end]);
    loop {
        if let Some(shed) = path
            .strip_suffix(|character: char| ".,:;!?#\"'*".contains(character))
        {
            path = shed;
        } else if path.ends_with(')')
            && path.matches(')').count() > path.matches('(').count()
        {
            path = &path[..path.len() - 1];
        } else {
            return path.len();
        }
    }
}

fn host(token: &str) -> &str {
    token.split(':').next().unwrap_or(token)
}

fn is_linkable_label(label: &str) -> bool {
    !label.contains('_')
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label.chars().count() <= 63
}

fn ends_with_html_tag(before: &str) -> bool {
    before.strip_suffix('>').is_some_and(|before| {
        before.rfind(['<', '>']).is_some_and(|opening| {
            before[opening..].starts_with('<')
                && before[opening + 1..].chars().next().is_some_and(|name| {
                    name.is_ascii_alphabetic() || name == '/'
                })
        })
    })
}

fn ends_with_entity(before: &str) -> bool {
    before
        .strip_suffix(';')
        .and_then(|before| before.rfind('&').map(|start| &before[start + 1..]))
        .is_some_and(is_entity_name)
}

fn starts_with_entity(after: &str) -> bool {
    after
        .strip_prefix('&')
        .and_then(|after| after.find(';').map(|end| &after[..end]))
        .is_some_and(is_entity_name)
}

fn is_entity_name(name: &str) -> bool {
    let name = name.strip_prefix('#').unwrap_or(name);
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

fn is_struck_through(markdown: &str, offset: usize) -> bool {
    let mut struck_through = 0_usize;
    for (event, range) in events(markdown) {
        match event {
            MarkdownEvent::Start(MarkdownTag::Strikethrough) => {
                struck_through += 1;
            }
            MarkdownEvent::End(MarkdownTag::Strikethrough) => {
                struck_through = struck_through.saturating_sub(1);
            }
            MarkdownEvent::Text(_) if range.contains(&offset) => {
                return struck_through > 0;
            }
            _ => {}
        }
    }
    false
}
