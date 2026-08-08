//! The four quote primitives the retired bash rewrite's shared fragment
//! carried: `fm_is_fence`, `fm_normalise_value`, `fm_semantic_inner`,
//! `fm_refuses`.

/// A strict fence line: exactly `---`, no trailing whitespace tolerated —
/// deliberately stricter than "carries a fence somewhere", so a
/// trailing-whitespace `---` routes to the fence-less backfill instead of
/// this rewrite.
pub fn is_strict_fence(line: &str) -> bool {
    line == "---"
}

fn is_double_quoted(value: &str) -> bool {
    value.len() >= 2 && value.starts_with('"') && value.ends_with('"')
}

fn is_single_quoted(value: &str) -> bool {
    value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'')
}

/// Re-expresses `value` as a double-quoted YAML scalar. An already
/// double-quoted value passes through verbatim; a single-quoted value is
/// re-quoted as double (escaping `\` and `"`); a bare value is wrapped with
/// no escaping applied.
pub fn normalise_value(value: &str) -> String {
    if is_double_quoted(value) {
        return value.to_owned();
    }
    if is_single_quoted(value) {
        let inner = &value[1..value.len() - 1];
        let escaped = inner.replace('\\', "\\\\").replace('"', "\\\"");
        return format!("\"{escaped}\"");
    }
    format!("\"{value}\"")
}

/// Strips exactly one layer of surrounding quotes (double or single) for
/// *comparison* purposes — never for re-emission.
pub fn semantic_inner(value: &str) -> &str {
    if is_double_quoted(value) || is_single_quoted(value) {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Whether `value` is wrapped in a matching pair of double or single
/// quotes.
pub fn is_quoted(value: &str) -> bool {
    is_double_quoted(value) || is_single_quoted(value)
}

/// Whether `value` cannot be safely mechanically quoted. An already-quoted
/// value never refuses; a bare value refuses if it carries a `#`
/// (comment-introducer shape) or a stray `"`.
pub fn refuses(value: &str) -> bool {
    if is_double_quoted(value) || is_single_quoted(value) {
        return false;
    }
    value.contains('#') || value.contains('"')
}

/// Whether a value is empty in the frontmatter sense: literally empty, an
/// empty quoted string, or an empty list.
pub fn is_empty_value(value: &str) -> bool {
    matches!(value.trim(), "" | "\"\"" | "''" | "[]")
}

#[cfg(test)]
mod tests {
    use super::{
        is_empty_value, is_strict_fence, normalise_value, refuses,
        semantic_inner,
    };

    #[test]
    fn a_bare_value_is_wrapped_in_double_quotes() {
        assert_eq!(normalise_value("alice"), "\"alice\"");
    }

    #[test]
    fn an_already_double_quoted_value_passes_through() {
        assert_eq!(normalise_value("\"alice\""), "\"alice\"");
    }

    #[test]
    fn a_single_quoted_value_is_reexpressed_as_double_with_escaping() {
        assert_eq!(
            normalise_value("'a \"quote\" and \\backslash'"),
            "\"a \\\"quote\\\" and \\\\backslash\""
        );
    }

    #[test]
    fn semantic_inner_strips_exactly_one_quote_layer() {
        assert_eq!(semantic_inner("\"alice\""), "alice");
        assert_eq!(semantic_inner("'alice'"), "alice");
        assert_eq!(semantic_inner("alice"), "alice");
    }

    #[test]
    fn a_quoted_value_never_refuses() {
        assert!(!refuses("\"a # b\""));
        assert!(!refuses("'a \" b'"));
    }

    #[test]
    fn a_bare_value_with_a_hash_or_quote_refuses() {
        assert!(refuses("a # b"));
        assert!(refuses("a \" b"));
    }

    #[test]
    fn a_bare_safe_value_does_not_refuse() {
        assert!(!refuses("alice"));
    }

    #[test]
    fn empty_shapes_are_recognised() {
        assert!(is_empty_value(""));
        assert!(is_empty_value("\"\""));
        assert!(is_empty_value("[]"));
        assert!(!is_empty_value("alice"));
    }

    #[test]
    fn only_an_exact_triple_dash_is_a_strict_fence() {
        assert!(is_strict_fence("---"));
        assert!(!is_strict_fence("--- "));
        assert!(!is_strict_fence("----"));
    }
}
