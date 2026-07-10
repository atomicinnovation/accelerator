//! Splitting a markdown config file into its YAML frontmatter and its body.
//!
//! Only the first two `---` fence lines delimit the frontmatter; the body is
//! never re-scanned, so a body `---` round-trips intact. A CRLF-terminated
//! fence (`---\r\n`) is recognised.

pub struct Split {
    pub frontmatter: String,
    pub body: String,
}

/// Splits `content` at its frontmatter fences.
///
/// # Errors
///
/// Returns a detail string when the frontmatter block is opened but never
/// closed.
pub fn split(content: &str) -> Result<Split, String> {
    let mut segments = content.split_inclusive('\n');
    let opens_frontmatter = segments.next().is_some_and(is_fence);
    if !opens_frontmatter {
        return Ok(Split {
            frontmatter: String::new(),
            body: content.to_owned(),
        });
    }

    let mut frontmatter = String::new();
    let mut body = String::new();
    let mut closed = false;
    for segment in segments {
        if !closed && is_fence(segment) {
            closed = true;
        } else if closed {
            body.push_str(segment);
        } else {
            frontmatter.push_str(segment);
        }
    }
    if !closed {
        return Err("unterminated frontmatter block".to_owned());
    }
    Ok(Split { frontmatter, body })
}

fn is_fence(segment: &str) -> bool {
    let without_newline = segment.strip_suffix('\n').unwrap_or(segment);
    let line = without_newline
        .strip_suffix('\r')
        .unwrap_or(without_newline);
    line == "---"
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn splits_frontmatter_from_body() -> Result<(), String> {
        let result = split("---\nkey: value\n---\nbody text\n")?;
        assert_eq!(result.frontmatter, "key: value\n");
        assert_eq!(result.body, "body text\n");
        Ok(())
    }

    #[test]
    fn a_file_without_a_fence_is_all_body() -> Result<(), String> {
        let result = split("no frontmatter here\n")?;
        assert_eq!(result.frontmatter, "");
        assert_eq!(result.body, "no frontmatter here\n");
        Ok(())
    }

    #[test]
    fn does_not_rescan_the_body_for_further_fences() -> Result<(), String> {
        let result = split("---\nkey: value\n---\nbefore\n---\nafter\n")?;
        assert_eq!(result.frontmatter, "key: value\n");
        assert_eq!(result.body, "before\n---\nafter\n");
        Ok(())
    }

    #[test]
    fn recognises_a_crlf_terminated_fence() -> Result<(), String> {
        let result = split("---\r\nkey: value\r\n---\r\nbody\r\n")?;
        assert_eq!(result.frontmatter, "key: value\r\n");
        assert_eq!(result.body, "body\r\n");
        Ok(())
    }

    #[test]
    fn a_body_without_a_trailing_newline_is_preserved() -> Result<(), String> {
        let result = split("---\nkey: value\n---\nno newline")?;
        assert_eq!(result.body, "no newline");
        Ok(())
    }

    #[test]
    fn an_unterminated_block_is_an_error() {
        assert!(split("---\nkey: value\n").is_err());
    }
}
