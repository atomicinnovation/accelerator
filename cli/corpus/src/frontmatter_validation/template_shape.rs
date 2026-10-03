//! Template-shape validation for the `templates/*.md` skeletons.
//!
//! A template is not a populated document, so the instance validator
//! ([`super::validate_file`]) structurally rejects it. This module owns the
//! parallel, template-only shape rules — base-field presence, the declared
//! type, the provenance bundle, per-type extras, the status-comment
//! vocabulary, the typed-linkage slot grammar, the closed linkage set, the
//! absence of any legacy own-identity key, and the work-item Schema-Reference
//! cross-check — plus the general canonical-quoting rule, so a hand-edited
//! template that drifts from canonical quoting is caught here rather than only
//! when a producer next emits from it.
//!
//! Pure logic: the filesystem walk (reading each `templates/<name>.md`) lives
//! in `corpus_adapters`. Its own [`TemplateViolation`] type keeps these
//! template-only variants out of the instance-validation [`super::Violation`]
//! enum, which no populated document could ever carry.

use core::fmt;

use crate::frontmatter_validation::canonical_quoting::is_canonically_quoted;
use crate::frontmatter_validation::canonical_quoting::is_quoted_scalar;
use crate::frontmatter_validation::is_bare_one;
use crate::frontmatter_validation::is_present;
use crate::frontmatter_validation::parse_entries;
use crate::frontmatter_validation::raw_value;
use crate::frontmatter_validation::schema::SchemaRow;
use crate::frontmatter_validation::schema::SCHEMA;
use crate::frontmatter_validation::strip_surrounding_quote;

/// The base fields every template must carry: the corpus base set plus the two
/// the template surface additionally pins (`producer`, `status`), which a
/// populated document may legitimately omit.
const TEMPLATE_BASE_FIELDS: [&str; 11] = [
    "type",
    "id",
    "title",
    "date",
    "author",
    "tags",
    "last_updated",
    "last_updated_by",
    "schema_version",
    "producer",
    "status",
];

const PROVENANCE_FIELDS: [&str; 2] = ["revision", "repository"];
const FORBIDDEN_PROVENANCE_FIELDS: [&str; 2] = ["git_commit", "branch"];

/// The typed-linkage source-type vocabulary; `pr` is the external-entity
/// prefix.
const SOURCE_TYPES: [&str; 15] = [
    "work-item",
    "plan",
    "adr",
    "pr",
    "note",
    "codebase-research",
    "issue-research",
    "pr-description",
    "design-inventory",
    "design-gap",
    "plan-validation",
    "plan-review",
    "work-item-review",
    "pr-review",
    "topic-research",
];

/// Every typed-linkage key name. `superseded_by` is a guard: no template
/// carries it, so the closed-set check rejects any template that adds it.
const LINKAGE_VOCABULARY: [&str; 9] = [
    "parent",
    "superseded_by",
    "target",
    "source",
    "supersedes",
    "blocks",
    "blocked_by",
    "derived_from",
    "relates_to",
];

const SINGLE_CARDINALITY: [&str; 4] =
    ["parent", "superseded_by", "target", "source"];
const LIST_CARDINALITY: [&str; 5] = [
    "supersedes",
    "blocks",
    "blocked_by",
    "derived_from",
    "relates_to",
];

/// The standalone guidance line a `blocked_by` slot must be accompanied by.
/// Carries a literal em dash (U+2014).
const INVERSE_GUIDANCE_LINE: &str = "# inverse of blocks — producers SHOULD \
     prefer writing blocks: on the canonical side";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateViolation {
    MissingTemplateFile { template: String },
    EmptyFrontmatter { template: String },
    MissingBaseField { template: String, field: String },
    WrongType { template: String, expected: String },
    WrongKind { template: String, expected: String },
    BadSchemaVersion { template: String },
    UnquotedId { template: String },
    ForbiddenOwnId { template: String, key: String },
    MissingProvenance { template: String, field: String },
    ForbiddenProvenance { template: String, field: String },
    MissingExtra { template: String, extra: String },
    BadLinkageSlot { template: String, key: String },
    UnknownLinkageKey { template: String, key: String },
    ClosedSetViolation { template: String, key: String },
    BadStatusVocab { template: String, vocab: String },
    UnquotedString { template: String, key: String },
    SchemaCrossCheck { work_item: String, schema: String },
}

impl TemplateViolation {
    /// The short code, e.g. `TEMPLATE-MISSING-BASE-FIELD`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingTemplateFile { .. } => "TEMPLATE-FILE-NOT-FOUND",
            Self::EmptyFrontmatter { .. } => "TEMPLATE-EMPTY-FRONTMATTER",
            Self::MissingBaseField { .. } => "TEMPLATE-MISSING-BASE-FIELD",
            Self::WrongType { .. } => "TEMPLATE-WRONG-TYPE",
            Self::WrongKind { .. } => "TEMPLATE-WRONG-KIND",
            Self::BadSchemaVersion { .. } => "TEMPLATE-BAD-SCHEMA-VERSION",
            Self::UnquotedId { .. } => "TEMPLATE-UNQUOTED-ID",
            Self::ForbiddenOwnId { .. } => "TEMPLATE-FORBIDDEN-OWN-ID",
            Self::MissingProvenance { .. } => "TEMPLATE-MISSING-PROVENANCE",
            Self::ForbiddenProvenance { .. } => "TEMPLATE-FORBIDDEN-PROVENANCE",
            Self::MissingExtra { .. } => "TEMPLATE-MISSING-EXTRA",
            Self::BadLinkageSlot { .. } => "TEMPLATE-BAD-LINKAGE-SLOT",
            Self::UnknownLinkageKey { .. } => "TEMPLATE-UNKNOWN-LINKAGE-KEY",
            Self::ClosedSetViolation { .. } => "TEMPLATE-CLOSED-SET",
            Self::BadStatusVocab { .. } => "TEMPLATE-BAD-STATUS-VOCAB",
            Self::UnquotedString { .. } => "TEMPLATE-UNQUOTED-STRING",
            Self::SchemaCrossCheck { .. } => "SCHEMA-CROSS-CHECK",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::MissingTemplateFile { template } => {
                format!("{template}: template file not found at templates/{template}")
            }
            Self::EmptyFrontmatter { template } => {
                format!("{template}: frontmatter block is empty or missing")
            }
            Self::MissingBaseField { template, field } => {
                format!("{template}: base field '{field}' missing")
            }
            Self::WrongType { template, expected } => {
                format!("{template}: type is not '{expected}'")
            }
            Self::WrongKind { template, expected } => {
                format!("{template}: kind is not '{expected}'")
            }
            Self::BadSchemaVersion { template } => {
                format!("{template}: schema_version is not bare integer 1")
            }
            Self::UnquotedId { template } => {
                format!("{template}: id value is not a quoted string")
            }
            Self::ForbiddenOwnId { template, key } => {
                format!("{template}: legacy own-id key '{key}' present")
            }
            Self::MissingProvenance { template, field } => {
                format!("{template}: provenance field '{field}' missing")
            }
            Self::ForbiddenProvenance { template, field } => {
                format!(
                    "{template}: forbidden provenance field '{field}' present"
                )
            }
            Self::MissingExtra { template, extra } => {
                format!("{template}: extra '{extra}' missing")
            }
            Self::BadLinkageSlot { template, key } => format!(
                "{template}: linkage slot '{key}' bad shape/comment (or \
                 missing inverse-guidance line)"
            ),
            Self::UnknownLinkageKey { template, key } => {
                format!("{template}: unknown linkage key '{key}'")
            }
            Self::ClosedSetViolation { template, key } => format!(
                "{template}: closed-set violated (linkage key '{key}' not in \
                 the schema row)"
            ),
            Self::BadStatusVocab { template, vocab } => format!(
                "{template}: status line missing pinned vocabulary '{vocab}'"
            ),
            Self::UnquotedString { template, key } => {
                format!(
                    "{template}: {key}: value must be a double-quoted string"
                )
            }
            Self::SchemaCrossCheck { work_item, schema } => format!(
                "work-item Schema Reference templates differ from SCHEMA \
                 (work-item={work_item}, schema={schema})"
            ),
        }
    }
}

impl fmt::Display for TemplateViolation {
    /// `<CODE> — <message>`, with a literal em dash (U+2014), matching
    /// [`super::Violation`]'s own formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} — {}", self.code(), self.message())
    }
}

fn is_fence(line: &str) -> bool {
    line.strip_prefix("---")
        .is_some_and(|rest| rest.chars().all(|c| c == ' ' || c == '\t'))
}

/// The frontmatter block: the lines between the first two `---` fences, with
/// CR bytes removed.
#[must_use]
pub fn extract_frontmatter(text: &str) -> String {
    let normalised = text.replace('\r', "");
    let mut collected: Vec<&str> = Vec::new();
    let mut seen_open = false;
    for line in normalised.split('\n') {
        if is_fence(line) {
            if seen_open {
                break;
            }
            seen_open = true;
            continue;
        }
        if seen_open {
            collected.push(line);
        }
    }
    collected.join("\n")
}

/// Every shape violation for one template's frontmatter block.
#[must_use]
pub fn validate_template(
    row: &SchemaRow,
    frontmatter: &str,
) -> Vec<TemplateViolation> {
    if frontmatter.trim().is_empty() {
        return vec![TemplateViolation::EmptyFrontmatter {
            template: row.template.to_owned(),
        }];
    }
    let entries = parse_entries(frontmatter);
    let mut found = Vec::new();

    check_presence(row, &entries, &mut found);
    check_kind(row, &entries, &mut found);
    check_own_id(row, &entries, &mut found);
    check_provenance(row, &entries, &mut found);
    check_extras(row, &entries, &mut found);
    check_linkage(row, frontmatter, &entries, &mut found);
    check_status(row, frontmatter, &mut found);
    check_canonical_quoting(row, &entries, &mut found);

    found
}

fn check_presence(
    row: &SchemaRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for field in TEMPLATE_BASE_FIELDS {
        if !is_present(entries, field) {
            found.push(TemplateViolation::MissingBaseField {
                template: row.template.to_owned(),
                field: field.to_owned(),
            });
        }
    }
    match raw_value(entries, "type") {
        Some(value) if strip_surrounding_quote(value) == row.linkage_type => {}
        _ => found.push(TemplateViolation::WrongType {
            template: row.template.to_owned(),
            expected: row.linkage_type.to_owned(),
        }),
    }
    if raw_value(entries, "schema_version")
        .is_none_or(|value| !is_bare_one(value))
    {
        found.push(TemplateViolation::BadSchemaVersion {
            template: row.template.to_owned(),
        });
    }
    if raw_value(entries, "id").is_none_or(|value| !is_quoted_scalar(value)) {
        found.push(TemplateViolation::UnquotedId {
            template: row.template.to_owned(),
        });
    }
}

/// Enforces the template's declared `kind:` against a kind-discriminated row.
///
/// Only fires for a row carrying a non-empty `kind`: a type-level default row
/// (`kind` empty) makes no demand, so the `kind` field a work-item template
/// legitimately carries as an extra is never mistaken for the discriminator.
fn check_kind(
    row: &SchemaRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    if row.kind.is_empty() {
        return;
    }
    let matches = raw_value(entries, "kind")
        .map(strip_surrounding_quote)
        .is_some_and(|declared| declared == row.kind);
    if !matches {
        found.push(TemplateViolation::WrongKind {
            template: row.template.to_owned(),
            expected: row.kind.to_owned(),
        });
    }
}

fn check_own_id(
    row: &SchemaRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for key in row.forbidden_own_id_keys {
        if is_present(entries, key) {
            found.push(TemplateViolation::ForbiddenOwnId {
                template: row.template.to_owned(),
                key: (*key).to_owned(),
            });
        }
    }
}

fn check_provenance(
    row: &SchemaRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    if row.code_state_anchored {
        for field in PROVENANCE_FIELDS {
            if !is_present(entries, field) {
                found.push(TemplateViolation::MissingProvenance {
                    template: row.template.to_owned(),
                    field: field.to_owned(),
                });
            }
        }
    }
    for field in FORBIDDEN_PROVENANCE_FIELDS {
        if is_present(entries, field) {
            found.push(TemplateViolation::ForbiddenProvenance {
                template: row.template.to_owned(),
                field: field.to_owned(),
            });
        }
    }
}

fn check_extras(
    row: &SchemaRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for extra in row.all_extras() {
        if !is_present(entries, extra) {
            found.push(TemplateViolation::MissingExtra {
                template: row.template.to_owned(),
                extra: extra.to_owned(),
            });
        }
    }
}

fn check_linkage(
    row: &SchemaRow,
    frontmatter: &str,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for &key in row.typed_linkage_keys {
        match check_linkage_slot(frontmatter, key) {
            SlotOutcome::Ok => {}
            SlotOutcome::Bad => {
                found.push(TemplateViolation::BadLinkageSlot {
                    template: row.template.to_owned(),
                    key: key.to_owned(),
                });
            }
            SlotOutcome::Unknown => {
                found.push(TemplateViolation::UnknownLinkageKey {
                    template: row.template.to_owned(),
                    key: key.to_owned(),
                });
            }
        }
    }
    let declared: Vec<&str> = row
        .typed_linkage_keys
        .iter()
        .copied()
        .chain(row.all_extras())
        .collect();
    for vkey in LINKAGE_VOCABULARY {
        if is_present(entries, vkey) && !declared.contains(&vkey) {
            found.push(TemplateViolation::ClosedSetViolation {
                template: row.template.to_owned(),
                key: vkey.to_owned(),
            });
        }
    }
}

fn check_status(
    row: &SchemaRow,
    frontmatter: &str,
    found: &mut Vec<TemplateViolation>,
) {
    let vocab = row.status_vocab.join(" | ");
    let status_line =
        frontmatter.lines().find(|line| line.starts_with("status:"));
    match status_line {
        Some(line) if line.contains(&vocab) => {}
        _ => found.push(TemplateViolation::BadStatusVocab {
            template: row.template.to_owned(),
            vocab,
        }),
    }
}

fn check_canonical_quoting(
    row: &SchemaRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for (key, value) in entries {
        if value.is_empty()
            || key == "id"
            || key == "schema_version"
            || row.typed_linkage_keys.contains(&key.as_str())
        {
            continue;
        }
        if !is_canonically_quoted(value) {
            found.push(TemplateViolation::UnquotedString {
                template: row.template.to_owned(),
                key: key.clone(),
            });
        }
    }
}

enum SlotOutcome {
    Ok,
    Bad,
    Unknown,
}

fn check_linkage_slot(frontmatter: &str, key: &str) -> SlotOutcome {
    let is_single = SINGLE_CARDINALITY.contains(&key);
    let is_list = LIST_CARDINALITY.contains(&key);
    if !is_single && !is_list {
        return SlotOutcome::Unknown;
    }
    let matched = frontmatter.lines().any(|line| {
        if is_single {
            single_slot_line_ok(line, key)
        } else {
            list_slot_line_ok(line, key)
        }
    });
    if !matched {
        return SlotOutcome::Bad;
    }
    if key == "blocked_by" && !frontmatter.contains(INVERSE_GUIDANCE_LINE) {
        return SlotOutcome::Bad;
    }
    SlotOutcome::Ok
}

fn single_slot_line_ok(line: &str, key: &str) -> bool {
    let head = format!("{key}:");
    let tokens: Vec<&str> = line.split_whitespace().collect();
    tokens.len() == 8
        && tokens[0] == head
        && tokens[1] == "\"\""
        && tokens[2] == "#"
        && tokens[3] == "typed-linkage"
        && tokens[4] == "ref:"
        && is_quoted_ref(tokens[5])
        && tokens[6] == "or"
        && tokens[7] == "\"\""
}

fn list_slot_line_ok(line: &str, key: &str) -> bool {
    let head = format!("{key}:");
    let tokens: Vec<&str> = line.split_whitespace().collect();
    tokens.len() == 9
        && tokens[0] == head
        && tokens[1] == "[]"
        && tokens[2] == "#"
        && tokens[3] == "typed-linkage"
        && tokens[4] == "list:"
        && is_list_ref(tokens[5])
        && tokens[6] == "...]"
        && tokens[7] == "or"
        && tokens[8] == "[]"
}

/// A `"<source-type>:<id>"` example token, as in a single-slot comment.
fn is_quoted_ref(token: &str) -> bool {
    token
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .is_some_and(is_typed_ref)
}

/// A `["<source-type>:<id>",` example token, as in a list-slot comment.
fn is_list_ref(token: &str) -> bool {
    token
        .strip_prefix("[\"")
        .and_then(|rest| rest.strip_suffix("\","))
        .is_some_and(is_typed_ref)
}

fn is_typed_ref(inner: &str) -> bool {
    let Some((source_type, id)) = inner.split_once(':') else {
        return false;
    };
    SOURCE_TYPES.contains(&source_type)
        && !id.is_empty()
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// The cross-check: the work-item Schema-Reference template names must equal
/// the template names [`SCHEMA`] carries.
#[must_use]
pub fn cross_check(schema_ref_templates: &[String]) -> Vec<TemplateViolation> {
    let mut reference = schema_ref_templates.to_vec();
    reference.sort();
    let mut schema: Vec<String> =
        SCHEMA.iter().map(|row| row.template.to_owned()).collect();
    schema.sort();
    if reference == schema {
        return Vec::new();
    }
    vec![TemplateViolation::SchemaCrossCheck {
        work_item: format!("{reference:?}"),
        schema: format!("{schema:?}"),
    }]
}

/// The template filenames named in a work item's `## Schema Reference` table
/// — each ``| `<name>.md` | … |`` row inside the section between that heading
/// and the next `## ` heading.
#[must_use]
pub fn schema_reference_templates(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_section = false;
    for line in content.lines() {
        if line.starts_with("## Schema Reference") {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            in_section = false;
        }
        if in_section {
            if let Some(name) = leading_backtick_template(line) {
                names.push(name);
            }
        }
    }
    names
}

fn leading_backtick_template(line: &str) -> Option<String> {
    let rest = line.strip_prefix('|')?.trim_start().strip_prefix('`')?;
    let end = rest.find('`')?;
    let name = &rest[..end];
    let stem = name.strip_suffix(".md")?;
    let well_formed = !stem.is_empty()
        && stem
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    well_formed.then(|| name.to_owned())
}

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::expect_used)]
mod tests {
    use super::{
        cross_check, extract_frontmatter, validate_template, TemplateViolation,
    };
    use crate::frontmatter_validation::schema::SchemaRow;
    use crate::frontmatter_validation::schema::SCHEMA;

    /// A conforming, canonically-quoted template body.
    fn conforming() -> String {
        [
            "---",
            "type: \"demo-type\"",
            "id: \"NNNN\"",
            "title: \"T\"",
            "date: \"2026-01-01T00:00:00+00:00\"",
            "author: \"A\"",
            "producer: \"create-demo\"",
            "status: \"captured\" # captured | archived",
            "tags: []",
            "last_updated: \"2026-01-01T00:00:00+00:00\"",
            "last_updated_by: \"A\"",
            "schema_version: 1",
            "parent: \"\" # typed-linkage ref: \"work-item:NNNN\" or \"\"",
            "---",
            "",
            "# body",
        ]
        .join("\n")
    }

    const fn demo_row() -> SchemaRow {
        SchemaRow {
            template: "demo.md",
            linkage_type: "demo-type",
            kind: "",
            code_state_anchored: false,
            required_extras: &[],
            optional_extras: &[],
            status_vocab: &["captured", "archived"],
            forbidden_own_id_keys: &[],
            typed_linkage_keys: &["parent"],
        }
    }

    fn check(row: &SchemaRow, body: &str) -> Vec<TemplateViolation> {
        validate_template(row, &extract_frontmatter(body))
    }

    fn any_code(violations: &[TemplateViolation], code: &str) -> bool {
        violations.iter().any(|v| v.code() == code)
    }

    #[test]
    fn a_conforming_canonical_template_yields_no_violations() {
        assert_eq!(check(&demo_row(), &conforming()), Vec::new());
    }

    #[test]
    fn a_missing_base_field_is_flagged() {
        let body = conforming().replace("producer: \"create-demo\"\n", "");
        assert!(any_code(
            &check(&demo_row(), &body),
            "TEMPLATE-MISSING-BASE-FIELD"
        ));
    }

    #[test]
    fn a_wrong_type_is_flagged() {
        let body =
            conforming().replace("type: \"demo-type\"", "type: \"wrong\"");
        assert!(any_code(&check(&demo_row(), &body), "TEMPLATE-WRONG-TYPE"));
    }

    #[test]
    fn a_kind_discriminated_row_requires_the_matching_kind() {
        let row = SchemaRow {
            kind: "finding",
            ..demo_row()
        };
        assert!(any_code(&check(&row, &conforming()), "TEMPLATE-WRONG-KIND"));
    }

    #[test]
    fn a_kind_discriminated_row_with_the_matching_kind_passes() {
        let row = SchemaRow {
            kind: "finding",
            ..demo_row()
        };
        let body = conforming()
            .replace("id: \"NNNN\"", "id: \"NNNN\"\nkind: \"finding\"");
        assert!(!any_code(&check(&row, &body), "TEMPLATE-WRONG-KIND"));
    }

    #[test]
    fn a_type_default_row_makes_no_kind_demand() {
        let body = conforming()
            .replace("id: \"NNNN\"", "id: \"NNNN\"\nkind: \"story\"");
        assert!(!any_code(&check(&demo_row(), &body), "TEMPLATE-WRONG-KIND"));
    }

    #[test]
    fn a_non_integer_schema_version_is_flagged() {
        let body =
            conforming().replace("schema_version: 1", "schema_version: 2");
        assert!(any_code(
            &check(&demo_row(), &body),
            "TEMPLATE-BAD-SCHEMA-VERSION"
        ));
    }

    #[test]
    fn an_unquoted_id_is_flagged() {
        let body = conforming().replace("id: \"NNNN\"", "id: NNNN");
        assert!(any_code(&check(&demo_row(), &body), "TEMPLATE-UNQUOTED-ID"));
    }

    #[test]
    fn a_bare_string_field_is_flagged_unquoted() {
        let body = conforming().replace("author: \"A\"", "author: A");
        assert!(any_code(
            &check(&demo_row(), &body),
            "TEMPLATE-UNQUOTED-STRING"
        ));
    }

    #[test]
    fn a_forbidden_own_id_key_is_flagged() {
        let row = SchemaRow {
            forbidden_own_id_keys: &["old_id"],
            ..demo_row()
        };
        let body = conforming()
            .replace("schema_version: 1", "schema_version: 1\nold_id: \"x\"");
        assert!(any_code(&check(&row, &body), "TEMPLATE-FORBIDDEN-OWN-ID"));
    }

    #[test]
    fn a_missing_provenance_bundle_is_flagged() {
        let row = SchemaRow {
            code_state_anchored: true,
            ..demo_row()
        };
        assert!(any_code(
            &check(&row, &conforming()),
            "TEMPLATE-MISSING-PROVENANCE"
        ));
    }

    #[test]
    fn a_forbidden_provenance_field_is_flagged() {
        let body = conforming().replace(
            "schema_version: 1",
            "schema_version: 1\ngit_commit: \"abc\"",
        );
        assert!(any_code(
            &check(&demo_row(), &body),
            "TEMPLATE-FORBIDDEN-PROVENANCE"
        ));
    }

    #[test]
    fn a_missing_extra_is_flagged() {
        let row = SchemaRow {
            required_extras: &["topic"],
            ..demo_row()
        };
        assert!(any_code(
            &check(&row, &conforming()),
            "TEMPLATE-MISSING-EXTRA"
        ));
    }

    #[test]
    fn a_template_missing_an_optional_extra_is_flagged() {
        let row = SchemaRow {
            optional_extras: &["reviewer"],
            ..demo_row()
        };
        assert!(any_code(
            &check(&row, &conforming()),
            "TEMPLATE-MISSING-EXTRA"
        ));
    }

    #[test]
    fn a_bad_linkage_slot_shape_is_flagged() {
        let body = conforming().replace(
            "# typed-linkage ref: \"work-item:NNNN\" or \"\"",
            "# see ADR-0034",
        );
        assert!(any_code(
            &check(&demo_row(), &body),
            "TEMPLATE-BAD-LINKAGE-SLOT"
        ));
    }

    #[test]
    fn an_out_of_vocabulary_source_type_is_flagged() {
        let body = conforming().replace("work-item:NNNN", "ticket:NNNN");
        assert!(any_code(
            &check(&demo_row(), &body),
            "TEMPLATE-BAD-LINKAGE-SLOT"
        ));
    }

    #[test]
    fn an_unknown_linkage_key_is_flagged() {
        let row = SchemaRow {
            typed_linkage_keys: &["bogus"],
            ..demo_row()
        };
        assert!(any_code(
            &check(&row, &conforming()),
            "TEMPLATE-UNKNOWN-LINKAGE-KEY"
        ));
    }

    #[test]
    fn a_spurious_linkage_key_violates_the_closed_set() {
        let extra = "relates_to: [] # typed-linkage list: \
             [\"work-item:NNNN\", ...] or []";
        let body = conforming().replace(
            "schema_version: 1",
            &format!("schema_version: 1\n{extra}"),
        );
        assert!(any_code(&check(&demo_row(), &body), "TEMPLATE-CLOSED-SET"));
    }

    #[test]
    fn a_wrong_status_vocabulary_is_flagged() {
        let body = conforming().replace("# captured | archived", "# draft");
        assert!(any_code(
            &check(&demo_row(), &body),
            "TEMPLATE-BAD-STATUS-VOCAB"
        ));
    }

    #[test]
    fn an_empty_frontmatter_is_flagged() {
        assert!(any_code(
            &validate_template(&demo_row(), ""),
            "TEMPLATE-EMPTY-FRONTMATTER"
        ));
    }

    #[test]
    fn a_list_slot_given_a_single_ref_value_is_flagged() {
        let row = SchemaRow {
            typed_linkage_keys: &["blocks"],
            ..demo_row()
        };
        let body = conforming().replace(
            "parent: \"\" # typed-linkage ref: \"work-item:NNNN\" or \"\"",
            "blocks: \"\" # typed-linkage list: \
             [\"work-item:NNNN\", ...] or []",
        );
        assert!(any_code(&check(&row, &body), "TEMPLATE-BAD-LINKAGE-SLOT"));
    }

    #[test]
    fn a_blocked_by_slot_missing_the_inverse_line_is_flagged() {
        let row = SchemaRow {
            typed_linkage_keys: &["blocked_by"],
            ..demo_row()
        };
        let body = conforming().replace(
            "parent: \"\" # typed-linkage ref: \"work-item:NNNN\" or \"\"",
            "blocked_by: [] # typed-linkage list: \
             [\"work-item:NNNN\", ...] or []",
        );
        assert!(any_code(&check(&row, &body), "TEMPLATE-BAD-LINKAGE-SLOT"));
    }

    #[test]
    fn a_blocked_by_slot_with_the_inverse_line_passes() {
        let row = SchemaRow {
            typed_linkage_keys: &["blocked_by"],
            ..demo_row()
        };
        let inverse = "# inverse of blocks — producers SHOULD prefer writing \
             blocks: on the canonical side";
        let slot = "blocked_by: [] # typed-linkage list: \
             [\"work-item:NNNN\", ...] or []";
        let body = conforming().replace(
            "parent: \"\" # typed-linkage ref: \"work-item:NNNN\" or \"\"",
            &format!("{slot}\n{inverse}"),
        );
        assert!(!any_code(&check(&row, &body), "TEMPLATE-BAD-LINKAGE-SLOT"));
    }

    #[test]
    fn cross_check_passes_on_the_schemas_template_set_in_any_order() {
        let reference: Vec<String> = SCHEMA
            .iter()
            .rev()
            .map(|row| row.template.to_owned())
            .collect();
        assert!(cross_check(&reference).is_empty());
    }

    #[test]
    fn cross_check_flags_a_divergent_set() {
        let reference = vec!["a.md".to_owned()];
        assert!(!cross_check(&reference).is_empty());
    }

    #[test]
    fn extract_frontmatter_returns_the_block_between_fences() {
        let text = "---\ntype: x\n---\nbody\n";
        assert_eq!(extract_frontmatter(text), "type: x");
    }

    #[test]
    fn extract_frontmatter_normalises_crlf() {
        let text = "---\r\ntype: x\r\n---\r\nbody\r\n";
        assert_eq!(extract_frontmatter(text), "type: x");
    }
}
