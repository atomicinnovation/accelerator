//! The frontmatter write-convention: a line-preserving `status:` value
//! replacement over `document::fence_offsets`.
//!
//! Quote style, inline comments, key order, CRLF line endings, and the body are
//! all preserved verbatim. Only a top-level (non-indented) `status:` key is
//! touched.

use std::fmt::Display;
use std::fmt::Formatter;

/// A reason a `status:` patch could not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    FrontmatterAbsent,
    FrontmatterMalformed,
    KeyNotFound,
    UnsupportedValueShape { reason: String },
}

impl Display for PatchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrontmatterAbsent => {
                write!(formatter, "frontmatter is absent")
            }
            Self::FrontmatterMalformed => {
                write!(formatter, "frontmatter is malformed")
            }
            Self::KeyNotFound => {
                write!(formatter, "status key not present in frontmatter")
            }
            Self::UnsupportedValueShape { reason } => {
                write!(formatter, "status value shape is unsupported: {reason}")
            }
        }
    }
}

impl std::error::Error for PatchError {}

/// Replaces the top-level `status:` value in `raw`'s frontmatter with
/// `new_value`, preserving everything else.
///
/// # Errors
///
/// [`PatchError`] when the frontmatter is absent or malformed, the `status:`
/// key is missing, or its value shape cannot be safely rewritten.
pub fn patch_status(
    raw: &[u8],
    new_value: &str,
) -> Result<Vec<u8>, PatchError> {
    let (yaml_start, body_start) = match document::fence_offsets(raw) {
        Ok(None) => return Err(PatchError::FrontmatterAbsent),
        Err(_) => return Err(PatchError::FrontmatterMalformed),
        Ok(Some(offsets)) => offsets,
    };

    let region = &raw[yaml_start..body_start];
    let mut pos = 0usize;
    let mut found: Option<(usize, usize)> = None;

    while pos < region.len() {
        let line_lf = region[pos..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map(|n| pos + n);
        let (line_end, next_pos) =
            line_lf.map_or((region.len(), region.len()), |lf| (lf + 1, lf + 1));
        let line = &region[pos..line_end];

        if split_line_ending(line).0 == b"---" {
            break;
        }

        let indented = line
            .first()
            .is_some_and(|&byte| byte == b' ' || byte == b'\t');
        if !indented && line.starts_with(b"status:") {
            found = Some((yaml_start + pos, yaml_start + line_end));
            break;
        }

        pos = next_pos;
    }

    let (line_start, line_end) = found.ok_or(PatchError::KeyNotFound)?;
    let new_line = replace_status_value(&raw[line_start..line_end], new_value)?;

    let mut out = Vec::with_capacity(raw.len());
    out.extend_from_slice(&raw[..line_start]);
    out.extend_from_slice(&new_line);
    out.extend_from_slice(&raw[line_end..]);
    Ok(out)
}

fn split_line_ending(line: &[u8]) -> (&[u8], &[u8]) {
    line.strip_suffix(b"\r\n")
        .map(|body| (body, &b"\r\n"[..]))
        .or_else(|| line.strip_suffix(b"\n").map(|body| (body, &b"\n"[..])))
        .unwrap_or((line, &b""[..]))
}

fn replace_status_value(
    line: &[u8],
    new_value: &str,
) -> Result<Vec<u8>, PatchError> {
    let (body, line_ending) = split_line_ending(line);

    let after_colon = &body["status:".len()..];
    let ws_len = after_colon
        .iter()
        .take_while(|&&byte| byte == b' ' || byte == b'\t')
        .count();
    let whitespace = &after_colon[..ws_len];
    let value = &after_colon[ws_len..];

    let Some(&first) = value.first() else {
        return Err(PatchError::UnsupportedValueShape {
            reason: "empty value (possibly a block scalar on the next line)"
                .to_owned(),
        });
    };

    let new_bytes = new_value.as_bytes();
    match first {
        b'|' | b'>' => Err(unsupported("block scalar")),
        b'&' => Err(unsupported("anchor")),
        b'*' => Err(unsupported("alias")),
        b'{' => Err(unsupported("flow mapping")),
        b'"' => rewrite_quoted(
            whitespace,
            value,
            new_bytes,
            line_ending,
            b'"',
            "unclosed double-quoted string",
        ),
        b'\'' => {
            rewrite_single_quoted(whitespace, value, new_bytes, line_ending)
        }
        _ => rewrite_plain(whitespace, value, new_bytes, line_ending),
    }
}

fn unsupported(reason: &str) -> PatchError {
    PatchError::UnsupportedValueShape {
        reason: reason.to_owned(),
    }
}

fn rewrite_quoted(
    whitespace: &[u8],
    value: &[u8],
    new_bytes: &[u8],
    line_ending: &[u8],
    quote: u8,
    unclosed: &str,
) -> Result<Vec<u8>, PatchError> {
    let rest = &value[1..];
    let close = rest
        .iter()
        .position(|&byte| byte == quote)
        .ok_or_else(|| unsupported(unclosed))?;
    let trailing = &rest[close + 1..];
    Ok(assemble(
        whitespace,
        quote,
        new_bytes,
        trailing,
        line_ending,
    ))
}

fn rewrite_single_quoted(
    whitespace: &[u8],
    value: &[u8],
    new_bytes: &[u8],
    line_ending: &[u8],
) -> Result<Vec<u8>, PatchError> {
    let rest = &value[1..];
    let mut index = 0;
    while index < rest.len() {
        if rest[index] == b'\'' {
            if index + 1 < rest.len() && rest[index + 1] == b'\'' {
                index += 2;
            } else {
                break;
            }
        } else {
            index += 1;
        }
    }
    if index >= rest.len() {
        return Err(unsupported("unclosed single-quoted string"));
    }
    let trailing = &rest[index + 1..];
    Ok(assemble(
        whitespace,
        b'\'',
        new_bytes,
        trailing,
        line_ending,
    ))
}

fn rewrite_plain(
    whitespace: &[u8],
    value: &[u8],
    new_bytes: &[u8],
    line_ending: &[u8],
) -> Result<Vec<u8>, PatchError> {
    let text = std::str::from_utf8(value)
        .map_err(|_| unsupported("invalid UTF-8 in value"))?;
    let comment_start = find_comment_start(text);
    let content = text[..comment_start.unwrap_or(text.len())].trim_end();
    if content.contains('#') {
        return Err(unsupported("value contains '#' outside a quoted region"));
    }

    let mut out = prefix(whitespace);
    out.extend_from_slice(new_bytes);
    if let Some(start) = comment_start {
        out.extend_from_slice(&value[start..]);
    }
    out.extend_from_slice(line_ending);
    Ok(out)
}

fn assemble(
    whitespace: &[u8],
    quote: u8,
    new_bytes: &[u8],
    trailing: &[u8],
    line_ending: &[u8],
) -> Vec<u8> {
    let mut out = prefix(whitespace);
    out.push(quote);
    out.extend_from_slice(new_bytes);
    out.push(quote);
    out.extend_from_slice(trailing);
    out.extend_from_slice(line_ending);
    out
}

fn prefix(whitespace: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(b"status:");
    out.extend_from_slice(whitespace);
    out
}

fn find_comment_start(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for index in 1..bytes.len() {
        if bytes[index] == b'#'
            && (bytes[index - 1] == b' ' || bytes[index - 1] == b'\t')
        {
            let mut start = index;
            while start > 0
                && (bytes[start - 1] == b' ' || bytes[start - 1] == b'\t')
            {
                start -= 1;
            }
            return Some(start);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{patch_status, PatchError};

    #[test]
    fn replaces_a_simple_unquoted_value() -> Result<(), PatchError> {
        let out = patch_status(b"---\nstatus: todo\n---\n# body\n", "done")?;
        assert_eq!(out, b"---\nstatus: done\n---\n# body\n");
        Ok(())
    }

    #[test]
    fn preserves_order_quotes_comments_and_crlf() -> Result<(), PatchError> {
        let out = patch_status(b"---\nstatus: \"todo\"\n---\n", "done")?;
        assert_eq!(out, b"---\nstatus: \"done\"\n---\n");

        let out = patch_status(b"---\nstatus: 'todo'\n---\n", "done")?;
        assert_eq!(out, b"---\nstatus: 'done'\n---\n");

        let out = patch_status(b"---\nstatus: todo  # note\n---\n", "done")?;
        assert_eq!(out, b"---\nstatus: done  # note\n---\n");

        let out = patch_status(b"---\r\nstatus: todo\r\n---\r\n", "done")?;
        assert_eq!(out, b"---\r\nstatus: done\r\n---\r\n");
        Ok(())
    }

    #[test]
    fn preserves_the_body_byte_for_byte() -> Result<(), PatchError> {
        let body = "# heading\n\n```\n---\nsome code\n```\n";
        let input = format!("---\nstatus: todo\n---\n{body}");
        let out = patch_status(input.as_bytes(), "done")?;
        let text = String::from_utf8(out).map_err(|_| {
            PatchError::UnsupportedValueShape {
                reason: "non-utf8 output".to_owned(),
            }
        })?;
        assert!(text.ends_with(body));
        Ok(())
    }

    #[test]
    fn does_not_touch_an_indented_status() -> Result<(), PatchError> {
        let out = patch_status(
            b"---\nmeta:\n  status: todo\nstatus: todo\n---\n",
            "done",
        )?;
        let text = String::from_utf8(out).unwrap_or_default();
        assert!(text.contains("  status: todo\n"));
        assert!(text.contains("\nstatus: done\n"));
        Ok(())
    }

    #[test]
    fn rejects_absent_malformed_missing_and_block_scalar() {
        assert_eq!(
            patch_status(b"# heading\nbody\n", "done"),
            Err(PatchError::FrontmatterAbsent)
        );
        assert_eq!(
            patch_status(b"---\ntitle: foo\nno close", "done"),
            Err(PatchError::FrontmatterMalformed)
        );
        assert_eq!(
            patch_status(b"---\ntitle: foo\n---\n", "done"),
            Err(PatchError::KeyNotFound)
        );
        assert!(matches!(
            patch_status(b"---\nstatus: |\n  todo\n---\n", "done"),
            Err(PatchError::UnsupportedValueShape { .. })
        ));
    }
}
