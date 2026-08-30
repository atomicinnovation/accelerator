//! The general canonical-quoting predicate.
//!
//! A bare scalar is canonical only when it is an integer, boolean, or null
//! literal; every string — scalar or flow element — must be double-quoted.
//!
//! Every scan is quote- and escape-aware, so a double-quoted value carrying an
//! escaped inner `"`, a `#`, a comma, or a `[`/`]` is read as one string
//! rather than truncated at the first structural-looking byte. Homed here in
//! `corpus` rather than a cross-domain shared crate: its only consumers are the
//! instance validator and the template-shape check, both in `corpus`. Work
//! item 0227's config validator can relocate it to a shared crate when it
//! needs it, rather than introduce a `config -> corpus` edge now.

use crate::frontmatter_validation::is_trailing_comment;

/// Whether `raw` is canonically quoted: a double-quoted scalar, a bare
/// integer/boolean/null literal, or a flow collection whose every element is.
#[must_use]
pub fn is_canonically_quoted(raw: &str) -> bool {
    let raw = raw.trim_start();
    if let Some(rest) = raw.strip_prefix('[') {
        let Some(close) = closing_bracket(rest) else {
            return false;
        };
        let tail = &rest[close + 1..];
        return (tail.is_empty() || is_trailing_comment(tail))
            && flow_elements(&rest[..close])
                .iter()
                .all(|element| is_canonical_scalar(element));
    }
    is_canonical_scalar(raw)
}

/// Whether `raw` is a double-quoted scalar, tolerating a trailing comment.
///
/// Locates the closing quote as the first *unescaped* `"`, so a value carrying
/// an escaped inner `\"` is not truncated at it.
#[must_use]
pub fn is_quoted_scalar(raw: &str) -> bool {
    let Some(rest) = raw.strip_prefix('"') else {
        return false;
    };
    let Some(close) = closing_quote(rest) else {
        return false;
    };
    let tail = &rest[close + 1..];
    tail.is_empty() || is_trailing_comment(tail)
}

fn is_canonical_scalar(raw: &str) -> bool {
    is_quoted_scalar(raw) || is_bare_int(raw) || is_bare_literal(raw)
}

/// An optional leading `-`, a digit run, then nothing or a trailing comment.
///
/// A leading zero is accepted: the general rule cannot know that a numeric
/// string field means a string, `id` keeps its dedicated must-be-quoted check,
/// and the emitter never produces this shape.
fn is_bare_int(raw: &str) -> bool {
    let body = raw.strip_prefix('-').unwrap_or(raw);
    let digits = body
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(body.len());
    if digits == 0 {
        return false;
    }
    let tail = &body[digits..];
    tail.is_empty() || is_trailing_comment(tail)
}

fn is_bare_literal(raw: &str) -> bool {
    ["true", "false", "null", "~"].iter().any(|literal| {
        raw.strip_prefix(literal)
            .is_some_and(|tail| tail.is_empty() || is_trailing_comment(tail))
    })
}

/// The first unescaped `"` in `rest`.
fn closing_quote(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

/// The first structural `]` in `rest` (the content after a `[`), skipping any
/// `]` inside a double-quoted element.
fn closing_bracket(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let mut index = 0;
    let mut in_quote = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if in_quote => index += 2,
            b'"' => {
                in_quote = !in_quote;
                index += 1;
            }
            b']' if !in_quote => return Some(index),
            _ => index += 1,
        }
    }
    None
}

/// Splits a flow collection's inner text on its structural commas, trimming
/// each element and dropping empty ones, so `["a, b"]` stays one element and
/// `[]` yields none.
fn flow_elements(inner: &str) -> Vec<&str> {
    let bytes = inner.as_bytes();
    let mut elements = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut in_quote = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if in_quote => {
                index += 2;
                continue;
            }
            b'"' => in_quote = !in_quote,
            b',' if !in_quote => {
                elements.push(inner[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    elements.push(inner[start..].trim());
    elements
        .into_iter()
        .filter(|element| !element.is_empty())
        .collect()
}
