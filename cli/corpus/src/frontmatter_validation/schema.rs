//! The per-type schema table (mirrors `scripts/templates-schema.tsv`) and the
//! cross-cutting emission rules (mirrors
//! `scripts/frontmatter-emission-rules.sh`). Pure data — no filesystem, no
//! regex.

/// One `templates-schema.tsv` row, keyed by [`crate::DocTypeKey::linkage_type_name`].
pub struct SchemaRow {
    pub linkage_type: &'static str,
    pub code_state_anchored: bool,
    pub extras: &'static [&'static str],
    pub status_vocab: &'static [&'static str],
    pub forbidden_own_id_keys: &'static [&'static str],
    pub typed_linkage_keys: &'static [&'static str],
}

/// Mirrors `scripts/templates-schema.tsv` row for row. A `corpus` test
/// asserts the two agree.
pub const SCHEMA: [SchemaRow; 13] = [
    SchemaRow {
        linkage_type: "work-item",
        code_state_anchored: false,
        extras: &["kind", "priority", "external_id"],
        status_vocab: &[
            "draft",
            "ready",
            "in-progress",
            "review",
            "done",
            "blocked",
            "abandoned",
        ],
        forbidden_own_id_keys: &["work_item_id"],
        typed_linkage_keys: &[
            "parent",
            "blocks",
            "blocked_by",
            "derived_from",
            "relates_to",
            "source",
        ],
    },
    SchemaRow {
        linkage_type: "plan",
        code_state_anchored: true,
        extras: &["reviewer"],
        status_vocab: &["draft", "ready", "in-progress", "done"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &[
            "parent",
            "blocks",
            "blocked_by",
            "derived_from",
            "relates_to",
        ],
    },
    SchemaRow {
        linkage_type: "plan-validation",
        code_state_anchored: false,
        extras: &["result"],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        linkage_type: "pr-description",
        code_state_anchored: true,
        extras: &["pr_url", "pr_number", "merge_commit"],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &["pr_title"],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        linkage_type: "adr",
        code_state_anchored: false,
        extras: &["decision_makers"],
        status_vocab: &[
            "proposed",
            "accepted",
            "rejected",
            "superseded",
            "deprecated",
        ],
        forbidden_own_id_keys: &["adr_id"],
        typed_linkage_keys: &["parent", "supersedes", "relates_to"],
    },
    SchemaRow {
        linkage_type: "codebase-research",
        code_state_anchored: true,
        extras: &["topic"],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        linkage_type: "issue-research",
        code_state_anchored: true,
        extras: &["topic"],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        linkage_type: "design-inventory",
        code_state_anchored: true,
        extras: &[
            "source",
            "source_kind",
            "source_location",
            "crawler",
            "sequence",
            "screenshots_incomplete",
        ],
        status_vocab: &["draft", "superseded"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        linkage_type: "design-gap",
        code_state_anchored: false,
        extras: &["current_inventory", "target_inventory"],
        status_vocab: &["draft", "accepted"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        linkage_type: "plan-review",
        code_state_anchored: false,
        extras: &[
            "reviewer",
            "verdict",
            "lenses",
            "review_number",
            "review_pass",
        ],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        linkage_type: "work-item-review",
        code_state_anchored: false,
        extras: &[
            "reviewer",
            "verdict",
            "lenses",
            "review_number",
            "review_pass",
        ],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        linkage_type: "pr-review",
        code_state_anchored: false,
        extras: &[
            "reviewer",
            "verdict",
            "lenses",
            "review_number",
            "pr_number",
        ],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &["pr_title", "review_pass"],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        linkage_type: "note",
        code_state_anchored: true,
        extras: &["topic"],
        status_vocab: &["captured"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
];

/// The row for a resolved type, or `None` when the type is unknown.
#[must_use]
pub fn row_for(linkage_type: &str) -> Option<&'static SchemaRow> {
    SCHEMA.iter().find(|row| row.linkage_type == linkage_type)
}

/// The base fields every conforming artifact MUST carry.
///
/// Mirrors `FM_BASE_FIELDS` — deliberately excludes `producer` and `status`
/// (hand-written legacy plans may omit `producer`; a status-less source
/// artifact leaves `status` unset).
pub const BASE_FIELDS: [&str; 9] = [
    "type",
    "id",
    "title",
    "date",
    "author",
    "tags",
    "last_updated",
    "last_updated_by",
    "schema_version",
];

/// Mirrors `FM_PROVENANCE_FIELDS`.
pub const PROVENANCE_FIELDS: [&str; 2] = ["revision", "repository"];

/// Mirrors `FM_FORBIDDEN_PROVENANCE_FIELDS`.
pub const FORBIDDEN_PROVENANCE_FIELDS: [&str; 2] = ["git_commit", "branch"];

/// Mirrors `FM_SOURCE_TYPE_RE`'s vocabulary — the source types a typed-linkage
/// reference's `<type>:` prefix may name.
pub const LINKAGE_SOURCE_TYPES: [&str; 14] = [
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

/// Mirrors `FM_OPTIONAL_EXTRAS`.
///
/// Per-type `extras` that are legitimately omitted when empty, so a
/// present-but-empty value is not additionally required. `work_item_id` is
/// the transitional foreign-ref alias.
pub const OPTIONAL_EXTRAS: [&str; 6] = [
    "external_id",
    "reviewer",
    "pr_url",
    "merge_commit",
    "decision_makers",
    "work_item_id",
];

/// Mirrors `OBSOLETE_LEGACY_KEYS` — fully-obsolete legacy linkage keys,
/// forbidden on every typed/type-inferable document.
pub const OBSOLETE_LEGACY_KEYS: [&str; 2] = ["ticket", "ticket_id"];

#[cfg(test)]
mod tests {
    use super::{row_for, SCHEMA};

    #[test]
    fn every_row_resolves_by_its_own_linkage_type() {
        for row in &SCHEMA {
            assert!(
                row_for(row.linkage_type).is_some(),
                "row {} does not resolve",
                row.linkage_type
            );
        }
    }

    #[test]
    fn an_unknown_type_resolves_to_none() {
        assert!(row_for("not-a-type").is_none());
    }

    #[test]
    fn thirteen_rows_are_present() {
        assert_eq!(SCHEMA.len(), 13);
    }

    #[test]
    fn pr_review_carries_two_forbidden_own_id_keys(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let row = row_for("pr-review").ok_or("pr-review row")?;
        assert_eq!(row.forbidden_own_id_keys, &["pr_title", "review_pass"]);
        Ok(())
    }

    const TEMPLATES_SCHEMA_TSV: &str =
        include_str!("../../../../scripts/templates-schema.tsv");

    #[test]
    fn every_row_matches_templates_schema_tsv(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut lines = TEMPLATES_SCHEMA_TSV.lines();
        lines.next().ok_or("missing header")?;

        let mut seen = 0;
        for line in lines {
            let columns: Vec<&str> = line.split('\t').collect();
            let [_template, type_name, anchored, extras, status_vocab, forbidden, linkkeys] =
                columns.as_slice()
            else {
                return Err(format!("unexpected column count: {line}").into());
            };
            let row = row_for(type_name)
                .ok_or_else(|| format!("no SCHEMA row for {type_name}"))?;

            assert_eq!(
                row.code_state_anchored,
                *anchored == "yes",
                "{type_name}"
            );
            assert_eq!(
                row.extras.join(" "),
                if *extras == "-" { "" } else { extras },
                "{type_name}"
            );
            assert_eq!(
                row.status_vocab.join(" | "),
                *status_vocab,
                "{type_name}"
            );
            assert_eq!(
                row.forbidden_own_id_keys.join(" "),
                if *forbidden == "-" { "" } else { forbidden },
                "{type_name}"
            );
            assert_eq!(
                row.typed_linkage_keys.join(" "),
                *linkkeys,
                "{type_name}"
            );
            seen += 1;
        }
        assert_eq!(seen, SCHEMA.len());
        Ok(())
    }
}
