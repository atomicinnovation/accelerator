//! Value lookup within a single file's frontmatter text.
//!
//! Ports the two awk programs in
//! [`scripts/config-read-value.sh`](../../../../scripts/config-read-value.sh)
//! (lines 56-111): the sectioned (2-level) lookup and the top-level lookup.
//! Matching is **string-prefix** (not regex) to avoid metacharacter
//! injection, exactly as the bash uses `substr`/`index` rather than `~`.
//!
//! Returns `Some(value)` on the **first** matching (sub)key — including
//! `Some("")` when the key is present but set empty (`key:`). `None` means
//! the key is absent. The found/not-found distinction is carried separately
//! from the value because found-empty must suppress the caller's default
//! while absent must apply it.

/// Split a key on the first `.`: `"agents.reviewer"` → `(Some("agents"),
/// "reviewer")`; `"enabled"` → `(None, "enabled")`. Mirrors the bash
/// `${KEY%%.*}` / `${KEY#*.}` split.
pub fn split_key(key: &str) -> (Option<&str>, &str) {
    match key.find('.') {
        Some(i) => (Some(&key[..i]), &key[i + 1..]),
        None => (None, key),
    }
}

/// Look up `subkey` (optionally under `section`) in frontmatter `text`.
pub fn lookup(text: &str, section: Option<&str>, subkey: &str) -> Option<String> {
    match section {
        Some(section) => lookup_sectioned(text, section, subkey),
        None => lookup_top_level(text, subkey),
    }
}

fn is_indented(line: &str) -> bool {
    matches!(line.bytes().next(), Some(b' ' | b'\t'))
}

/// A non-empty line whose first char is not a space/tab. Mirrors awk
/// `/^[^ \t]/ && /[^ \t]/`.
fn is_nonindented_nonblank(line: &str) -> bool {
    !line.is_empty() && !is_indented(line)
}

fn trim_inline(s: &str) -> &str {
    s.trim_matches([' ', '\t'])
}

/// Strip a single matched layer of surrounding `"` or `'`. Mirrors awk
/// `val ~ /^".*"$/ || val ~ /^'.*'$/`.
fn strip_one_quote_layer(val: &str) -> &str {
    let b = val.as_bytes();
    if b.len() >= 2 {
        let (first, last) = (b[0], b[b.len() - 1]);
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &val[1..val.len() - 1];
        }
    }
    val
}

/// Extract and clean the value after a matched `key:` prefix: inline-trim,
/// then strip one quote layer.
fn clean_value(after_colon: &str) -> String {
    strip_one_quote_layer(trim_inline(after_colon)).to_string()
}

fn lookup_top_level(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in text.split('\n') {
        if is_indented(line) || line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(&prefix) {
            return Some(clean_value(rest));
        }
    }
    None
}

fn lookup_sectioned(text: &str, section: &str, subkey: &str) -> Option<String> {
    let section_prefix = format!("{section}:");
    let key_prefix = format!("{subkey}:");
    let mut in_section = false;
    for line in text.split('\n') {
        // Section start: line begins with `section:` AND the prefix is the
        // whole line OR the next char is a space/tab. Mirrors the awk
        // substr/length check.
        if let Some(rest) = line.strip_prefix(&section_prefix) {
            let starts_section =
                rest.is_empty() || matches!(rest.bytes().next(), Some(b' ' | b'\t'));
            if starts_section {
                in_section = true;
                continue;
            }
        }
        // Exit the section on a new non-indented, non-blank top-level line.
        if in_section && is_nonindented_nonblank(line) {
            in_section = false;
        }
        if in_section {
            let stripped = line.trim_start_matches([' ', '\t']);
            if let Some(rest) = stripped.strip_prefix(&key_prefix) {
                return Some(clean_value(rest));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_key_splits_on_first_dot_only() {
        assert_eq!(split_key("agents.reviewer"), (Some("agents"), "reviewer"));
        assert_eq!(split_key("enabled"), (None, "enabled"));
        assert_eq!(
            split_key("a.b.c"),
            (Some("a"), "b.c"),
            "only the first dot splits",
        );
    }

    #[test]
    fn top_level_match() {
        assert_eq!(
            lookup("enabled: true\nother: x", None, "enabled"),
            Some("true".to_string()),
        );
    }

    #[test]
    fn top_level_absent_is_none() {
        assert_eq!(lookup("enabled: true", None, "missing"), None);
    }

    #[test]
    fn sectioned_match() {
        let fm = "agents:\n  reviewer: my-reviewer\n  locator: my-locator";
        assert_eq!(
            lookup(fm, Some("agents"), "reviewer"),
            Some("my-reviewer".to_string()),
        );
        assert_eq!(
            lookup(fm, Some("agents"), "locator"),
            Some("my-locator".to_string()),
        );
    }

    #[test]
    fn sectioned_subkey_in_wrong_section_is_none() {
        let fm = "agents:\n  reviewer: a\nother:\n  reviewer: b";
        // First section wins for the agents lookup; the `other.reviewer`
        // must not leak.
        assert_eq!(
            lookup(fm, Some("agents"), "reviewer"),
            Some("a".to_string())
        );
        assert_eq!(lookup(fm, Some("missing"), "reviewer"), None);
    }

    #[test]
    fn blank_lines_within_a_section_are_allowed() {
        let fm = "agents:\n  reviewer: a\n\n  locator: b";
        assert_eq!(lookup(fm, Some("agents"), "locator"), Some("b".to_string()));
    }

    #[test]
    fn section_ends_at_next_top_level_key() {
        let fm = "agents:\n  reviewer: a\nother: x\n  reviewer: leaked";
        // The indented `reviewer: leaked` is after `other:` closed the
        // section, so it must not be found.
        assert_eq!(
            lookup(fm, Some("agents"), "reviewer"),
            Some("a".to_string())
        );
    }

    #[test]
    fn within_file_first_match_wins() {
        // A duplicate subkey in one section resolves to the FIRST occurrence
        // (awk `exit`), NOT the last — a HashMap/last-write-wins would invert
        // this.
        let fm = "agents:\n  reviewer: first\n  reviewer: second";
        assert_eq!(
            lookup(fm, Some("agents"), "reviewer"),
            Some("first".to_string()),
        );
    }

    #[test]
    fn top_level_first_match_wins() {
        assert_eq!(
            lookup("enabled: first\nenabled: second", None, "enabled"),
            Some("first".to_string()),
        );
    }

    #[test]
    fn found_empty_is_some_empty_not_none() {
        // `key:` (present but empty) must return Some("") so the caller
        // suppresses the default — distinct from an absent key (None).
        assert_eq!(lookup("enabled:", None, "enabled"), Some(String::new()));
        assert_eq!(
            lookup("agents:\n  reviewer:", Some("agents"), "reviewer"),
            Some(String::new()),
        );
    }

    #[test]
    fn one_quote_layer_is_stripped() {
        assert_eq!(
            lookup(r#"name: "quoted value""#, None, "name"),
            Some("quoted value".to_string()),
        );
        assert_eq!(
            lookup("name: 'single'", None, "name"),
            Some("single".to_string()),
        );
        // Only ONE layer: value `""double""` → `"double"`.
        assert_eq!(
            lookup("name: \"\"double\"\"", None, "name"),
            Some("\"double\"".to_string()),
        );
        // Mismatched quotes are not stripped.
        assert_eq!(
            lookup(r#"name: "mixed'"#, None, "name"),
            Some(r#""mixed'"#.to_string()),
        );
    }

    #[test]
    fn whitespace_around_value_is_trimmed() {
        assert_eq!(
            lookup("key:    spaced   ", None, "key"),
            Some("spaced".to_string()),
        );
    }

    #[test]
    fn array_value_is_returned_verbatim() {
        assert_eq!(
            lookup("lenses: [security, architecture]", None, "lenses"),
            Some("[security, architecture]".to_string()),
        );
    }

    #[test]
    fn section_header_requires_colon_then_space_or_eol() {
        // `agentsX:` must not be mistaken for the `agents` section.
        let fm = "agentsX:\n  reviewer: nope";
        assert_eq!(lookup(fm, Some("agents"), "reviewer"), None);
        // `agents:foo` (no space after colon) is not a section start.
        let fm2 = "agents:foo\n  reviewer: nope";
        assert_eq!(lookup(fm2, Some("agents"), "reviewer"), None);
    }

    #[test]
    fn prefix_match_does_not_confuse_sibling_keys() {
        // `reviewers:` must not match a lookup for `reviewer`.
        let fm = "agents:\n  reviewers: a\n  reviewer: b";
        assert_eq!(
            lookup(fm, Some("agents"), "reviewer"),
            Some("b".to_string())
        );
    }
}
