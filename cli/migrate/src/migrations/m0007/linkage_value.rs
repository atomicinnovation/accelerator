//! The three-stage linkage-value normalisation pipeline: path → typed ref,
//! non-canonical PR reference → `pr:N`, single-candidate bare number →
//! `type:N`. A line-oriented port of the retired bash rewrite's own
//! `normalize_paths`/`normalize_pr_ref`/`normalize_bare` passes — each
//! scans every double-quoted token in the raw value text (a bare scalar or
//! a `[...]` list are the same shape here: quoted tokens embedded in
//! surrounding text) and replaces only the tokens its own shape matches.

use std::path::PathBuf;

use corpus::doc_type::DocTypeKey;

pub struct Outcome {
    pub value: String,
    pub unmapped_path: bool,
    pub bare_ambiguous: bool,
}

/// Applies the pipeline to one linkage value (`source_type`/`key` are the
/// current file's own type and the frontmatter key being rewritten).
#[must_use]
pub fn normalise(
    value: &str,
    source_type: &str,
    key: &str,
    table: &[(DocTypeKey, PathBuf)],
) -> Outcome {
    let mut unmapped_path = false;
    let after_paths = for_each_quoted_token(value, |token| {
        normalise_path_token(token, table, &mut unmapped_path)
    });
    let after_pr = for_each_quoted_token(&after_paths, |token| {
        match_pr_ref(token).map(|number| format!("\"pr:{number}\""))
    });
    let mut bare_ambiguous = false;
    let after_bare = for_each_quoted_token(&after_pr, |token| {
        normalise_bare_token(token, source_type, key, &mut bare_ambiguous)
    });
    Outcome {
        value: after_bare,
        unmapped_path,
        bare_ambiguous,
    }
}

/// Replaces every double-quoted token in `value` for which `transform`
/// returns `Some`; a token `transform` declines (`None`), or an unterminated
/// trailing quote, is copied through verbatim.
fn for_each_quoted_token(
    value: &str,
    mut transform: impl FnMut(&str) -> Option<String>,
) -> String {
    let mut out = String::new();
    let mut rest = value;
    loop {
        let Some(start) = rest.find('"') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        let after_quote = &rest[start + 1..];
        let Some(end) = after_quote.find('"') else {
            out.push_str(&rest[start..]);
            break;
        };
        let inner = &after_quote[..end];
        if let Some(replacement) = transform(inner) {
            out.push_str(&replacement);
        } else {
            out.push('"');
            out.push_str(inner);
            out.push('"');
        }
        rest = &after_quote[end + 1..];
    }
    out
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn normalise_path_token(
    token: &str,
    table: &[(DocTypeKey, PathBuf)],
    unmapped_path: &mut bool,
) -> Option<String> {
    if !(token.starts_with("meta/") && token.ends_with(".md")) {
        return None;
    }
    if let Some((kind, id)) = corpus::linkage::resolve_path_target(token, table)
    {
        Some(format!("\"{kind}:{id}\""))
    } else {
        *unmapped_path = true;
        None
    }
}

/// Whether `token` is wholly a non-canonical PR reference — `PR #N`,
/// `PR#N`, `pr #N`, `PR-N`/`pr-N`, or bare `#N` — returning just the
/// digits. An already-canonical `pr:N` has `:` immediately after `pr`,
/// which this never matches, so it is left untouched (idempotent).
fn match_pr_ref(token: &str) -> Option<String> {
    if let Some(rest) = token.strip_prefix('#') {
        return all_digits(rest).then(|| rest.to_owned());
    }
    let bytes = token.as_bytes();
    if bytes.len() < 2
        || !bytes[0].eq_ignore_ascii_case(&b'p')
        || !bytes[1].eq_ignore_ascii_case(&b'r')
    {
        return None;
    }
    let rest = &token[2..];
    let rest = rest
        .strip_prefix(' ')
        .or_else(|| rest.strip_prefix('-'))
        .unwrap_or(rest);
    let rest = rest.strip_prefix('#').unwrap_or(rest);
    all_digits(rest).then(|| rest.to_owned())
}

fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

#[allow(clippy::option_if_let_else)]
fn normalise_bare_token(
    token: &str,
    source_type: &str,
    key: &str,
    bare_ambiguous: &mut bool,
) -> Option<String> {
    if !all_digits(token) {
        return None;
    }
    if let Some(target_type) = bare_target_type(source_type, key) {
        Some(format!("\"{target_type}:{token}\""))
    } else {
        *bare_ambiguous = true;
        None
    }
}

/// The deterministic target doc-type for a bare-number value on
/// `(source_type, key)`, per ADR-0034's table — only the single-candidate
/// pairings; a multi-candidate or loose key is left for the interactive
/// hook.
fn bare_target_type(source_type: &str, key: &str) -> Option<&'static str> {
    match key {
        "parent" => {
            matches!(source_type, "work-item" | "plan").then_some("work-item")
        }
        "blocks" | "blocked_by" => {
            (source_type == "work-item").then_some("work-item")
        }
        "supersedes" | "superseded_by" => {
            (source_type == "adr").then_some("adr")
        }
        "target" => match source_type {
            "plan-review" | "plan-validation" => Some("plan"),
            "work-item-review" => Some("work-item"),
            _ => None,
        },
        "source" => (source_type == "work-item").then_some("note"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::normalise;
    use corpus::doc_type::DocTypeKey;
    use std::path::PathBuf;

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![
            (DocTypeKey::WorkItems, PathBuf::from("meta/work")),
            (DocTypeKey::Decisions, PathBuf::from("meta/decisions")),
        ]
    }

    #[test]
    fn a_mapped_path_becomes_a_typed_reference() {
        let outcome =
            normalise("\"meta/work/0042-foo.md\"", "plan", "parent", &table());
        assert_eq!(outcome.value, "\"work-item:0042\"");
        assert!(!outcome.unmapped_path);
    }

    #[test]
    fn an_unmapped_path_is_left_untouched_and_flagged() {
        let outcome =
            normalise("\"meta/docs/x.md\"", "plan", "parent", &table());
        assert_eq!(outcome.value, "\"meta/docs/x.md\"");
        assert!(outcome.unmapped_path);
    }

    #[test]
    fn pr_reference_variants_all_coerce_to_canonical() {
        for (input, expected) in [
            ("\"PR #416\"", "\"pr:416\""),
            ("\"PR#416\"", "\"pr:416\""),
            ("\"pr #416\"", "\"pr:416\""),
            ("\"PR-416\"", "\"pr:416\""),
            ("\"pr-416\"", "\"pr:416\""),
            ("\"#416\"", "\"pr:416\""),
        ] {
            let outcome = normalise(input, "plan", "relates_to", &table());
            assert_eq!(outcome.value, expected, "input {input}");
        }
    }

    #[test]
    fn an_already_canonical_pr_ref_is_not_re_grabbed() {
        let outcome = normalise("\"pr:416\"", "plan", "relates_to", &table());
        assert_eq!(outcome.value, "\"pr:416\"");
    }

    #[test]
    fn a_single_candidate_bare_number_resolves() {
        let outcome = normalise("\"0042\"", "work-item", "parent", &table());
        assert_eq!(outcome.value, "\"work-item:0042\"");
        assert!(!outcome.bare_ambiguous);
    }

    #[test]
    fn a_multi_candidate_bare_number_is_left_and_flagged() {
        let outcome =
            normalise("\"0042\"", "work-item", "relates_to", &table());
        assert_eq!(outcome.value, "\"0042\"");
        assert!(outcome.bare_ambiguous);
    }

    #[test]
    fn a_mixed_list_normalises_each_element_independently() {
        let outcome = normalise(
            "[\"meta/work/0042-foo.md\", \"work-item:0099\"]",
            "work-item",
            "blocks",
            &table(),
        );
        assert_eq!(outcome.value, "[\"work-item:0042\", \"work-item:0099\"]");
    }
}
