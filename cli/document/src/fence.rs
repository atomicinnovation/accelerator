//! Splitting a markdown document into its YAML frontmatter and its body.
//!
//! [`fence_offsets`] is the byte-offset primitive; [`split`] derives the owned
//! `String` halves from it. Only the first two `---` fence lines delimit the
//! frontmatter; the body after the closing fence is never re-scanned, so a body
//! `---` round-trips intact. A CRLF-terminated fence (`---\r\n`) is recognised,
//! as is a closing fence that ends the input with no trailing newline.

use crate::error::DocumentError;

const MAX_SCAN: usize = 1 << 20;

/// The owned frontmatter and body halves of a document.
pub struct Split {
    pub frontmatter: String,
    pub body: String,
}

/// Returns the byte range `(yaml_start, body_start)` of the YAML content region
/// between `---` fences, or `None` when the content does not open with a fence.
///
/// `yaml_start` is the first byte after the opening fence line; `body_start` is
/// the first byte after the closing fence line (or the input length when the
/// closing fence ends the input without a trailing newline).
///
/// # Errors
///
/// [`DocumentError::Unterminated`] when an opening fence is present but no
/// closing fence is found within the 1 MiB scan window.
pub fn fence_offsets(
    raw: &[u8],
) -> Result<Option<(usize, usize)>, DocumentError> {
    let Some(first_lf) = raw.iter().position(|&byte| byte == b'\n') else {
        return Ok(None);
    };
    let first_line_end = if first_lf > 0 && raw[first_lf - 1] == b'\r' {
        first_lf - 1
    } else {
        first_lf
    };
    if &raw[..first_line_end] != b"---" {
        return Ok(None);
    }

    let scan_end = raw.len().min(MAX_SCAN);
    let yaml_start = first_lf + 1;
    if yaml_start >= raw.len() {
        return Err(DocumentError::Unterminated);
    }

    let mut pos = yaml_start;
    while pos < scan_end {
        let line_lf = raw[pos..scan_end]
            .iter()
            .position(|&byte| byte == b'\n')
            .map(|offset| pos + offset);
        let Some(line_lf) = line_lf else {
            let last = &raw[pos..scan_end];
            let last = last.strip_suffix(b"\r").unwrap_or(last);
            if scan_end == raw.len() && last == b"---" {
                return Ok(Some((yaml_start, raw.len())));
            }
            break;
        };
        let line_end = if line_lf > pos && raw[line_lf - 1] == b'\r' {
            line_lf - 1
        } else {
            line_lf
        };
        if &raw[pos..line_end] == b"---" {
            let body_start = if line_lf < raw.len() {
                line_lf + 1
            } else {
                raw.len()
            };
            return Ok(Some((yaml_start, body_start)));
        }
        pos = line_lf + 1;
    }

    Err(DocumentError::Unterminated)
}

/// Splits `content` into its owned frontmatter and body halves.
///
/// # Errors
///
/// [`DocumentError::Unterminated`] when the frontmatter block is opened but
/// never closed.
pub fn split(content: &str) -> Result<Split, DocumentError> {
    let Some((yaml_start, body_start)) = fence_offsets(content.as_bytes())?
    else {
        return Ok(Split {
            frontmatter: String::new(),
            body: content.to_owned(),
        });
    };
    let region = &content[yaml_start..body_start];
    let without_newline = region.strip_suffix('\n').unwrap_or(region);
    let without_return = without_newline
        .strip_suffix('\r')
        .unwrap_or(without_newline);
    let frontmatter =
        without_return.strip_suffix("---").unwrap_or(without_return);
    Ok(Split {
        frontmatter: frontmatter.to_owned(),
        body: content[body_start..].to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{fence_offsets, split};

    #[test]
    fn splits_frontmatter_from_body() -> Result<(), super::DocumentError> {
        let result = split("---\nkey: value\n---\nbody text\n")?;
        assert_eq!(result.frontmatter, "key: value\n");
        assert_eq!(result.body, "body text\n");
        Ok(())
    }

    #[test]
    fn a_file_without_a_fence_is_all_body() -> Result<(), super::DocumentError>
    {
        let result = split("no frontmatter here\n")?;
        assert_eq!(result.frontmatter, "");
        assert_eq!(result.body, "no frontmatter here\n");
        Ok(())
    }

    #[test]
    fn does_not_rescan_the_body() -> Result<(), super::DocumentError> {
        let result = split("---\nkey: value\n---\nbefore\n---\nafter\n")?;
        assert_eq!(result.frontmatter, "key: value\n");
        assert_eq!(result.body, "before\n---\nafter\n");
        Ok(())
    }

    #[test]
    fn recognises_a_crlf_terminated_fence() -> Result<(), super::DocumentError>
    {
        let result = split("---\r\nkey: value\r\n---\r\nbody\r\n")?;
        assert_eq!(result.frontmatter, "key: value\r\n");
        assert_eq!(result.body, "body\r\n");
        Ok(())
    }

    #[test]
    fn empty_frontmatter_is_an_empty_string() -> Result<(), super::DocumentError>
    {
        let result = split("---\n---\nbody\n")?;
        assert_eq!(result.frontmatter, "");
        assert_eq!(result.body, "body\n");
        Ok(())
    }

    #[test]
    fn a_body_opening_with_a_blank_line_is_preserved(
    ) -> Result<(), super::DocumentError> {
        let result = split("---\nkey: value\n---\n\nfirst real line\n")?;
        assert_eq!(result.body, "\nfirst real line\n");
        Ok(())
    }

    #[test]
    fn a_body_without_a_trailing_newline_is_preserved(
    ) -> Result<(), super::DocumentError> {
        // Pins the value directly. The render round-trip compares one `split`
        // against another, which would still agree if the body were dropped.
        let result = split("---\nkey: value\n---\nno newline")?;
        assert_eq!(result.body, "no newline");
        Ok(())
    }

    #[test]
    fn accepts_a_closing_fence_with_no_trailing_newline(
    ) -> Result<(), super::DocumentError> {
        let result = split("---\nkey: value\n---")?;
        assert_eq!(result.frontmatter, "key: value\n");
        assert_eq!(result.body, "");
        Ok(())
    }

    #[test]
    fn an_unterminated_block_is_an_error() {
        assert!(split("---\nkey: value\n").is_err());
    }

    #[test]
    fn a_closing_fence_beyond_the_scan_cap_is_unterminated() {
        let mut input = String::from("---\n");
        input.push_str(&"filler line\n".repeat(120_000));
        input.push_str("---\n");
        assert!(input.len() > super::MAX_SCAN);
        assert!(fence_offsets(input.as_bytes()).is_err());
        assert!(split(&input).is_err());
    }
}
