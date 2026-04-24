use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocTypeKey {
    Decisions,
    Tickets,
    Plans,
    Research,
    PlanReviews,
    PrReviews,
    Validations,
    Notes,
    Prs,
    Templates,
}

impl DocTypeKey {
    pub fn all() -> [DocTypeKey; 10] {
        [
            DocTypeKey::Decisions,
            DocTypeKey::Tickets,
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::PlanReviews,
            DocTypeKey::PrReviews,
            DocTypeKey::Validations,
            DocTypeKey::Notes,
            DocTypeKey::Prs,
            DocTypeKey::Templates,
        ]
    }

    pub fn config_path_key(self) -> Option<&'static str> {
        match self {
            DocTypeKey::Decisions => Some("decisions"),
            DocTypeKey::Tickets => Some("tickets"),
            DocTypeKey::Plans => Some("plans"),
            DocTypeKey::Research => Some("research"),
            DocTypeKey::PlanReviews => Some("review_plans"),
            DocTypeKey::PrReviews => Some("review_prs"),
            DocTypeKey::Validations => Some("validations"),
            DocTypeKey::Notes => Some("notes"),
            DocTypeKey::Prs => Some("prs"),
            DocTypeKey::Templates => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DocTypeKey::Decisions => "Decisions",
            DocTypeKey::Tickets => "Tickets",
            DocTypeKey::Plans => "Plans",
            DocTypeKey::Research => "Research",
            DocTypeKey::PlanReviews => "Plan reviews",
            DocTypeKey::PrReviews => "PR reviews",
            DocTypeKey::Validations => "Validations",
            DocTypeKey::Notes => "Notes",
            DocTypeKey::Prs => "PRs",
            DocTypeKey::Templates => "Templates",
        }
    }

    pub fn in_lifecycle(self) -> bool {
        !matches!(self, DocTypeKey::Templates)
    }

    pub fn in_kanban(self) -> bool {
        matches!(self, DocTypeKey::Tickets)
    }

    pub fn is_virtual(self) -> bool {
        matches!(self, DocTypeKey::Templates)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocType {
    pub key: DocTypeKey,
    pub label: String,
    pub dir_path: Option<PathBuf>,
    pub in_lifecycle: bool,
    pub in_kanban: bool,
    pub r#virtual: bool,
}

pub fn describe_types(cfg: &crate::config::Config) -> Vec<DocType> {
    let mut out = Vec::with_capacity(DocTypeKey::all().len());
    for key in DocTypeKey::all() {
        let dir_path = key
            .config_path_key()
            .and_then(|k| cfg.doc_paths.get(k).cloned());
        out.push(DocType {
            key,
            label: key.label().to_string(),
            dir_path,
            in_lifecycle: key.in_lifecycle(),
            in_kanban: key.in_kanban(),
            r#virtual: key.is_virtual(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_case_round_trip_covers_every_variant() {
        let pairs = [
            (DocTypeKey::Decisions, "decisions"),
            (DocTypeKey::Tickets, "tickets"),
            (DocTypeKey::Plans, "plans"),
            (DocTypeKey::Research, "research"),
            (DocTypeKey::PlanReviews, "plan-reviews"),
            (DocTypeKey::PrReviews, "pr-reviews"),
            (DocTypeKey::Validations, "validations"),
            (DocTypeKey::Notes, "notes"),
            (DocTypeKey::Prs, "prs"),
            (DocTypeKey::Templates, "templates"),
        ];
        for (variant, wire) in pairs {
            let ser = serde_json::to_string(&variant).unwrap();
            assert_eq!(ser, format!("\"{wire}\""));
            let de: DocTypeKey = serde_json::from_str(&ser).unwrap();
            assert_eq!(de, variant);
        }
    }

    #[test]
    fn all_returns_every_variant_exactly_once() {
        let mut v = DocTypeKey::all().to_vec();
        v.sort_by_key(|k| k.label().to_string());
        v.dedup();
        assert_eq!(v.len(), 10, "DocTypeKey::all must return 10 distinct variants");
    }

    #[test]
    fn templates_is_virtual_and_out_of_lifecycle() {
        assert!(DocTypeKey::Templates.is_virtual());
        assert!(!DocTypeKey::Templates.in_lifecycle());
        assert!(!DocTypeKey::Templates.in_kanban());
    }

    #[test]
    fn tickets_is_the_only_kanban_type() {
        for k in DocTypeKey::all() {
            assert_eq!(
                k.in_kanban(),
                matches!(k, DocTypeKey::Tickets),
                "in_kanban mismatch for {k:?}",
            );
        }
    }

    #[test]
    fn describe_types_populates_dir_paths_from_config() {
        let mut doc_paths = std::collections::HashMap::new();
        doc_paths.insert("decisions".into(), PathBuf::from("/abs/decisions"));
        doc_paths.insert("review_plans".into(), PathBuf::from("/abs/reviews/plans"));

        let cfg = crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths,
            templates: Default::default(),
        };

        let types = describe_types(&cfg);
        assert_eq!(types.len(), 10);
        let decisions = types.iter().find(|t| t.key == DocTypeKey::Decisions).unwrap();
        assert_eq!(decisions.dir_path.as_deref(), Some(std::path::Path::new("/abs/decisions")));
        let plan_reviews = types.iter().find(|t| t.key == DocTypeKey::PlanReviews).unwrap();
        assert_eq!(plan_reviews.dir_path.as_deref(), Some(std::path::Path::new("/abs/reviews/plans")));
        let templates = types.iter().find(|t| t.key == DocTypeKey::Templates).unwrap();
        assert!(templates.dir_path.is_none());
        assert!(templates.r#virtual);
    }

    #[test]
    fn virtual_flag_always_serialised_in_json() {
        let cfg = crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths: Default::default(),
            templates: Default::default(),
        };
        let types = describe_types(&cfg);
        let decisions = types.iter().find(|t| t.key == DocTypeKey::Decisions).unwrap();
        let json = serde_json::to_value(decisions).unwrap();
        assert_eq!(json.get("virtual"), Some(&serde_json::Value::Bool(false)),
            "virtual must always be emitted, even when false");
        let templates = types.iter().find(|t| t.key == DocTypeKey::Templates).unwrap();
        let json = serde_json::to_value(templates).unwrap();
        assert_eq!(json.get("virtual"), Some(&serde_json::Value::Bool(true)));
    }
}
