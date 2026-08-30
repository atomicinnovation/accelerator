//! Template-shape validation for the `templates/*.md` skeletons.
//!
//! A template is not a populated document, so the instance validator
//! ([`super::validate_file`]) structurally rejects it. This module owns the
//! parallel, template-only shape rules — base-field presence, the declared
//! type, the provenance bundle, per-type extras, the status-comment
//! vocabulary, the typed-linkage slot grammar, the closed linkage set, the
//! absence of any legacy own-identity key, the schema-TSV field-count
//! self-check, and the work-item Schema-Reference cross-check — plus the
//! general canonical-quoting rule, so a hand-edited template that drifts from
//! canonical quoting is caught here rather than only when a producer next
//! emits from it.
//!
//! Pure logic: the filesystem walk (reading each `templates/<name>.md` and the
//! TSV) lives in `corpus_adapters`. Its own [`TemplateViolation`] type keeps
//! these template-only variants out of the instance-validation
//! [`super::Violation`] enum, which no populated document could ever carry.

use core::fmt;

use crate::frontmatter_validation::canonical_quoting::is_canonically_quoted;
use crate::frontmatter_validation::canonical_quoting::is_quoted_scalar;
use crate::frontmatter_validation::is_bare_one;
use crate::frontmatter_validation::is_present;
use crate::frontmatter_validation::parse_entries;
use crate::frontmatter_validation::raw_value;
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
const SOURCE_TYPES: [&str; 14] = [
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

/// The schema-TSV's tab-field count (its 7 columns).
const SCHEMA_TAB_FIELDS: usize = 7;

/// One `templates-schema.tsv` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateRow {
    pub template: String,
    pub doc_type: String,
    pub code_state_anchored: bool,
    pub extras: Vec<String>,
    pub status_vocab: String,
    pub forbidden_own_id_keys: Vec<String>,
    pub typed_linkage_keys: Vec<String>,
}

/// The committed template schema, embedded so the check ships with the binary
/// and its field-count self-check runs over the exact bytes under test.
pub const TEMPLATES_SCHEMA_TSV: &str = include_str!("templates-schema.tsv");

/// A single template-shape (or schema-self-check) violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateViolation {
    MissingTemplateFile { template: String },
    EmptyFrontmatter { template: String },
    MissingBaseField { template: String, field: String },
    WrongType { template: String, expected: String },
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
    SchemaNoRows,
    SchemaFieldCount { line: usize, found: usize },
    SchemaCrossCheck { work_item: String, tsv: String },
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
            Self::SchemaNoRows => "SCHEMA-NO-ROWS",
            Self::SchemaFieldCount { .. } => "SCHEMA-FIELD-COUNT",
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
                 the TSV row)"
            ),
            Self::BadStatusVocab { template, vocab } => format!(
                "{template}: status line missing pinned vocabulary '{vocab}'"
            ),
            Self::UnquotedString { template, key } => {
                format!(
                    "{template}: {key}: value must be a double-quoted string"
                )
            }
            Self::SchemaNoRows => "templates-schema.tsv has no rows".to_owned(),
            Self::SchemaFieldCount { line, found } => format!(
                "templates-schema.tsv:{line} has {found} fields, expected \
                 {SCHEMA_TAB_FIELDS}"
            ),
            Self::SchemaCrossCheck { work_item, tsv } => format!(
                "work-item Schema Reference templates differ from TSV \
                 (work-item={work_item}, tsv={tsv})"
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

fn split_ws(field: &str) -> Vec<String> {
    field.split_whitespace().map(str::to_owned).collect()
}

/// Parses the whole `templates-schema.tsv`.
///
/// # Errors
///
/// [`TemplateViolation::SchemaNoRows`] when no data row follows the header;
/// [`TemplateViolation::SchemaFieldCount`] for the first line whose tab-field
/// count is not 7.
pub fn parse_schema_tsv(
    content: &str,
) -> Result<Vec<TemplateRow>, TemplateViolation> {
    let lines: Vec<&str> = content.lines().collect();
    let has_data = lines.iter().skip(1).any(|line| !line.trim().is_empty());
    if !has_data {
        return Err(TemplateViolation::SchemaNoRows);
    }
    for (index, line) in lines.iter().enumerate() {
        let found = line.split('\t').count();
        if found != SCHEMA_TAB_FIELDS {
            return Err(TemplateViolation::SchemaFieldCount {
                line: index + 1,
                found,
            });
        }
    }
    let rows = lines
        .iter()
        .skip(1)
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            TemplateRow {
                template: fields[0].to_owned(),
                doc_type: fields[1].to_owned(),
                code_state_anchored: fields[2] == "yes",
                extras: split_ws(fields[3]),
                status_vocab: fields[4].to_owned(),
                forbidden_own_id_keys: if fields[5] == "-" {
                    Vec::new()
                } else {
                    split_ws(fields[5])
                },
                typed_linkage_keys: split_ws(fields[6]),
            }
        })
        .collect();
    Ok(rows)
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
    row: &TemplateRow,
    frontmatter: &str,
) -> Vec<TemplateViolation> {
    if frontmatter.trim().is_empty() {
        return vec![TemplateViolation::EmptyFrontmatter {
            template: row.template.clone(),
        }];
    }
    let entries = parse_entries(frontmatter);
    let mut found = Vec::new();

    check_presence(row, &entries, &mut found);
    check_own_id(row, &entries, &mut found);
    check_provenance(row, &entries, &mut found);
    check_extras(row, &entries, &mut found);
    check_linkage(row, frontmatter, &entries, &mut found);
    check_status(row, frontmatter, &mut found);
    check_canonical_quoting(row, &entries, &mut found);

    found
}

fn check_presence(
    row: &TemplateRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for field in TEMPLATE_BASE_FIELDS {
        if !is_present(entries, field) {
            found.push(TemplateViolation::MissingBaseField {
                template: row.template.clone(),
                field: field.to_owned(),
            });
        }
    }
    match raw_value(entries, "type") {
        Some(value) if strip_surrounding_quote(value) == row.doc_type => {}
        _ => found.push(TemplateViolation::WrongType {
            template: row.template.clone(),
            expected: row.doc_type.clone(),
        }),
    }
    if raw_value(entries, "schema_version")
        .is_none_or(|value| !is_bare_one(value))
    {
        found.push(TemplateViolation::BadSchemaVersion {
            template: row.template.clone(),
        });
    }
    if raw_value(entries, "id").is_none_or(|value| !is_quoted_scalar(value)) {
        found.push(TemplateViolation::UnquotedId {
            template: row.template.clone(),
        });
    }
}

fn check_own_id(
    row: &TemplateRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for key in &row.forbidden_own_id_keys {
        if is_present(entries, key) {
            found.push(TemplateViolation::ForbiddenOwnId {
                template: row.template.clone(),
                key: key.clone(),
            });
        }
    }
}

fn check_provenance(
    row: &TemplateRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    if row.code_state_anchored {
        for field in PROVENANCE_FIELDS {
            if !is_present(entries, field) {
                found.push(TemplateViolation::MissingProvenance {
                    template: row.template.clone(),
                    field: field.to_owned(),
                });
            }
        }
    }
    for field in FORBIDDEN_PROVENANCE_FIELDS {
        if is_present(entries, field) {
            found.push(TemplateViolation::ForbiddenProvenance {
                template: row.template.clone(),
                field: field.to_owned(),
            });
        }
    }
}

fn check_extras(
    row: &TemplateRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for extra in &row.extras {
        if !is_present(entries, extra) {
            found.push(TemplateViolation::MissingExtra {
                template: row.template.clone(),
                extra: extra.clone(),
            });
        }
    }
}

fn check_linkage(
    row: &TemplateRow,
    frontmatter: &str,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for key in &row.typed_linkage_keys {
        match check_linkage_slot(frontmatter, key) {
            SlotOutcome::Ok => {}
            SlotOutcome::Bad => {
                found.push(TemplateViolation::BadLinkageSlot {
                    template: row.template.clone(),
                    key: key.clone(),
                });
            }
            SlotOutcome::Unknown => {
                found.push(TemplateViolation::UnknownLinkageKey {
                    template: row.template.clone(),
                    key: key.clone(),
                });
            }
        }
    }
    let declared: Vec<&str> = row
        .typed_linkage_keys
        .iter()
        .chain(row.extras.iter())
        .map(String::as_str)
        .collect();
    for vkey in LINKAGE_VOCABULARY {
        if is_present(entries, vkey) && !declared.contains(&vkey) {
            found.push(TemplateViolation::ClosedSetViolation {
                template: row.template.clone(),
                key: vkey.to_owned(),
            });
        }
    }
}

fn check_status(
    row: &TemplateRow,
    frontmatter: &str,
    found: &mut Vec<TemplateViolation>,
) {
    let status_line =
        frontmatter.lines().find(|line| line.starts_with("status:"));
    match status_line {
        Some(line) if line.contains(&row.status_vocab) => {}
        _ => found.push(TemplateViolation::BadStatusVocab {
            template: row.template.clone(),
            vocab: row.status_vocab.clone(),
        }),
    }
}

fn check_canonical_quoting(
    row: &TemplateRow,
    entries: &[(String, String)],
    found: &mut Vec<TemplateViolation>,
) {
    for (key, value) in entries {
        if value.is_empty()
            || key == "id"
            || key == "schema_version"
            || row.typed_linkage_keys.iter().any(|slot| slot == key)
        {
            continue;
        }
        if !is_canonically_quoted(value) {
            found.push(TemplateViolation::UnquotedString {
                template: row.template.clone(),
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
/// the TSV's template names.
#[must_use]
pub fn cross_check(
    schema_ref_templates: &[String],
    tsv_templates: &[String],
) -> Vec<TemplateViolation> {
    let mut reference = schema_ref_templates.to_vec();
    reference.sort();
    let mut tsv = tsv_templates.to_vec();
    tsv.sort();
    if reference == tsv {
        return Vec::new();
    }
    vec![TemplateViolation::SchemaCrossCheck {
        work_item: format!("{reference:?}"),
        tsv: format!("{tsv:?}"),
    }]
}

/// The template filenames named in a work item's `## Schema Reference` table
/// — each `| `<name>.md` | … |` row inside the section between that heading
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
        cross_check, extract_frontmatter, parse_schema_tsv, validate_template,
        TemplateRow, TemplateViolation,
    };

    const HEADER: &str = "template\ttype\tcode_state_anchored\textras\t\
         status_vocab\tforbidden_own_id_key\ttyped_linkage_keys";

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

    fn demo_row() -> TemplateRow {
        TemplateRow {
            template: "demo.md".to_owned(),
            doc_type: "demo-type".to_owned(),
            code_state_anchored: false,
            extras: Vec::new(),
            status_vocab: "captured | archived".to_owned(),
            forbidden_own_id_keys: Vec::new(),
            typed_linkage_keys: vec!["parent".to_owned()],
        }
    }

    fn check(row: &TemplateRow, body: &str) -> Vec<TemplateViolation> {
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
        let mut row = demo_row();
        row.forbidden_own_id_keys = vec!["old_id".to_owned()];
        let body = conforming()
            .replace("schema_version: 1", "schema_version: 1\nold_id: \"x\"");
        assert!(any_code(&check(&row, &body), "TEMPLATE-FORBIDDEN-OWN-ID"));
    }

    #[test]
    fn a_missing_provenance_bundle_is_flagged() {
        let mut row = demo_row();
        row.code_state_anchored = true;
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
        let mut row = demo_row();
        row.extras = vec!["topic".to_owned()];
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
        let mut row = demo_row();
        row.typed_linkage_keys = vec!["bogus".to_owned()];
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
        let mut row = demo_row();
        row.typed_linkage_keys = vec!["blocks".to_owned()];
        let body = conforming().replace(
            "parent: \"\" # typed-linkage ref: \"work-item:NNNN\" or \"\"",
            "blocks: \"\" # typed-linkage list: \
             [\"work-item:NNNN\", ...] or []",
        );
        assert!(any_code(&check(&row, &body), "TEMPLATE-BAD-LINKAGE-SLOT"));
    }

    #[test]
    fn a_blocked_by_slot_missing_the_inverse_line_is_flagged() {
        let mut row = demo_row();
        row.typed_linkage_keys = vec!["blocked_by".to_owned()];
        let body = conforming().replace(
            "parent: \"\" # typed-linkage ref: \"work-item:NNNN\" or \"\"",
            "blocked_by: [] # typed-linkage list: \
             [\"work-item:NNNN\", ...] or []",
        );
        assert!(any_code(&check(&row, &body), "TEMPLATE-BAD-LINKAGE-SLOT"));
    }

    #[test]
    fn a_blocked_by_slot_with_the_inverse_line_passes() {
        let mut row = demo_row();
        row.typed_linkage_keys = vec!["blocked_by".to_owned()];
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
    fn parse_schema_tsv_flags_a_short_row() {
        let tsv = format!("{HEADER}\ndemo.md\tdemo-type\tno\t\tx\t-");
        assert!(matches!(
            parse_schema_tsv(&tsv),
            Err(TemplateViolation::SchemaFieldCount { found: 6, .. })
        ));
    }

    #[test]
    fn parse_schema_tsv_flags_an_empty_table() {
        assert_eq!(
            parse_schema_tsv(&format!("{HEADER}\n")),
            Err(TemplateViolation::SchemaNoRows)
        );
    }

    #[test]
    fn parse_schema_tsv_reads_a_well_formed_row() {
        let tsv = format!(
            "{HEADER}\nwork-item.md\twork-item\tno\tkind priority\t\
             draft | ready\twork_item_id\tparent blocks"
        );
        let rows = parse_schema_tsv(&tsv).expect("well-formed");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].doc_type, "work-item");
        assert_eq!(rows[0].extras, vec!["kind", "priority"]);
        assert_eq!(rows[0].forbidden_own_id_keys, vec!["work_item_id"]);
        assert_eq!(rows[0].typed_linkage_keys, vec!["parent", "blocks"]);
    }

    #[test]
    fn a_dash_forbidden_own_id_column_reads_as_empty() {
        let tsv = format!("{HEADER}\ndemo.md\tdemo-type\tno\t\tx\t-\tparent");
        let rows = parse_schema_tsv(&tsv).expect("well-formed");
        assert!(rows[0].forbidden_own_id_keys.is_empty());
    }

    #[test]
    fn cross_check_passes_on_a_matching_set() {
        let reference = vec!["a.md".to_owned(), "b.md".to_owned()];
        let tsv = vec!["b.md".to_owned(), "a.md".to_owned()];
        assert!(cross_check(&reference, &tsv).is_empty());
    }

    #[test]
    fn cross_check_flags_a_divergent_set() {
        let reference = vec!["a.md".to_owned()];
        let tsv = vec!["b.md".to_owned()];
        assert!(!cross_check(&reference, &tsv).is_empty());
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
