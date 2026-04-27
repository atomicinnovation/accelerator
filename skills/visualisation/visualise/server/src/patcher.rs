use crate::frontmatter::{self, FenceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketStatus {
    Todo,
    InProgress,
    Done,
}

impl TicketStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in-progress",
            Self::Done => "done",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PatchError {
    #[error("frontmatter is absent")]
    FrontmatterAbsent,
    #[error("frontmatter is malformed")]
    FrontmatterMalformed,
    #[error("status key not present in frontmatter")]
    KeyNotFound,
    #[error("status value shape is unsupported: {reason}")]
    UnsupportedValueShape { reason: String },
}

/// Replaces the `status:` value in `raw`'s frontmatter with `new_value`.
///
/// Operates line-by-line so comments, key order, and surrounding whitespace
/// are preserved verbatim. Only a top-level (non-indented) `status:` key is
/// touched; nested occurrences inside YAML mappings are ignored.
pub fn patch_status(raw: &[u8], new_value: TicketStatus) -> Result<Vec<u8>, PatchError> {
    let (yaml_start, body_start) = match frontmatter::fence_offsets(raw) {
        Ok(None) => return Err(PatchError::FrontmatterAbsent),
        Err(FenceError::Malformed) => return Err(PatchError::FrontmatterMalformed),
        Ok(Some(offsets)) => offsets,
    };

    // Scan the frontmatter region for a top-level `status:` line.
    // fm_region includes the closing `---` line.
    let fm_region = &raw[yaml_start..body_start];

    let mut pos = 0usize;
    let mut found_line: Option<(usize, usize)> = None; // (abs_start, abs_end_excl)

    while pos < fm_region.len() {
        let line_lf = fm_region[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|n| pos + n);

        let (line_end_incl, next_pos) = match line_lf {
            Some(lf) => (lf + 1, lf + 1),
            None => (fm_region.len(), fm_region.len()),
        };

        let line_slice = &fm_region[pos..line_end_incl];

        // Stop at the closing fence.
        if strip_line_ending(line_slice) == b"---" {
            break;
        }

        // Only match top-level (non-indented) `status:` keys.
        let first_byte = line_slice.first().copied();
        if !first_byte.map(|b| b == b' ' || b == b'\t').unwrap_or(false)
            && line_slice.starts_with(b"status:")
        {
            let abs_start = yaml_start + pos;
            let abs_end = yaml_start + line_end_incl;
            found_line = Some((abs_start, abs_end));
            break;
        }

        pos = next_pos;
    }

    let (line_start, line_end) = found_line.ok_or(PatchError::KeyNotFound)?;
    let line_bytes = &raw[line_start..line_end];
    let new_line = replace_status_value(line_bytes, new_value)?;

    let mut out = Vec::with_capacity(raw.len());
    out.extend_from_slice(&raw[..line_start]);
    out.extend_from_slice(&new_line);
    out.extend_from_slice(&raw[line_end..]);
    Ok(out)
}

fn strip_line_ending(line: &[u8]) -> &[u8] {
    if line.ends_with(b"\r\n") {
        &line[..line.len() - 2]
    } else if line.ends_with(b"\n") {
        &line[..line.len() - 1]
    } else {
        line
    }
}

/// Rewrites the value portion of a `status: <value>` line, preserving
/// the original quote style (none / single / double), any trailing inline
/// comment, and the line's original line ending (LF or CRLF).
fn replace_status_value(line: &[u8], new_value: TicketStatus) -> Result<Vec<u8>, PatchError> {
    let (without_ending, line_ending) = if line.ends_with(b"\r\n") {
        (&line[..line.len() - 2], &b"\r\n"[..])
    } else if line.ends_with(b"\n") {
        (&line[..line.len() - 1], &b"\n"[..])
    } else {
        (line, &b""[..])
    };

    // "status:" is 7 bytes.
    let after_colon = &without_ending[7..];

    // Preserve the whitespace between `:` and the value.
    let ws_len = after_colon
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count();
    let ws = &after_colon[..ws_len];
    let value_part = &after_colon[ws_len..];

    if value_part.is_empty() {
        return Err(PatchError::UnsupportedValueShape {
            reason: "empty value (possibly block scalar on next line)".into(),
        });
    }

    let new_str = new_value.as_str().as_bytes();

    match value_part[0] {
        b'|' | b'>' => Err(PatchError::UnsupportedValueShape {
            reason: "block scalar".into(),
        }),
        b'&' => Err(PatchError::UnsupportedValueShape {
            reason: "anchor".into(),
        }),
        b'*' => Err(PatchError::UnsupportedValueShape {
            reason: "alias".into(),
        }),
        b'{' => Err(PatchError::UnsupportedValueShape {
            reason: "flow mapping".into(),
        }),
        b'"' => {
            // Double-quoted value: find the closing `"`.
            let rest = &value_part[1..];
            let close = rest.iter().position(|&b| b == b'"').ok_or_else(|| {
                PatchError::UnsupportedValueShape {
                    reason: "unclosed double-quoted string".into(),
                }
            })?;
            let trailing = &rest[close + 1..];

            let mut out = build_prefix(ws);
            out.push(b'"');
            out.extend_from_slice(new_str);
            out.push(b'"');
            out.extend_from_slice(trailing);
            out.extend_from_slice(line_ending);
            Ok(out)
        }
        b'\'' => {
            // Single-quoted value: closing `'` may be doubled ('') as escape.
            let rest = &value_part[1..];
            let mut i = 0;
            while i < rest.len() {
                if rest[i] == b'\'' {
                    if i + 1 < rest.len() && rest[i + 1] == b'\'' {
                        i += 2;
                    } else {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            if i >= rest.len() {
                return Err(PatchError::UnsupportedValueShape {
                    reason: "unclosed single-quoted string".into(),
                });
            }
            let trailing = &rest[i + 1..];

            let mut out = build_prefix(ws);
            out.push(b'\'');
            out.extend_from_slice(new_str);
            out.push(b'\'');
            out.extend_from_slice(trailing);
            out.extend_from_slice(line_ending);
            Ok(out)
        }
        _ => {
            // Unquoted value.
            let value_str =
                std::str::from_utf8(value_part).map_err(|_| PatchError::UnsupportedValueShape {
                    reason: "invalid UTF-8 in value".into(),
                })?;

            let comment_start = find_comment_start(value_str);
            let value_content = value_str[..comment_start.unwrap_or(value_str.len())].trim_end();

            // Reject values whose content contains a bare '#' (ambiguous without quoting).
            if value_content.contains('#') {
                return Err(PatchError::UnsupportedValueShape {
                    reason: "value contains '#' outside quoted region".into(),
                });
            }

            let mut out = build_prefix(ws);
            out.extend_from_slice(new_str);
            if let Some(cp) = comment_start {
                out.extend_from_slice(&value_part[cp..]);
            }
            out.extend_from_slice(line_ending);
            Ok(out)
        }
    }
}

fn build_prefix(ws: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(b"status:");
    v.extend_from_slice(ws);
    v
}

/// Returns the byte index within `s` where the inline comment starts
/// (the start of the whitespace run that precedes `#`).
fn find_comment_start(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'#' && i > 0 && (bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') {
            // Walk back to the start of the whitespace run.
            let mut ws_start = i;
            while ws_start > 0 && (bytes[ws_start - 1] == b' ' || bytes[ws_start - 1] == b'\t') {
                ws_start -= 1;
            }
            return Some(ws_start);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    // ── Step 1.1 ────────────────────────────────────────────────────────────
    #[test]
    fn replaces_simple_unquoted_status_value() {
        let input = b("---\nstatus: todo\n---\n# body\n");
        let out = patch_status(&input, TicketStatus::InProgress).unwrap();
        assert_eq!(out, b("---\nstatus: in-progress\n---\n# body\n"));
    }

    // ── Step 1.2 ────────────────────────────────────────────────────────────
    #[test]
    fn preserves_other_frontmatter_keys_and_order() {
        let input = b("---\ntitle: Foo\nstatus: todo\nticket: bar\n---\nbody\n");
        let out = patch_status(&input, TicketStatus::Done).unwrap();
        let out_str = std::str::from_utf8(&out).unwrap();
        assert!(out_str.contains("title: Foo\n"));
        assert!(out_str.contains("status: done\n"));
        assert!(out_str.contains("ticket: bar\n"));
        // Key order preserved: title comes before status
        let title_pos = out_str.find("title:").unwrap();
        let status_pos = out_str.find("status:").unwrap();
        let ticket_pos = out_str.find("ticket:").unwrap();
        assert!(title_pos < status_pos && status_pos < ticket_pos);
    }

    // ── Step 1.3 ────────────────────────────────────────────────────────────
    #[test]
    fn preserves_quoted_status_values() {
        let input = b("---\nstatus: \"todo\"\n---\nbody\n");
        let out = patch_status(&input, TicketStatus::InProgress).unwrap();
        assert_eq!(out, b("---\nstatus: \"in-progress\"\n---\nbody\n"));
    }

    // ── Step 1.4 ────────────────────────────────────────────────────────────
    #[test]
    fn preserves_inline_comment_after_status() {
        let input = b("---\nstatus: todo  # current\n---\nbody\n");
        let out = patch_status(&input, TicketStatus::InProgress).unwrap();
        assert_eq!(out, b("---\nstatus: in-progress  # current\n---\nbody\n"));
    }

    // ── Step 1.5 ────────────────────────────────────────────────────────────
    #[test]
    fn preserves_body_byte_for_byte() {
        // Body contains `---` inside a fenced code block.
        let body = "# heading\n\n```\n---\nsome code\n```\n";
        let input_str = format!("---\nstatus: todo\n---\n{body}");
        let input = input_str.as_bytes().to_vec();
        let out = patch_status(&input, TicketStatus::Done).unwrap();
        let out_str = std::str::from_utf8(&out).unwrap();
        let body_part = out_str.split_once("---\n").unwrap().1; // after opening fence
        let body_part = body_part.split_once("---\n").unwrap().1; // after closing fence
        assert_eq!(body_part, body);
    }

    // ── Step 1.6 ────────────────────────────────────────────────────────────
    #[test]
    fn preserves_crlf_line_endings() {
        let input = b("---\r\nstatus: todo\r\n---\r\nbody\r\n");
        let out = patch_status(&input, TicketStatus::InProgress).unwrap();
        assert_eq!(out, b("---\r\nstatus: in-progress\r\n---\r\nbody\r\n"));
    }

    // ── Step 1.7 ────────────────────────────────────────────────────────────
    #[test]
    fn idempotent_for_same_value() {
        let input = b("---\nstatus: done\n---\n");
        let out = patch_status(&input, TicketStatus::Done).unwrap();
        assert_eq!(out, input);
    }

    // ── Step 1.8 ────────────────────────────────────────────────────────────
    #[test]
    fn accepts_each_ticket_status_variant() {
        let base = b("---\nstatus: todo\n---\n");
        let todo = patch_status(&base, TicketStatus::Todo).unwrap();
        assert!(std::str::from_utf8(&todo)
            .unwrap()
            .contains("status: todo\n"));

        let ip = patch_status(&base, TicketStatus::InProgress).unwrap();
        assert!(std::str::from_utf8(&ip)
            .unwrap()
            .contains("status: in-progress\n"));

        let done = patch_status(&base, TicketStatus::Done).unwrap();
        assert!(std::str::from_utf8(&done)
            .unwrap()
            .contains("status: done\n"));
    }

    // ── Step 1.9 ────────────────────────────────────────────────────────────
    #[test]
    fn rejects_when_status_key_missing() {
        let input = b("---\ntitle: foo\n---\nbody\n");
        let err = patch_status(&input, TicketStatus::Done).unwrap_err();
        assert_eq!(err, PatchError::KeyNotFound);
    }

    // ── Step 1.10 ───────────────────────────────────────────────────────────
    #[test]
    fn rejects_when_frontmatter_absent() {
        let input = b("# Heading\nbody\n");
        let err = patch_status(&input, TicketStatus::Done).unwrap_err();
        assert_eq!(err, PatchError::FrontmatterAbsent);
    }

    // ── Step 1.11 ───────────────────────────────────────────────────────────
    #[test]
    fn rejects_when_frontmatter_malformed() {
        let input = b("---\ntitle: foo\nno closing fence");
        let err = patch_status(&input, TicketStatus::Done).unwrap_err();
        assert_eq!(err, PatchError::FrontmatterMalformed);
    }

    // ── Step 1.12 ───────────────────────────────────────────────────────────
    #[test]
    fn does_not_mutate_indented_status_in_nested_mapping() {
        let input = b("---\nmetadata:\n  status: todo\nstatus: todo\n---\nbody\n");
        let out = patch_status(&input, TicketStatus::Done).unwrap();
        let out_str = std::str::from_utf8(&out).unwrap();
        // Indented status: unchanged
        assert!(out_str.contains("  status: todo\n"));
        // Top-level status: replaced
        assert!(out_str.contains("\nstatus: done\n"));
    }

    // ── Step 1.13 ───────────────────────────────────────────────────────────
    #[test]
    fn does_not_close_frontmatter_at_body_internal_triple_dash() {
        // The body contains `---` on its own line. A naive "second --- ends
        // frontmatter" parser would mis-locate the close if the body `---`
        // appeared before a real status: line — but since we use fence_offsets
        // (which correctly finds the first --- WITHIN the YAML region), the
        // real frontmatter close is found and only the frontmatter's status:
        // is rewritten.
        let input = b("---\nstatus: todo\n---\n# body\n\n---\n\nmore text\n");
        let out = patch_status(&input, TicketStatus::Done).unwrap();
        let out_str = std::str::from_utf8(&out).unwrap();
        // Frontmatter status replaced
        assert!(out_str.starts_with("---\nstatus: done\n---\n"));
        // Body's `---` preserved
        assert!(out_str.ends_with("\n---\n\nmore text\n"));
    }

    // ── Step 1.14 ───────────────────────────────────────────────────────────
    #[test]
    fn rejects_block_scalar_status_value() {
        let input = b("---\nstatus: |\n  todo\n---\nbody\n");
        let err = patch_status(&input, TicketStatus::Done).unwrap_err();
        match err {
            PatchError::UnsupportedValueShape { reason } => {
                assert!(reason.contains("block scalar"), "reason: {reason}");
            }
            other => panic!("expected UnsupportedValueShape, got {other:?}"),
        }
    }

    // ── Step 1.15 ───────────────────────────────────────────────────────────
    #[test]
    fn preserves_single_quoted_status_value() {
        let input = b("---\nstatus: 'todo'\n---\nbody\n");
        let out = patch_status(&input, TicketStatus::InProgress).unwrap();
        assert_eq!(out, b("---\nstatus: 'in-progress'\n---\nbody\n"));
    }

    // ── Step 1.16 ───────────────────────────────────────────────────────────
    #[test]
    fn rejects_anchored_status_value() {
        let input = b("---\nstatus: &s todo\n---\nbody\n");
        let err = patch_status(&input, TicketStatus::Done).unwrap_err();
        match err {
            PatchError::UnsupportedValueShape { reason } => {
                assert!(reason.contains("anchor"), "reason: {reason}");
            }
            other => panic!("expected UnsupportedValueShape, got {other:?}"),
        }
    }

    // ── Step 1.17 ───────────────────────────────────────────────────────────
    // Flow-style top-level mapping: `{status: todo}` — the line-based scanner
    // never sees a `status:` key because the whole document is on one line
    // starting with `{`, so KeyNotFound is the correct error.
    #[test]
    fn rejects_flow_style_mapping_status_with_key_not_found() {
        let input = b("---\n{status: todo}\n---\nbody\n");
        let err = patch_status(&input, TicketStatus::Done).unwrap_err();
        assert_eq!(err, PatchError::KeyNotFound);
    }

    // ── Step 1.18 ───────────────────────────────────────────────────────────
    #[test]
    fn preserves_line_specific_line_ending() {
        // All LF except the `status:` line which uses CRLF.
        let input = b"---\ntitle: foo\nstatus: todo\r\nticket: bar\n---\nbody\n";
        let out = patch_status(input, TicketStatus::Done).unwrap();
        // The status line must use CRLF in the output.
        let out_str = std::str::from_utf8(&out).unwrap();
        assert!(
            out_str.contains("status: done\r\n"),
            "expected CRLF on status line"
        );
        // All other lines still use LF only.
        assert!(out_str.contains("title: foo\n"));
        assert!(out_str.contains("ticket: bar\n"));
    }
}
