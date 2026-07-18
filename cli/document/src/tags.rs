//! The explicit-tag guard.
//!
//! serde-saphyr resolves a YAML tag against its schema and hands the visitor
//! the resolved base value, so a tag is invisible at the serde boundary — an
//! `!custom` node would deserialise as its untagged value and the tag would be
//! silently dropped. The guard therefore runs over the parser's event stream,
//! where every node still carries the tag it was written with.
//!
//! Scanning events rather than raw text is what makes the rule exact: a tag
//! written inside a quoted scalar (`note: "see !!important"`) is just string
//! content and carries no tag, while a tag on a deeply nested value is still a
//! tagged event. Aliases are `Alias` events here, not expansions, so the scan
//! stays bounded on an alias-bomb input.

use serde_saphyr::granit_parser::{Event, Parser};

use crate::error::DocumentError;

/// Rejects frontmatter carrying an explicit YAML tag on any node.
///
/// # Errors
///
/// [`DocumentError::Tagged`] naming the first tag encountered.
pub fn reject_tagged(frontmatter: &str) -> Result<(), DocumentError> {
    for event in Parser::new_from_str(frontmatter) {
        // A scan failure is not this guard's to report: `parse_frontmatter`
        // lets serde-saphyr raise the `InvalidYaml` diagnostic so malformed
        // input keeps one error path.
        let Ok((event, _)) = event else { return Ok(()) };

        let tag = match &event {
            Event::Scalar(_, _, _, tag)
            | Event::SequenceStart(_, _, tag)
            | Event::MappingStart(_, _, tag) => tag.as_ref(),
            _ => None,
        };

        if let Some(tag) = tag {
            return Err(DocumentError::Tagged(tag.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::reject_tagged;
    use crate::error::DocumentError;

    fn rejects(frontmatter: &str) -> bool {
        matches!(reject_tagged(frontmatter), Err(DocumentError::Tagged(_)))
    }

    #[test]
    fn a_local_tag_is_rejected() {
        assert!(rejects("key: !custom value\n"));
    }

    #[test]
    fn a_standard_tag_is_rejected() {
        assert!(rejects("key: !!str 123\n"));
        assert!(rejects("key: !!int 7\n"));
    }

    #[test]
    fn a_tag_on_a_nested_value_is_rejected() {
        assert!(rejects("outer:\n  inner: !custom v\n"));
    }

    #[test]
    fn a_tag_on_a_sequence_item_is_rejected() {
        assert!(rejects("key:\n  - !custom v\n"));
    }

    #[test]
    fn a_tag_on_a_collection_is_rejected() {
        assert!(rejects("key: !custom {a: 1}\n"));
        assert!(rejects("key: !custom [1, 2]\n"));
    }

    #[test]
    fn a_tag_on_the_root_node_is_rejected() {
        assert!(rejects("!custom {a: 1}\n"));
    }

    #[test]
    fn a_tag_written_inside_a_quoted_scalar_is_content_not_a_tag() {
        // The event stream is what makes this exact: the scalar carries no tag,
        // its text merely contains one. A raw-text scan would false-positive.
        assert!(reject_tagged("note: \"see !!important\"\n").is_ok());
        assert!(reject_tagged("note: 'a !custom thing'\n").is_ok());
    }

    #[test]
    fn untagged_frontmatter_passes() {
        assert!(reject_tagged("a: 1\nb: [x, y]\nc:\n  d: true\n").is_ok());
    }

    #[test]
    fn the_error_names_the_offending_tag() {
        assert!(matches!(
            reject_tagged("key: !custom v\n"),
            Err(DocumentError::Tagged(tag)) if tag.contains("custom")
        ));
    }

    #[test]
    fn a_scan_failure_defers_to_the_yaml_diagnostic() {
        // Malformed input keeps one error path — `InvalidYaml`, from
        // serde-saphyr — rather than being reported as a tag failure.
        assert!(reject_tagged("key: : :\n  - broken\n").is_ok());
    }
}
