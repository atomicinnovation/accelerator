//! The frontmatter conformance checks: per-file structural validation and
//! whole-corpus referential integrity. Pure logic (no regex, no
//! filesystem), mirroring `corpus::linkage`'s dependency discipline.
//!
//! **Deliberately operates on the raw frontmatter text, not a parsed YAML
//! value tree.** Several bash checks (`UNQUOTED-ID`, `BAD-LINKAGE-SHAPE`)
//! depend on literal quote characters in the source — `id: "0042"` and
//! `id: 0042` parse to the identical string value once run through a real
//! YAML parser (the quoting is pure syntax, not semantics, for any value
//! that would resolve to a string either way), so a parsed
//! [`crate::Mapping`] cannot recover the distinction a byte-for-byte port
//! needs. [`parse_entries`] instead replicates the retired bash
//! implementation's own naive line scanner exactly, including its quirks:
//! first-occurrence-wins on a duplicate key, and no awareness of
//! block-style (multi-line) YAML lists — only flow-style (`[a, b]`) lists
//! are ever correctly scanned, matching every template's actual emission
//! convention and the retired implementation's own limitation.
//!
//! Whether the frontmatter is well-formed YAML at all (a tagged node, or a
//! non-mapping root) is a separate, stricter concern the adapter layer
//! decides via `corpus_adapters::document::parse` before ever calling into
//! this module — bash's naive scanner has no equivalent concept, so this
//! module never needs one either.

pub mod schema;
pub mod shape;
pub mod violation;

use std::collections::HashMap;
use std::path::PathBuf;

pub use crate::frontmatter_validation::violation::Violation;

/// One `key: value` line per entry, value whitespace-trimmed.
///
/// Mirrors the retired bash implementation's own line scanner: a line
/// qualifies iff it starts with a letter or underscore and carries a `:`
/// later on the same line, with everything before that colon a valid
/// `[A-Za-z0-9_]+` key. Indented lines (block-list items, nested mapping
/// keys) never qualify — this is a flat, single-line-per-key scan, not a
/// YAML parser.
#[must_use]
pub fn parse_entries(raw_frontmatter: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for line in raw_frontmatter.lines() {
        let Some(first) = line.chars().next() else {
            continue;
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            continue;
        }
        let Some(colon_index) = line.find(':') else {
            continue;
        };
        let key = &line[..colon_index];
        if key.is_empty()
            || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        let value = line[colon_index + 1..].trim();
        entries.push((key.to_owned(), value.to_owned()));
    }
    entries
}

/// The first value for `key`, or `None` if absent — first-occurrence-wins,
/// matching the retired bash implementation's own lookup.
#[must_use]
pub fn raw_value<'a>(
    entries: &'a [(String, String)],
    key: &str,
) -> Option<&'a str> {
    entries
        .iter()
        .find(|(existing, _)| existing == key)
        .map(|(_, value)| value.as_str())
}

/// Whether `key` is present (regardless of value) — matches the retired
/// bash implementation's own check.
#[must_use]
pub fn is_present(entries: &[(String, String)], key: &str) -> bool {
    entries.iter().any(|(existing, _)| existing == key)
}

/// Strips exactly one layer of surrounding double or single quotes.
///
/// Leaves any trailing content (inline comment, stray characters)
/// untouched. Mirrors the retired bash implementation's own unwrapper
/// exactly — deliberately not a general-purpose unquoter.
#[must_use]
pub fn strip_surrounding_quote(raw: &str) -> &str {
    if let Some(rest) = raw.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return &rest[..end];
        }
    } else if let Some(rest) = raw.strip_prefix('\'') {
        if let Some(end) = rest.find('\'') {
            return &rest[..end];
        }
    }
    raw
}

fn is_iso_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20 {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| {
        bytes[range].iter().all(u8::is_ascii_digit)
    };
    if !(digits(0..4)
        && bytes[4] == b'-'
        && digits(5..7)
        && bytes[7] == b'-'
        && digits(8..10)
        && bytes[10] == b'T'
        && digits(11..13)
        && bytes[13] == b':'
        && digits(14..16)
        && bytes[16] == b':'
        && digits(17..19))
    {
        return false;
    }
    match &value[19..] {
        "Z" => true,
        rest if rest.len() == 6 => {
            let rest = rest.as_bytes();
            matches!(rest[0], b'+' | b'-')
                && rest[1].is_ascii_digit()
                && rest[2].is_ascii_digit()
                && rest[3] == b':'
                && rest[4].is_ascii_digit()
                && rest[5].is_ascii_digit()
        }
        _ => false,
    }
}

/// The type this frontmatter's `type:` field names, with its surrounding
/// quote stripped.
///
/// `None` when the key is absent. Never falls back to path inference: a
/// file's own declared type is either present and self-consistent, or the
/// file fails `INVALID-TYPE` regardless of where it lives.
#[must_use]
pub fn declared_type(entries: &[(String, String)]) -> Option<&str> {
    raw_value(entries, "type").map(strip_surrounding_quote)
}

fn linkage_elements(raw: &str) -> Vec<&str> {
    let without_comment = raw.split('#').next().unwrap_or("").trim_end();
    let without_leading =
        without_comment.strip_prefix('[').unwrap_or(without_comment);
    let without_brackets =
        without_leading.strip_suffix(']').unwrap_or(without_leading);
    without_brackets
        .split(',')
        .map(str::trim)
        .filter(|element| !element.is_empty())
        .collect()
}

/// A double-quoted element's stripped inner text, or `None` when the
/// element isn't double-quoted (bash's own asymmetry: a single-quoted
/// element is treated as unquoted).
fn quoted_inner(element: &str) -> Option<&str> {
    if element.len() >= 2 && element.starts_with('"') && element.ends_with('"')
    {
        Some(&element[1..element.len() - 1])
    } else {
        None
    }
}

/// Validates one file's structural conformance against its own declared
/// `type:`.
///
/// Returns only `[Violation::InvalidType]` when the type is missing or
/// unrecognised — matching bash's early return, no further check is even
/// attempted.
#[must_use]
pub fn validate_file(raw_frontmatter: &str) -> Vec<Violation> {
    let entries = parse_entries(raw_frontmatter);
    let declared = declared_type(&entries).unwrap_or_default();
    let Some(row) = schema::row_for(declared) else {
        return vec![Violation::InvalidType {
            found: declared.to_owned(),
        }];
    };

    let mut violations = Vec::new();
    check_base_fields(&entries, &mut violations);
    check_id_quoting(&entries, &mut violations);
    check_schema_version(&entries, &mut violations);
    check_timestamps(&entries, &mut violations);
    check_status(&entries, row, &mut violations);
    check_provenance(&entries, row, &mut violations);
    check_forbidden_own_id(&entries, row, &mut violations);
    check_obsolete_legacy_keys(&entries, &mut violations);
    check_required_extras(&entries, row, &mut violations);
    check_empty_placeholders(&entries, &mut violations);
    check_linkage_shape(&entries, row, &mut violations);
    violations
}

fn check_base_fields(
    entries: &[(String, String)],
    violations: &mut Vec<Violation>,
) {
    for field in schema::BASE_FIELDS {
        if !is_present(entries, field) {
            violations.push(Violation::MissingBaseField { field });
        }
    }
}

fn check_id_quoting(
    entries: &[(String, String)],
    violations: &mut Vec<Violation>,
) {
    if let Some(id) = raw_value(entries, "id") {
        if !id.is_empty() && !is_quoted_scalar(id) {
            violations.push(Violation::UnquotedId);
        }
    }
}

fn check_schema_version(
    entries: &[(String, String)],
    violations: &mut Vec<Violation>,
) {
    if let Some(schema_version) = raw_value(entries, "schema_version") {
        if !is_bare_one(schema_version) {
            violations.push(Violation::BadSchemaVersion);
        }
    }
}

fn check_timestamps(
    entries: &[(String, String)],
    violations: &mut Vec<Violation>,
) {
    for field in ["date", "last_updated"] {
        let Some(raw) = raw_value(entries, field) else {
            continue;
        };
        let inner = strip_surrounding_quote(raw);
        if !inner.is_empty() && !is_iso_timestamp(inner) {
            violations.push(Violation::BadTimestamp {
                field,
                value: inner.to_owned(),
            });
        }
    }
}

fn check_status(
    entries: &[(String, String)],
    row: &schema::SchemaRow,
    violations: &mut Vec<Violation>,
) {
    if let Some(field_value) = raw_value(entries, "status") {
        let inner = strip_surrounding_quote(field_value);
        if !inner.is_empty() && !row.status_vocab.contains(&inner) {
            violations.push(Violation::BadStatus {
                value: inner.to_owned(),
                vocab: row.status_vocab.join(" | "),
            });
        }
    }
}

fn check_provenance(
    entries: &[(String, String)],
    row: &schema::SchemaRow,
    violations: &mut Vec<Violation>,
) {
    if row.code_state_anchored {
        for field in schema::PROVENANCE_FIELDS {
            if !is_present(entries, field) {
                violations.push(Violation::MissingProvenance { field });
            }
        }
    } else {
        for field in schema::PROVENANCE_FIELDS {
            if is_present(entries, field) {
                violations.push(Violation::ProvenanceOnNonAnchored { field });
            }
        }
    }
    for field in schema::FORBIDDEN_PROVENANCE_FIELDS {
        if is_present(entries, field) {
            violations.push(Violation::ForbiddenProvenance { field });
        }
    }
}

fn check_forbidden_own_id(
    entries: &[(String, String)],
    row: &schema::SchemaRow,
    violations: &mut Vec<Violation>,
) {
    for key in row.forbidden_own_id_keys {
        if is_present(entries, key) {
            violations.push(Violation::ForbiddenOwnId { key });
        }
    }
}

fn check_obsolete_legacy_keys(
    entries: &[(String, String)],
    violations: &mut Vec<Violation>,
) {
    for key in schema::OBSOLETE_LEGACY_KEYS {
        if is_present(entries, key) {
            violations.push(Violation::ObsoleteLegacyKey { key });
        }
    }
}

fn check_required_extras(
    entries: &[(String, String)],
    row: &schema::SchemaRow,
    violations: &mut Vec<Violation>,
) {
    for extra in row.extras {
        if schema::OPTIONAL_EXTRAS.contains(extra) {
            continue;
        }
        if !is_present(entries, extra) {
            violations.push(Violation::MissingExtra {
                extra: (*extra).to_owned(),
            });
        }
    }
}

fn check_empty_placeholders(
    entries: &[(String, String)],
    violations: &mut Vec<Violation>,
) {
    for (key, value) in entries {
        if key == "tags" {
            continue;
        }
        if value == "\"\"" || value == "[]" {
            violations.push(Violation::EmptyPlaceholder { key: key.clone() });
        }
    }
}

fn check_linkage_shape(
    entries: &[(String, String)],
    row: &schema::SchemaRow,
    violations: &mut Vec<Violation>,
) {
    for key in row.typed_linkage_keys {
        let Some(field_value) = raw_value(entries, key) else {
            continue;
        };
        for element in linkage_elements(field_value) {
            match quoted_inner(element) {
                Some("") => {}
                Some(inner) => {
                    if !shape::is_well_formed(inner) {
                        violations.push(Violation::BadLinkageShape {
                            key: (*key).to_owned(),
                            value: inner.to_owned(),
                            quoted: true,
                        });
                    }
                }
                None => {
                    violations.push(Violation::BadLinkageShape {
                        key: (*key).to_owned(),
                        value: element.to_owned(),
                        quoted: false,
                    });
                }
            }
        }
    }
}

fn is_quoted_scalar(raw: &str) -> bool {
    let Some(rest) = raw.strip_prefix('"') else {
        return false;
    };
    let Some(close) = rest.find('"') else {
        return false;
    };
    let tail = &rest[close + 1..];
    tail.is_empty() || is_trailing_comment(tail)
}

fn is_bare_one(raw: &str) -> bool {
    let Some(rest) = raw.strip_prefix('1') else {
        return false;
    };
    rest.is_empty() || is_trailing_comment(rest)
}

fn is_trailing_comment(tail: &str) -> bool {
    let stripped = tail.trim_start();
    stripped.len() < tail.len() && stripped.starts_with('#')
}

/// The whole-corpus referential-integrity index.
///
/// Every `(type, id)` key a `Parsed` file resolves to, mapped to every file
/// that resolves to it — appending rather than overwriting, so a genuine
/// collision (feeding [`duplicate_check`]) is visible instead of silently
/// losing an entry, as bash's associative-array-free `build_index` did.
#[derive(Debug, Default, Clone)]
pub struct Index(HashMap<String, Vec<PathBuf>>);

impl Index {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, type_id: String, path: PathBuf) {
        self.0.entry(type_id).or_default().push(path);
    }

    #[must_use]
    pub fn contains(&self, type_id: &str) -> bool {
        self.0.contains_key(type_id)
    }

    #[must_use]
    pub fn count(&self, type_id: &str) -> usize {
        self.0.get(type_id).map_or(0, Vec::len)
    }
}

/// The referential-integrity violations for one file's typed-linkage values.
///
/// Every well-formed, non-`pr:`-prefixed reference must resolve to a real
/// corpus artifact. Returns nothing when the type is missing or
/// unrecognised — the caller has already reported (or chosen not to
/// report) `INVALID-TYPE`; this never re-derives it. Malformed elements are
/// silently skipped here (already reported by [`validate_file`]).
#[must_use]
pub fn dangling_refs(raw_frontmatter: &str, index: &Index) -> Vec<Violation> {
    let entries = parse_entries(raw_frontmatter);
    let Some(declared) = declared_type(&entries) else {
        return Vec::new();
    };
    let Some(row) = schema::row_for(declared) else {
        return Vec::new();
    };

    let mut violations = Vec::new();
    for key in row.typed_linkage_keys {
        let Some(field_value) = raw_value(&entries, key) else {
            continue;
        };
        for element in linkage_elements(field_value) {
            let Some(inner) = quoted_inner(element) else {
                continue;
            };
            if inner.is_empty() || !shape::is_well_formed(inner) {
                continue;
            }
            if inner.starts_with("pr:") {
                continue;
            }
            if !index.contains(inner) {
                violations.push(Violation::DanglingRef {
                    key: (*key).to_owned(),
                    value: inner.to_owned(),
                });
            }
        }
    }
    violations
}

/// Whether `type_id` (the file's own resolved `(type, id)` key) is claimed
/// by more than one file in `index` — the referential-integrity check
/// bash's own index had no way to express.
#[must_use]
pub fn duplicate_check(type_id: &str, index: &Index) -> Option<Violation> {
    (index.count(type_id) > 1).then(|| Violation::DuplicateId {
        type_id: type_id.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        dangling_refs, duplicate_check, parse_entries, validate_file, Index,
        Violation,
    };

    fn minimal_valid_work_item() -> String {
        [
            "type: work-item",
            "id: \"0042\"",
            "title: Example",
            "date: \"2026-01-01T00:00:00+00:00\"",
            "author: Toby",
            "tags: []",
            "last_updated: \"2026-01-01T00:00:00+00:00\"",
            "last_updated_by: Toby",
            "schema_version: 1",
            "status: draft",
            "kind: feature",
            "priority: normal",
        ]
        .join("\n")
    }

    #[test]
    fn a_conforming_work_item_has_no_violations() {
        assert!(validate_file(&minimal_valid_work_item()).is_empty());
    }

    #[test]
    fn an_absent_type_is_invalid_type_and_nothing_else() {
        let violations = validate_file("id: \"0042\"\n");
        assert_eq!(
            violations,
            vec![Violation::InvalidType {
                found: String::new()
            }]
        );
    }

    #[test]
    fn an_unrecognised_type_short_circuits_every_other_check() {
        let violations = validate_file("type: not-a-real-type\n");
        assert_eq!(
            violations,
            vec![Violation::InvalidType {
                found: "not-a-real-type".to_owned()
            }]
        );
    }

    #[test]
    fn missing_base_fields_are_reported_in_schema_order() {
        let violations = validate_file("type: note\ntopic: x\n");
        let fields: Vec<&str> = violations
            .iter()
            .filter_map(|violation| match violation {
                Violation::MissingBaseField { field } => Some(*field),
                _ => None,
            })
            .collect();
        assert_eq!(
            fields,
            vec![
                "id",
                "title",
                "date",
                "author",
                "tags",
                "last_updated",
                "last_updated_by",
                "schema_version",
            ]
        );
    }

    #[test]
    fn an_unquoted_numeric_id_is_flagged() {
        let mut source = minimal_valid_work_item();
        source = source.replace("id: \"0042\"", "id: 0042");
        assert!(validate_file(&source).contains(&Violation::UnquotedId));
    }

    #[test]
    fn a_quoted_empty_id_does_not_trigger_unquoted_id() {
        // The bash quirk: an explicitly-quoted empty string still counts as
        // "quoted" — UNQUOTED-ID is about quote *syntax*, not content.
        let mut source = minimal_valid_work_item();
        source = source.replace("id: \"0042\"", "id: \"\"");
        assert!(!validate_file(&source).contains(&Violation::UnquotedId));
    }

    #[test]
    fn a_truly_empty_id_value_triggers_neither_unquoted_id_nor_empty_placeholder(
    ) {
        // bash's own gap, preserved rather than fixed: `id:` with nothing
        // after the colon short-circuits both checks (`-n "$BK_VAL"` guards
        // UNQUOTED-ID; the value isn't literally `""` so EMPTY-PLACEHOLDER
        // doesn't fire either).
        let mut source = minimal_valid_work_item();
        source = source.replace("id: \"0042\"", "id:");
        let violations = validate_file(&source);
        assert!(!violations.contains(&Violation::UnquotedId));
        assert!(!violations.iter().any(
            |v| matches!(v, Violation::EmptyPlaceholder { key } if key == "id")
        ));
    }

    #[test]
    fn schema_version_must_be_the_bare_integer_one() {
        let mut source = minimal_valid_work_item();
        source = source.replace("schema_version: 1", "schema_version: \"1\"");
        assert!(validate_file(&source).contains(&Violation::BadSchemaVersion));
    }

    #[test]
    fn a_non_iso_timestamp_is_flagged() {
        let mut source = minimal_valid_work_item();
        source = source.replace(
            "date: \"2026-01-01T00:00:00+00:00\"",
            "date: \"2026-01-01\"",
        );
        assert!(validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::BadTimestamp { field, .. } if *field == "date")));
    }

    #[test]
    fn a_zulu_iso_timestamp_is_accepted() {
        let mut source = minimal_valid_work_item();
        source = source.replace(
            "date: \"2026-01-01T00:00:00+00:00\"",
            "date: \"2026-01-01T00:00:00Z\"",
        );
        assert!(!validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::BadTimestamp { .. })));
    }

    #[test]
    fn a_status_outside_the_vocab_is_flagged() {
        let mut source = minimal_valid_work_item();
        source = source.replace("status: draft", "status: nonexistent");
        assert!(validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::BadStatus { .. })));
    }

    #[test]
    fn an_anchored_type_missing_provenance_is_flagged() {
        let violations = validate_file(
            "type: plan\nid: \"x\"\ntitle: t\ndate: \"2026-01-01T00:00:00Z\"\n\
             author: a\ntags: []\nlast_updated: \"2026-01-01T00:00:00Z\"\n\
             last_updated_by: a\nschema_version: 1\nstatus: draft\n",
        );
        assert!(violations
            .iter()
            .any(|v| matches!(v, Violation::MissingProvenance { field } if *field == "revision")));
    }

    #[test]
    fn a_non_anchored_type_carrying_provenance_is_flagged() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nrevision: abc123\n");
        assert!(validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::ProvenanceOnNonAnchored { field } if *field == "revision")));
    }

    #[test]
    fn a_forbidden_provenance_field_is_always_flagged() {
        let mut source = minimal_valid_work_item();
        source.push_str("\ngit_commit: abc123\n");
        assert!(validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::ForbiddenProvenance { field } if *field == "git_commit")));
    }

    #[test]
    fn a_forbidden_own_id_key_is_flagged() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nwork_item_id: \"0042\"\n");
        assert!(validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::ForbiddenOwnId { key } if *key == "work_item_id")));
    }

    #[test]
    fn pr_review_carries_both_of_its_forbidden_own_id_keys() {
        let source = "type: pr-review\nid: \"1-review-1\"\ntitle: t\n\
             date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
             schema_version: 1\nstatus: complete\nreviewer: a\nverdict: v\n\
             lenses: []\nreview_number: 1\npr_number: \"1\"\n\
             pr_title: x\nreview_pass: 1\n";
        let violations = validate_file(source);
        assert!(violations
            .iter()
            .any(|v| matches!(v, Violation::ForbiddenOwnId { key } if *key == "pr_title")));
        assert!(violations
            .iter()
            .any(|v| matches!(v, Violation::ForbiddenOwnId { key } if *key == "review_pass")));
    }

    #[test]
    fn an_obsolete_legacy_key_is_flagged() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nticket: FOO-1\n");
        assert!(validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::ObsoleteLegacyKey { key } if *key == "ticket")));
    }

    #[test]
    fn a_missing_required_extra_is_flagged() {
        let source = "type: note\nid: \"x\"\ntitle: t\n\
             date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
             schema_version: 1\nstatus: captured\n";
        assert!(validate_file(source)
            .iter()
            .any(|v| matches!(v, Violation::MissingExtra { extra } if extra == "topic")));
    }

    #[test]
    fn an_optional_extra_absent_is_not_required() {
        // reviewer is optional on a plan.
        let violations = validate_file(
            "type: plan\nid: \"x\"\ntitle: t\ndate: \"2026-01-01T00:00:00Z\"\n\
             author: a\ntags: []\nlast_updated: \"2026-01-01T00:00:00Z\"\n\
             last_updated_by: a\nschema_version: 1\nstatus: draft\n\
             revision: abc\nrepository: r\n",
        );
        assert!(!violations
            .iter()
            .any(|v| matches!(v, Violation::MissingExtra { .. })));
    }

    #[test]
    fn an_empty_placeholder_string_is_flagged() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nexternal_id: \"\"\n");
        assert!(validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::EmptyPlaceholder { key } if key == "external_id")));
    }

    #[test]
    fn an_empty_placeholder_list_is_flagged() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nrelates_to: []\n");
        // relates_to is a typed-linkage key; an empty list is a legitimate
        // absence for linkage purposes but still an EMPTY-PLACEHOLDER
        // literal-text hit, matching bash.
        let violations = validate_file(&source);
        assert!(violations
            .iter()
            .any(|v| matches!(v, Violation::EmptyPlaceholder { key } if key == "relates_to")));
    }

    #[test]
    fn tags_is_exempt_from_empty_placeholder() {
        assert!(!validate_file(&minimal_valid_work_item())
            .iter()
            .any(|v| matches!(v, Violation::EmptyPlaceholder { .. })));
    }

    #[test]
    fn a_double_quoted_typed_ref_is_accepted() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nparent: \"work-item:0001\"\n");
        assert!(!validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::BadLinkageShape { .. })));
    }

    #[test]
    fn an_unquoted_typed_ref_is_rejected() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nparent: work-item:0001\n");
        assert!(validate_file(&source).iter().any(
            |v| matches!(v, Violation::BadLinkageShape { quoted, .. } if !quoted)
        ));
    }

    #[test]
    fn a_single_quoted_typed_ref_is_rejected_as_unquoted() {
        // The documented asymmetry: only double quotes count.
        let mut source = minimal_valid_work_item();
        source.push_str("\nparent: 'work-item:0001'\n");
        assert!(validate_file(&source).iter().any(
            |v| matches!(v, Violation::BadLinkageShape { quoted, .. } if !quoted)
        ));
    }

    #[test]
    fn a_bare_path_shaped_linkage_value_is_rejected() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nparent: \"meta/work/0001-x.md\"\n");
        assert!(validate_file(&source).iter().any(|v| matches!(
            v,
            Violation::BadLinkageShape { quoted: true, .. }
        )));
    }

    #[test]
    fn a_multi_element_quoted_list_is_accepted() {
        let mut source = minimal_valid_work_item();
        source
            .push_str("\nrelates_to: [\"work-item:0001\", \"adr:ADR-0001\"]\n");
        assert!(!validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::BadLinkageShape { .. })));
    }

    #[test]
    fn a_mixed_list_with_one_unquoted_element_is_rejected() {
        let mut source = minimal_valid_work_item();
        source.push_str("\nrelates_to: [\"work-item:0001\", adr:ADR-0001]\n");
        assert!(validate_file(&source).iter().any(|v| matches!(
            v,
            Violation::BadLinkageShape { quoted: false, .. }
        )));
    }

    #[test]
    fn a_trailing_inline_comment_on_a_linkage_list_is_ignored() {
        let mut source = minimal_valid_work_item();
        source.push_str(
            "\nrelates_to: [\"work-item:0001\"] # a trailing comment\n",
        );
        assert!(!validate_file(&source)
            .iter()
            .any(|v| matches!(v, Violation::BadLinkageShape { .. })));
    }

    #[test]
    fn multiple_violations_on_one_file_are_all_reported() {
        let violations = validate_file("type: note\ntopic: x\n");
        assert!(violations.len() > 1);
    }

    #[test]
    fn parse_entries_takes_the_first_occurrence_of_a_duplicate_key() {
        let entries = parse_entries("status: proposed\nstatus: accepted\n");
        assert_eq!(super::raw_value(&entries, "status"), Some("proposed"));
    }

    #[test]
    fn parse_entries_ignores_indented_block_list_items() {
        let entries = parse_entries("relates_to:\n  - work-item:0001\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(super::raw_value(&entries, "relates_to"), Some(""));
    }

    #[test]
    fn dangling_refs_is_empty_for_an_invalid_type() {
        assert!(dangling_refs("type: bogus\n", &Index::new()).is_empty());
    }

    #[test]
    fn dangling_refs_tolerates_a_pr_prefixed_reference() {
        let source = "type: work-item\nrelates_to: [\"pr:42\"]\n";
        assert!(dangling_refs(source, &Index::new()).is_empty());
    }

    #[test]
    fn dangling_refs_flags_a_reference_absent_from_the_index() {
        let source = "type: work-item\nparent: [\"work-item:0001\"]\n";
        let violations = dangling_refs(source, &Index::new());
        assert_eq!(
            violations,
            vec![Violation::DanglingRef {
                key: "parent".to_owned(),
                value: "work-item:0001".to_owned(),
            }]
        );
    }

    #[test]
    fn dangling_refs_accepts_a_reference_present_in_the_index() {
        let mut index = Index::new();
        index.insert(
            "work-item:0001".to_owned(),
            std::path::PathBuf::from("meta/work/0001-x.md"),
        );
        let source = "type: work-item\nparent: [\"work-item:0001\"]\n";
        assert!(dangling_refs(source, &index).is_empty());
    }

    #[test]
    fn dangling_refs_skips_a_malformed_element_already_reported_elsewhere() {
        let source = "type: work-item\nparent: [unquoted]\n";
        assert!(dangling_refs(source, &Index::new()).is_empty());
    }

    #[test]
    fn duplicate_check_fires_only_above_one_entry() {
        let mut index = Index::new();
        let path = std::path::PathBuf::from("meta/work/0001-a.md");
        index.insert("work-item:0001".to_owned(), path.clone());
        assert_eq!(duplicate_check("work-item:0001", &index), None);

        index.insert("work-item:0001".to_owned(), path);
        assert_eq!(
            duplicate_check("work-item:0001", &index),
            Some(Violation::DuplicateId {
                type_id: "work-item:0001".to_owned()
            })
        );
    }
}
