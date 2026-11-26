use corpus::DocTypeKey;

struct Facts {
    kind: DocTypeKey,
    wire_token: &'static str,
    config_path_key: Option<&'static str>,
    linkage_type_name: Option<&'static str>,
    label: &'static str,
    predicates: Predicates,
    nested_root_filename: Option<&'static str>,
}

/// In order: in lifecycle, carries target frontmatter, participates in
/// lifecycle, in kanban, is virtual.
type Predicates = [bool; 5];

const T: bool = true;
const F: bool = false;

const BUILT_IN_IN_CATALOGUE_ORDER: [Facts; 15] = [
    Facts {
        kind: DocTypeKey::Decisions,
        wire_token: "decisions",
        config_path_key: Some("decisions"),
        linkage_type_name: Some("adr"),
        label: "Decisions",
        predicates: [T, F, F, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::WorkItems,
        wire_token: "work-items",
        config_path_key: Some("work"),
        linkage_type_name: Some("work-item"),
        label: "Work items",
        predicates: [T, F, T, T, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::Plans,
        wire_token: "plans",
        config_path_key: Some("plans"),
        linkage_type_name: Some("plan"),
        label: "Plans",
        predicates: [T, F, T, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::Research,
        wire_token: "codebase-research",
        config_path_key: Some("research_codebase"),
        linkage_type_name: Some("codebase-research"),
        label: "Codebase research",
        predicates: [T, F, T, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::PlanReviews,
        wire_token: "plan-reviews",
        config_path_key: Some("review_plans"),
        linkage_type_name: Some("plan-review"),
        label: "Plan reviews",
        predicates: [T, T, T, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::PrReviews,
        wire_token: "pr-reviews",
        config_path_key: Some("review_prs"),
        linkage_type_name: Some("pr-review"),
        label: "PR reviews",
        predicates: [T, T, T, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::WorkItemReviews,
        wire_token: "work-item-reviews",
        config_path_key: Some("review_work"),
        linkage_type_name: Some("work-item-review"),
        label: "Work item reviews",
        predicates: [T, T, T, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::Validations,
        wire_token: "validations",
        config_path_key: Some("validations"),
        linkage_type_name: Some("plan-validation"),
        label: "Validations",
        predicates: [T, T, T, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::Notes,
        wire_token: "notes",
        config_path_key: Some("notes"),
        linkage_type_name: Some("note"),
        label: "Notes",
        predicates: [T, F, F, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::PrDescriptions,
        wire_token: "pr-descriptions",
        config_path_key: Some("prs"),
        linkage_type_name: Some("pr-description"),
        label: "PR descriptions",
        predicates: [T, F, T, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::DesignGaps,
        wire_token: "design-gaps",
        config_path_key: Some("research_design_gaps"),
        linkage_type_name: Some("design-gap"),
        label: "Design gaps",
        predicates: [T, F, F, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::DesignInventories,
        wire_token: "design-inventories",
        config_path_key: Some("research_design_inventories"),
        linkage_type_name: Some("design-inventory"),
        label: "Design inventories",
        predicates: [T, F, F, F, F],
        nested_root_filename: Some("inventory.md"),
    },
    Facts {
        kind: DocTypeKey::TopicResearch,
        wire_token: "topic-research",
        config_path_key: Some("research_topics"),
        linkage_type_name: Some("topic-research"),
        label: "Topic research",
        predicates: [T, F, F, F, F],
        nested_root_filename: Some("manifest.md"),
    },
    Facts {
        kind: DocTypeKey::RootCauseAnalyses,
        wire_token: "root-cause-analyses",
        config_path_key: Some("research_issues"),
        linkage_type_name: Some("issue-research"),
        label: "Root cause analyses",
        predicates: [F, F, F, F, F],
        nested_root_filename: None,
    },
    Facts {
        kind: DocTypeKey::Templates,
        wire_token: "templates",
        config_path_key: None,
        linkage_type_name: None,
        label: "Templates",
        predicates: [F, F, F, F, T],
        nested_root_filename: None,
    },
];

#[test]
fn the_built_in_doc_types_are_declared_in_catalogue_order() {
    let declared: Vec<DocTypeKey> = DocTypeKey::all().to_vec();
    let expected: Vec<DocTypeKey> = BUILT_IN_IN_CATALOGUE_ORDER
        .iter()
        .map(|row| row.kind)
        .collect();
    assert_eq!(declared, expected);
}

#[test]
fn each_built_in_doc_type_declares_its_facts() {
    for row in &BUILT_IN_IN_CATALOGUE_ORDER {
        let kind = row.kind;
        let context = row.wire_token;
        assert_eq!(kind.wire_str(), row.wire_token, "{context}");
        assert_eq!(kind.config_path_key(), row.config_path_key, "{context}");
        assert_eq!(
            kind.linkage_type_name(),
            row.linkage_type_name,
            "{context}"
        );
        assert_eq!(kind.label(), row.label, "{context}");
        assert_eq!(
            [
                kind.in_lifecycle(),
                kind.carries_target_frontmatter(),
                kind.participates_in_lifecycle(),
                kind.in_kanban(),
                kind.is_virtual(),
            ],
            row.predicates,
            "{context}"
        );
        assert_eq!(
            kind.nested_manifest_filename(),
            row.nested_root_filename,
            "{context}"
        );
    }
}
