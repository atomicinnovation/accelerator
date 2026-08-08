//! Strict-fence frontmatter extraction and single-key lookup, mirroring
//! `has_strict_fence`/`fm_get` from the bash source. Reuses
//! `corpus::frontmatter_validation::parse_entries` for the actual line scan
//! once the frontmatter region is sliced out.

use corpus::frontmatter_validation::parse_entries;
use corpus::frontmatter_validation::raw_value;

use crate::migrations::m0007::quote::is_strict_fence;

/// A strict leading fence: line one is exactly `---`, no trailing
/// whitespace tolerated.
pub fn has_strict_fence(content: &str) -> bool {
    content.lines().next().is_some_and(is_strict_fence)
}

/// The line index (0-based) of the closing strict fence — the first line
/// after line 0 that is itself a strict fence — or `None` when line 0 isn't
/// a strict fence, or no closing fence follows.
fn closing_fence_index(content: &str) -> Option<usize> {
    let mut lines = content.lines();
    if !lines.next().is_some_and(is_strict_fence) {
        return None;
    }
    lines.position(is_strict_fence).map(|index| index + 1)
}

/// The frontmatter text between the first two strict fence lines, or `None`
/// when there is no strict leading fence or no closing fence.
pub fn frontmatter_text(content: &str) -> Option<&str> {
    let close = closing_fence_index(content)?;
    let mut offset = 0;
    for line in content.lines().take(close).skip(1) {
        offset += line.len() + 1;
    }
    let first_line_len = content.lines().next()?.len() + 1;
    Some(&content[first_line_len..first_line_len + offset])
}

/// The text of the document body, after the closing fence — `None` when
/// there is no strict-fenced frontmatter region at all.
pub fn body_text(content: &str) -> Option<&str> {
    let close = closing_fence_index(content)?;
    let mut offset = 0;
    for line in content.lines().take(close + 1) {
        offset += line.len() + 1;
    }
    Some(content.get(offset..).unwrap_or_default())
}

/// The raw (untrimmed-of-quotes) value of `key` in the first frontmatter
/// region, or `None` if the file has no strict fence or the key is absent.
pub fn get(content: &str, key: &str) -> Option<String> {
    let entries = parse_entries(frontmatter_text(content)?);
    raw_value(&entries, key).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{body_text, frontmatter_text, get, has_strict_fence};

    #[test]
    fn a_well_formed_document_has_a_strict_fence() {
        assert!(has_strict_fence("---\ntype: plan\n---\nbody\n"));
    }

    #[test]
    fn a_trailing_whitespace_fence_is_not_strict() {
        assert!(!has_strict_fence("--- \ntype: plan\n---\nbody\n"));
    }

    #[test]
    fn frontmatter_text_slices_between_the_fences() {
        let content = "---\ntype: plan\ntitle: A\n---\nbody\n";
        assert_eq!(frontmatter_text(content), Some("type: plan\ntitle: A\n"));
    }

    #[test]
    fn frontmatter_text_is_none_without_a_closing_fence() {
        assert_eq!(frontmatter_text("---\ntype: plan\n"), None);
    }

    #[test]
    fn body_text_is_everything_after_the_closing_fence() {
        let content = "---\ntype: plan\n---\n\nbody line\n";
        assert_eq!(body_text(content), Some("\nbody line\n"));
    }

    #[test]
    fn get_returns_the_raw_value() {
        let content = "---\ntitle: \"A title\"\n---\nbody\n";
        assert_eq!(get(content, "title"), Some("\"A title\"".to_owned()));
    }

    #[test]
    fn get_is_none_for_an_absent_key() {
        let content = "---\ntitle: \"A title\"\n---\nbody\n";
        assert_eq!(get(content, "missing"), None);
    }
}
