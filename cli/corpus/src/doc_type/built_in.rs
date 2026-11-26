//! The doc types the corpus declares itself, in catalogue order.

use crate::doc_type::DocType;
use crate::doc_type::DocTypeKey;

static DECISIONS: DocType = DocType {
    wire_token: "decisions",
    config_path_key: Some("decisions"),
    linkage_type_name: Some("adr"),
    label: "Decisions",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: false,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static WORK_ITEMS: DocType = DocType {
    wire_token: "work-items",
    config_path_key: Some("work"),
    linkage_type_name: Some("work-item"),
    label: "Work items",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: true,
    in_kanban: true,
    is_virtual: false,
    nested_root_filename: None,
};

static PLANS: DocType = DocType {
    wire_token: "plans",
    config_path_key: Some("plans"),
    linkage_type_name: Some("plan"),
    label: "Plans",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: true,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static RESEARCH: DocType = DocType {
    wire_token: "codebase-research",
    config_path_key: Some("research_codebase"),
    linkage_type_name: Some("codebase-research"),
    label: "Codebase research",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: true,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static PLAN_REVIEWS: DocType = DocType {
    wire_token: "plan-reviews",
    config_path_key: Some("review_plans"),
    linkage_type_name: Some("plan-review"),
    label: "Plan reviews",
    in_lifecycle: true,
    carries_target_frontmatter: true,
    participates_in_lifecycle: true,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static PR_REVIEWS: DocType = DocType {
    wire_token: "pr-reviews",
    config_path_key: Some("review_prs"),
    linkage_type_name: Some("pr-review"),
    label: "PR reviews",
    in_lifecycle: true,
    carries_target_frontmatter: true,
    participates_in_lifecycle: true,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static WORK_ITEM_REVIEWS: DocType = DocType {
    wire_token: "work-item-reviews",
    config_path_key: Some("review_work"),
    linkage_type_name: Some("work-item-review"),
    label: "Work item reviews",
    in_lifecycle: true,
    carries_target_frontmatter: true,
    participates_in_lifecycle: true,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static VALIDATIONS: DocType = DocType {
    wire_token: "validations",
    config_path_key: Some("validations"),
    linkage_type_name: Some("plan-validation"),
    label: "Validations",
    in_lifecycle: true,
    carries_target_frontmatter: true,
    participates_in_lifecycle: true,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static NOTES: DocType = DocType {
    wire_token: "notes",
    config_path_key: Some("notes"),
    linkage_type_name: Some("note"),
    label: "Notes",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: false,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static PR_DESCRIPTIONS: DocType = DocType {
    wire_token: "pr-descriptions",
    config_path_key: Some("prs"),
    linkage_type_name: Some("pr-description"),
    label: "PR descriptions",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: true,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static DESIGN_GAPS: DocType = DocType {
    wire_token: "design-gaps",
    config_path_key: Some("research_design_gaps"),
    linkage_type_name: Some("design-gap"),
    label: "Design gaps",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: false,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static DESIGN_INVENTORIES: DocType = DocType {
    wire_token: "design-inventories",
    config_path_key: Some("research_design_inventories"),
    linkage_type_name: Some("design-inventory"),
    label: "Design inventories",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: false,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: Some("inventory.md"),
};

static TOPIC_RESEARCH: DocType = DocType {
    wire_token: "topic-research",
    config_path_key: Some("research_topics"),
    linkage_type_name: Some("topic-research"),
    label: "Topic research",
    in_lifecycle: true,
    carries_target_frontmatter: false,
    participates_in_lifecycle: false,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: Some("manifest.md"),
};

static ROOT_CAUSE_ANALYSES: DocType = DocType {
    wire_token: "root-cause-analyses",
    config_path_key: Some("research_issues"),
    linkage_type_name: Some("issue-research"),
    label: "Root cause analyses",
    in_lifecycle: false,
    carries_target_frontmatter: false,
    participates_in_lifecycle: false,
    in_kanban: false,
    is_virtual: false,
    nested_root_filename: None,
};

static TEMPLATES: DocType = DocType {
    wire_token: "templates",
    config_path_key: None,
    linkage_type_name: None,
    label: "Templates",
    in_lifecycle: false,
    carries_target_frontmatter: false,
    participates_in_lifecycle: false,
    in_kanban: false,
    is_virtual: true,
    nested_root_filename: None,
};

impl DocTypeKey {
    pub const DECISIONS: Self = Self::declared(&DECISIONS);
    pub const WORK_ITEMS: Self = Self::declared(&WORK_ITEMS);
    pub const PLANS: Self = Self::declared(&PLANS);
    pub const RESEARCH: Self = Self::declared(&RESEARCH);
    pub const PLAN_REVIEWS: Self = Self::declared(&PLAN_REVIEWS);
    pub const PR_REVIEWS: Self = Self::declared(&PR_REVIEWS);
    pub const WORK_ITEM_REVIEWS: Self = Self::declared(&WORK_ITEM_REVIEWS);
    pub const VALIDATIONS: Self = Self::declared(&VALIDATIONS);
    pub const NOTES: Self = Self::declared(&NOTES);
    pub const PR_DESCRIPTIONS: Self = Self::declared(&PR_DESCRIPTIONS);
    pub const DESIGN_GAPS: Self = Self::declared(&DESIGN_GAPS);
    pub const DESIGN_INVENTORIES: Self = Self::declared(&DESIGN_INVENTORIES);
    pub const TOPIC_RESEARCH: Self = Self::declared(&TOPIC_RESEARCH);
    pub const ROOT_CAUSE_ANALYSES: Self = Self::declared(&ROOT_CAUSE_ANALYSES);
    pub const TEMPLATES: Self = Self::declared(&TEMPLATES);
}

pub(super) const IN_CATALOGUE_ORDER: [DocTypeKey; 15] = [
    DocTypeKey::DECISIONS,
    DocTypeKey::WORK_ITEMS,
    DocTypeKey::PLANS,
    DocTypeKey::RESEARCH,
    DocTypeKey::PLAN_REVIEWS,
    DocTypeKey::PR_REVIEWS,
    DocTypeKey::WORK_ITEM_REVIEWS,
    DocTypeKey::VALIDATIONS,
    DocTypeKey::NOTES,
    DocTypeKey::PR_DESCRIPTIONS,
    DocTypeKey::DESIGN_GAPS,
    DocTypeKey::DESIGN_INVENTORIES,
    DocTypeKey::TOPIC_RESEARCH,
    DocTypeKey::ROOT_CAUSE_ANALYSES,
    DocTypeKey::TEMPLATES,
];
