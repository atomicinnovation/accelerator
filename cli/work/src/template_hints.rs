//! Template-hint-comment parsing. Port of
//! `work-item-template-field-hints.sh:52-88`.

/// Hardcoded fallback values matching the shipping template's own trailing
/// comments — empty for any field other than `kind`/`status`/`priority`.
#[must_use]
pub fn hardcoded_fallback(field: &str) -> Vec<String> {
    let values: &[&str] = match field {
        "kind" => &["story", "epic", "task", "bug", "spike"],
        "status" => &[
            "draft",
            "ready",
            "in-progress",
            "review",
            "done",
            "blocked",
            "abandoned",
        ],
        "priority" => &["high", "medium", "low"],
        _ => &[],
    };
    values.iter().map(|v| (*v).to_owned()).collect()
}

/// Scans `template_frontmatter`'s raw text for `field`'s line and its
/// trailing `# a | b | c` comment.
///
/// Falls back to [`hardcoded_fallback`] when the line or comment is absent
/// — a line scan, not a YAML parse, which would drop the comment.
#[must_use]
pub fn extract_hints(template_frontmatter: &str, field: &str) -> Vec<String> {
    let prefix = format!("{field}:");
    let Some(line) = template_frontmatter
        .lines()
        .find(|line| line.starts_with(prefix.as_str()))
    else {
        return hardcoded_fallback(field);
    };
    let Some(hash_index) = line.find('#') else {
        return hardcoded_fallback(field);
    };
    line[hash_index + 1..]
        .split('|')
        .map(|token| {
            token
                .trim_matches(|c: char| c.is_ascii_whitespace())
                .to_owned()
        })
        .filter(|token| !token.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{extract_hints, hardcoded_fallback};

    #[test]
    fn comment_present_yields_the_pipe_separated_values() {
        let frontmatter =
            "status: draft   # draft | ready | in-progress | review | done | blocked | abandoned\n";
        assert_eq!(
            extract_hints(frontmatter, "status"),
            vec![
                "draft",
                "ready",
                "in-progress",
                "review",
                "done",
                "blocked",
                "abandoned"
            ]
        );
    }

    #[test]
    fn comment_absent_falls_back_to_hardcoded() {
        let frontmatter = "title: \"Title as Short Noun Phrase\"\n";
        assert_eq!(extract_hints(frontmatter, "title"), Vec::<String>::new());
    }

    #[test]
    fn field_line_absent_falls_back_to_hardcoded() {
        let frontmatter = "status: draft\n";
        assert_eq!(
            extract_hints(frontmatter, "kind"),
            hardcoded_fallback("kind")
        );
    }

    #[test]
    fn hardcoded_fallback_is_empty_for_unknown_fields() {
        assert!(hardcoded_fallback("title").is_empty());
    }
}
