//! The doc types the corpus declares itself, in catalogue order.

use crate::cluster::ClusterKeyRule;
use crate::cluster::CompletenessMark;
use crate::doc_type::DocType;
use crate::doc_type::DocTypeKey;
use crate::doc_type::Facet;
use crate::doc_type::FilenameFilter;
use crate::doc_type::LibraryPhase;
use crate::doc_type::LibraryPlacement;
use crate::doc_type::Listing;
use crate::linkage::LinkageIdRule;
use crate::slug::SlugRule;

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
    slug_rule: SlugRule::NumberedPrefix("ADR-"),
    cluster_key_rule: ClusterKeyRule::Unclustered,
    lifecycle_rank: Some(7),
    completeness_mark: Some(CompletenessMark::Decision),
    linkage_id_rule: LinkageIdRule::AdrNumber,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Remember,
        position: 0,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::WorkItem,
    cluster_key_rule: ClusterKeyRule::WorkItemId,
    lifecycle_rank: Some(0),
    completeness_mark: Some(CompletenessMark::WorkItem),
    linkage_id_rule: LinkageIdRule::WorkItemIdInFilename,
    cluster_anchor: true,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Define,
        position: 0,
    }),
    facets: &[Facet::Status, Facet::Project, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: Some("drafts"),
        filename_filter: FilenameFilter::WorkItem,
    },
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Parent,
    lifecycle_rank: Some(2),
    completeness_mark: Some(CompletenessMark::Plan),
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Build,
        position: 0,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Parent,
    lifecycle_rank: Some(1),
    completeness_mark: Some(CompletenessMark::Research),
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Discover,
        position: 3,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalIdThenReviewSuffix,
    cluster_key_rule: ClusterKeyRule::Target,
    lifecycle_rank: Some(3),
    completeness_mark: Some(CompletenessMark::PlanReview),
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Build,
        position: 1,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalIdThenReviewSuffix,
    cluster_key_rule: ClusterKeyRule::Target,
    lifecycle_rank: Some(6),
    completeness_mark: Some(CompletenessMark::PrReview),
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Ship,
        position: 1,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::WorkItemReview,
    cluster_key_rule: ClusterKeyRule::Target,
    lifecycle_rank: Some(6),
    completeness_mark: None,
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Define,
        position: 1,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Target,
    lifecycle_rank: Some(4),
    completeness_mark: Some(CompletenessMark::Validation),
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Build,
        position: 2,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Unclustered,
    lifecycle_rank: Some(8),
    completeness_mark: Some(CompletenessMark::Notes),
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Remember,
        position: 1,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Parent,
    lifecycle_rank: Some(5),
    completeness_mark: Some(CompletenessMark::PrDescription),
    linkage_id_rule: LinkageIdRule::PrNumber,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Ship,
        position: 0,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Unclustered,
    lifecycle_rank: Some(10),
    completeness_mark: Some(CompletenessMark::DesignGap),
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Discover,
        position: 1,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Unclustered,
    lifecycle_rank: Some(9),
    completeness_mark: Some(CompletenessMark::DesignInventory),
    linkage_id_rule: LinkageIdRule::SetDirectory,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Discover,
        position: 0,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Set,
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
    slug_rule: SlugRule::VerbatimStem,
    cluster_key_rule: ClusterKeyRule::Unclustered,
    lifecycle_rank: None,
    completeness_mark: None,
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Discover,
        position: 2,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Set,
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
    slug_rule: SlugRule::DateAndOptionalId,
    cluster_key_rule: ClusterKeyRule::Unclustered,
    lifecycle_rank: None,
    completeness_mark: None,
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Operate,
        position: 0,
    }),
    facets: &[Facet::Status, Facet::ClusterSlug],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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
    slug_rule: SlugRule::NoSlug,
    cluster_key_rule: ClusterKeyRule::Unclustered,
    lifecycle_rank: None,
    completeness_mark: None,
    linkage_id_rule: LinkageIdRule::Stem,
    cluster_anchor: false,
    library_placement: None,
    facets: &[],
    listing: Listing::Flat {
        drafts_subdirectory: None,
        filename_filter: FilenameFilter::Markdown,
    },
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

static IN_CATALOGUE_ORDER: [DocTypeKey; 15] = [
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

/// The doc types the corpus declares, in catalogue order, for the
/// declarations assembly and tests to build a catalogue from.
#[must_use]
pub fn doc_types() -> &'static [DocTypeKey] {
    &IN_CATALOGUE_ORDER
}
