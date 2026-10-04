//! The per-type schema table and the cross-cutting emission rules. Pure
//! data — no filesystem, no regex.

/// One schema row, keyed by the `(type, kind)` pair
/// [`crate::DocTypeKey::linkage_type_name`] plus the artifact `kind`
/// discriminator (empty for a type-level default row).
///
/// `template` names the `templates/` skeleton the row's documents are
/// emitted from.
pub struct SchemaRow {
    pub template: &'static str,
    pub linkage_type: &'static str,
    pub kind: &'static str,
    pub code_state_anchored: bool,
    pub required_extras: &'static [&'static str],
    /// Per-type keys legitimately omitted when empty, so a present-but-empty
    /// value is not additionally required.
    pub optional_extras: &'static [&'static str],
    /// Extras whose empty list is a value in its own right rather than an
    /// unfilled placeholder.
    pub empty_list_extras: &'static [&'static str],
    pub status_vocab: &'static [&'static str],
    pub forbidden_own_id_keys: &'static [&'static str],
    pub typed_linkage_keys: &'static [&'static str],
}

pub const SCHEMA: [SchemaRow; 19] = [
    SchemaRow {
        template: "work-item.md",
        linkage_type: "work-item",
        kind: "",
        code_state_anchored: false,
        required_extras: &["kind", "priority"],
        optional_extras: &["external_id"],
        empty_list_extras: &[],
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
        template: "plan.md",
        linkage_type: "plan",
        kind: "",
        code_state_anchored: true,
        required_extras: &[],
        optional_extras: &["reviewer"],
        empty_list_extras: &[],
        status_vocab: &["draft", "ready", "in-progress", "done", "superseded"],
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
        template: "validation.md",
        linkage_type: "plan-validation",
        kind: "",
        code_state_anchored: false,
        required_extras: &["result"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        template: "pr-description.md",
        linkage_type: "pr-description",
        kind: "",
        code_state_anchored: true,
        required_extras: &["pr_number"],
        optional_extras: &["pr_url", "merge_commit"],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &["pr_title"],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "adr.md",
        linkage_type: "adr",
        kind: "",
        code_state_anchored: false,
        required_extras: &[],
        optional_extras: &["decision_makers"],
        empty_list_extras: &[],
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
        template: "codebase-research.md",
        linkage_type: "codebase-research",
        kind: "",
        code_state_anchored: true,
        required_extras: &["topic"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "rca.md",
        linkage_type: "issue-research",
        kind: "",
        code_state_anchored: true,
        required_extras: &["topic"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "design-inventory.md",
        linkage_type: "design-inventory",
        kind: "",
        code_state_anchored: true,
        required_extras: &[
            "source",
            "source_kind",
            "source_location",
            "crawler",
            "sequence",
            "screenshots_incomplete",
        ],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["draft", "superseded"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "design-gap.md",
        linkage_type: "design-gap",
        kind: "",
        code_state_anchored: false,
        required_extras: &["current_inventory", "target_inventory"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["draft", "accepted"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "plan-review.md",
        linkage_type: "plan-review",
        kind: "",
        code_state_anchored: false,
        required_extras: &["verdict", "lenses", "review_number", "review_pass"],
        optional_extras: &["reviewer"],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        template: "work-item-review.md",
        linkage_type: "work-item-review",
        kind: "",
        code_state_anchored: false,
        required_extras: &["verdict", "lenses", "review_number", "review_pass"],
        optional_extras: &["reviewer"],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        template: "pr-review.md",
        linkage_type: "pr-review",
        kind: "",
        code_state_anchored: false,
        required_extras: &["verdict", "lenses", "review_number", "pr_number"],
        optional_extras: &["reviewer"],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &["pr_title", "review_pass"],
        typed_linkage_keys: &["parent", "target", "relates_to"],
    },
    SchemaRow {
        template: "note.md",
        linkage_type: "note",
        kind: "",
        code_state_anchored: true,
        required_extras: &["topic"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["captured"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "topic-research-manifest.md",
        linkage_type: "topic-research",
        kind: "manifest",
        code_state_anchored: false,
        required_extras: &["slug", "round_count", "finding_count", "primary"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &[
            "briefed",
            "outlined",
            "researching",
            "synthesised",
            "complete",
        ],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "topic-research-brief.md",
        linkage_type: "topic-research",
        kind: "brief",
        code_state_anchored: false,
        required_extras: &["source_profiles"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["draft", "complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "topic-research-outline.md",
        linkage_type: "topic-research",
        kind: "outline",
        code_state_anchored: false,
        required_extras: &[],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "topic-research-finding.md",
        linkage_type: "topic-research",
        kind: "finding",
        code_state_anchored: false,
        required_extras: &["round", "question", "source_profile"],
        optional_extras: &["depth"],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "topic-research-synthesis.md",
        linkage_type: "topic-research",
        kind: "synthesis",
        code_state_anchored: false,
        required_extras: &["rounds_covered"],
        optional_extras: &[],
        empty_list_extras: &[],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
    SchemaRow {
        template: "topic-research-level-note.md",
        linkage_type: "topic-research",
        kind: "level-note",
        code_state_anchored: false,
        required_extras: &[
            "round",
            "question",
            "source_profile",
            "level",
            "depth",
            "follow_ups",
        ],
        optional_extras: &[],
        empty_list_extras: &["follow_ups"],
        status_vocab: &["complete"],
        forbidden_own_id_keys: &[],
        typed_linkage_keys: &["parent", "relates_to"],
    },
];

impl SchemaRow {
    /// Every per-type key the row names, required first.
    pub fn all_extras(&self) -> impl Iterator<Item = &'static str> {
        self.required_extras
            .iter()
            .chain(self.optional_extras)
            .copied()
    }
}

/// The row for a resolved `(type, kind)` pair, preferring an exact
/// kind-specific row and falling back to the type-level default row
/// `(type, "")`. `None` when no row shares the `linkage_type`.
#[must_use]
pub fn row_for(linkage_type: &str, kind: &str) -> Option<&'static SchemaRow> {
    SCHEMA
        .iter()
        .find(|row| row.linkage_type == linkage_type && row.kind == kind)
        .or_else(|| {
            SCHEMA.iter().find(|row| {
                row.linkage_type == linkage_type && row.kind.is_empty()
            })
        })
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
pub const LINKAGE_SOURCE_TYPES: [&str; 15] = [
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

/// Fully-obsolete legacy linkage keys, forbidden on every
/// typed/type-inferable document.
pub const OBSOLETE_LEGACY_KEYS: [&str; 3] =
    ["ticket", "ticket_id", "research_status"];

/// `name` as the schema's own `'static` key, when any schema row, base field
/// or reserved key names it.
#[must_use]
pub fn named_key(name: &str) -> Option<&'static str> {
    let row_keys = SCHEMA.iter().flat_map(|row| {
        row.required_extras
            .iter()
            .chain(row.optional_extras)
            .chain(row.forbidden_own_id_keys)
            .chain(row.typed_linkage_keys)
    });
    BASE_FIELDS
        .iter()
        .chain(&PROVENANCE_FIELDS)
        .chain(&FORBIDDEN_PROVENANCE_FIELDS)
        .chain(&OBSOLETE_LEGACY_KEYS)
        .chain(&["kind", "status", "producer"])
        .chain(row_keys)
        .copied()
        .find(|key| *key == name)
}

#[cfg(test)]
mod tests {
    use super::{row_for, SCHEMA};

    /// The committed `topic-research-status-vocab.json` fixture is the
    /// cross-language contract the frontend chip ramp reads. Regenerate it from
    /// this test's rendering when the manifest `status_vocab` changes, so the
    /// TS `lifecycle-*` map cannot silently drift from the Rust vocabulary.
    #[test]
    fn topic_research_status_vocab_fixture_matches_the_schema_row(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let vocab = row_for("topic-research", "manifest")
            .ok_or("topic-research manifest row")?
            .status_vocab;
        let rendered = format!(
            "[{}]",
            vocab
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let fixture = include_str!(
            "../../tests/fixtures/topic-research-status-vocab.json"
        );
        assert_eq!(
            rendered,
            fixture.trim(),
            "the committed status-vocab fixture has drifted from schema.rs; \
             regenerate cli/corpus/tests/fixtures/topic-research-status-vocab.json"
        );
        Ok(())
    }

    #[test]
    fn every_row_resolves_by_its_own_linkage_type_and_kind() {
        for row in &SCHEMA {
            assert!(
                row_for(row.linkage_type, row.kind).is_some(),
                "row {} does not resolve",
                row.linkage_type
            );
        }
    }

    #[test]
    fn an_unknown_type_resolves_to_none() {
        assert!(row_for("not-a-type", "").is_none());
    }

    #[test]
    fn a_kind_on_a_type_lacking_that_kind_falls_back_to_the_type_default() {
        let fallback = row_for("work-item", "no-such-kind");
        assert!(fallback.is_some_and(
            |row| row.linkage_type == "work-item" && row.kind.is_empty()
        ));
    }

    #[test]
    fn a_kind_on_a_type_with_no_default_row_does_not_fall_back() {
        // A hypothetical multi-kind type carrying no `(type, "")` row must not
        // resolve an unmatched kind — guarding the UnknownKind split.
        assert!(row_for("work-item", "").is_some());
        assert!(row_for("not-a-type", "anything").is_none());
    }

    #[test]
    fn every_empty_list_extra_is_an_extra_of_its_row() {
        for row in &SCHEMA {
            for extra in row.empty_list_extras {
                assert!(
                    row.required_extras.contains(extra)
                        || row.optional_extras.contains(extra),
                    "{}/{}: {extra}",
                    row.linkage_type,
                    row.kind
                );
            }
        }
    }

    #[test]
    fn only_a_level_note_may_record_an_empty_list() {
        let permitting: Vec<(&str, &[&str])> = SCHEMA
            .iter()
            .filter(|row| !row.empty_list_extras.is_empty())
            .map(|row| (row.kind, row.empty_list_extras))
            .collect();
        assert_eq!(permitting, [("level-note", &["follow_ups"][..])]);
    }

    #[test]
    fn nineteen_rows_are_present() {
        assert_eq!(SCHEMA.len(), 19);
    }

    #[test]
    fn topic_research_kinds_each_resolve_to_a_distinct_row() {
        for kind in [
            "manifest",
            "brief",
            "outline",
            "finding",
            "synthesis",
            "level-note",
        ] {
            let resolved = row_for("topic-research", kind).map(|row| row.kind);
            assert_eq!(resolved, Some(kind), "topic-research/{kind}");
        }
        // No (topic-research, "") default row exists, so an unmatched kind
        // must not silently fall back.
        assert!(row_for("topic-research", "").is_none());
        assert!(row_for("topic-research", "bogus").is_none());
    }

    #[test]
    fn no_kind_discriminated_row_shadows_a_same_named_extra() {
        // Guards the `kind` overload: introducing a (type, kind) row whose
        // `kind` also appears in that type's own `extras` silently re-routes
        // every existing `kind: <that>` document from the type default row to
        // the new one. Such a row is a migration, not an addition — this fails
        // until it is made explicit here.
        for row in &SCHEMA {
            if row.kind.is_empty() {
                continue;
            }
            let default = row_for(row.linkage_type, "");
            let clashes = default.is_some_and(|type_default| {
                type_default.all_extras().any(|extra| extra == row.kind)
            });
            assert!(
                !clashes,
                "(type={}, kind={}) shadows a same-named extra on the type \
                 default row — treat it as an explicit migration",
                row.linkage_type, row.kind
            );
        }
    }

    #[test]
    fn pr_review_carries_two_forbidden_own_id_keys(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let row = row_for("pr-review", "").ok_or("pr-review row")?;
        assert_eq!(row.forbidden_own_id_keys, &["pr_title", "review_pass"]);
        Ok(())
    }

    #[test]
    fn every_row_names_a_distinct_template() {
        let mut templates: Vec<&str> =
            SCHEMA.iter().map(|row| row.template).collect();
        assert!(templates.iter().all(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "md")
        }));
        templates.sort_unstable();
        templates.dedup();
        assert_eq!(templates.len(), SCHEMA.len());
    }

    #[test]
    fn a_finding_row_holds_depth_as_optional_and_a_level_note_row_requires_it(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let finding =
            row_for("topic-research", "finding").ok_or("finding row")?;
        assert!(finding.optional_extras.contains(&"depth"));
        assert!(!finding.required_extras.contains(&"depth"));
        let note =
            row_for("topic-research", "level-note").ok_or("level-note row")?;
        assert!(note.required_extras.contains(&"depth"));
        Ok(())
    }

    #[test]
    fn every_formerly_global_optional_extra_is_optional_on_exactly_its_rows() {
        let expected = [
            ("decision_makers", "adr"),
            ("external_id", "work-item"),
            ("merge_commit", "pr-description"),
            ("pr_url", "pr-description"),
            ("reviewer", "plan"),
            ("reviewer", "plan-review"),
            ("reviewer", "pr-review"),
            ("reviewer", "work-item-review"),
        ];
        let formerly_global: Vec<&str> =
            expected.iter().map(|(extra, _)| *extra).collect();
        let mut optional: Vec<(&str, &str)> = SCHEMA
            .iter()
            .flat_map(|row| {
                row.optional_extras
                    .iter()
                    .map(move |extra| (*extra, row.linkage_type))
            })
            .filter(|(extra, _)| formerly_global.contains(extra))
            .collect();
        optional.sort_unstable();
        assert_eq!(optional, expected);
        for row in &SCHEMA {
            for extra in row.optional_extras {
                assert!(
                    !row.required_extras.contains(extra),
                    "{} requires its optional {extra}",
                    row.linkage_type
                );
            }
        }
    }
}
