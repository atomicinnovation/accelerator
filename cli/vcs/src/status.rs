//! The backend-neutral status model and its pure renderer.
//!
//! Both the git and jj adapters populate a [`StatusReport`]; this module owns
//! the sort, the summary line, and the empty/conflict wording, so neither
//! adapter re-implements the agnostic format.

/// The closed set of change types the agnostic status format renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Untracked,
    Conflicted,
}

impl ChangeType {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Untracked => "untracked",
            Self::Conflicted => "conflicted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub change_type: ChangeType,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusReport {
    pub branch: Vec<String>,
    pub changes: Vec<FileChange>,
}

#[must_use]
pub fn render(report: &StatusReport) -> String {
    let branch = if report.branch.is_empty() {
        "(none)".to_owned()
    } else {
        report.branch.join(", ")
    };
    if report.changes.is_empty() {
        return format!("Branch: {branch}\nNo changes");
    }

    let conflicted = report
        .changes
        .iter()
        .filter(|change| change.change_type == ChangeType::Conflicted)
        .count();
    let summary = if conflicted > 0 {
        format!("{} changed, {conflicted} conflicted", report.changes.len())
    } else {
        format!("{} changed", report.changes.len())
    };

    let mut ordered: Vec<&FileChange> = report.changes.iter().collect();
    ordered
        .sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let lines: String = ordered
        .iter()
        .map(|change| {
            format!("  {}  {}", change.change_type.label(), change.path)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("Branch: {branch}\n{summary}\n{lines}")
}

#[cfg(test)]
mod tests {
    use super::render;
    use super::ChangeType;
    use super::FileChange;
    use super::StatusReport;

    fn change(change_type: ChangeType, path: &str) -> FileChange {
        FileChange {
            change_type,
            path: path.to_owned(),
        }
    }

    #[test]
    fn a_clean_repository_renders_the_header_and_no_changes() {
        let report = StatusReport {
            branch: vec!["main".to_owned()],
            changes: Vec::new(),
        };
        assert_eq!(render(&report), "Branch: main\nNo changes");
    }

    #[test]
    fn an_empty_branch_renders_as_none() {
        let report = StatusReport::default();
        assert_eq!(render(&report), "Branch: (none)\nNo changes");
    }

    #[test]
    fn multiple_bookmarks_are_comma_joined() {
        let report = StatusReport {
            branch: vec!["alpha".to_owned(), "beta".to_owned()],
            changes: Vec::new(),
        };
        assert_eq!(render(&report), "Branch: alpha, beta\nNo changes");
    }

    #[test]
    fn a_single_change_renders_one_changed_and_the_line() {
        let report = StatusReport {
            branch: vec!["main".to_owned()],
            changes: vec![change(ChangeType::Modified, "a.txt")],
        };
        assert_eq!(
            render(&report),
            "Branch: main\n1 changed\n  modified  a.txt"
        );
    }

    #[test]
    fn changes_are_sorted_by_path_in_byte_order() {
        let report = StatusReport {
            branch: Vec::new(),
            changes: vec![
                change(ChangeType::Untracked, "b.txt"),
                change(ChangeType::Added, "Z.txt"),
                change(ChangeType::Modified, "a.txt"),
            ],
        };
        assert_eq!(
            render(&report),
            "Branch: (none)\n3 changed\n  \
             added  Z.txt\n  modified  a.txt\n  untracked  b.txt"
        );
    }

    #[test]
    fn a_conflict_appends_the_conflicted_count_to_the_summary() {
        let report = StatusReport {
            branch: Vec::new(),
            changes: vec![
                change(ChangeType::Modified, "a.txt"),
                change(ChangeType::Conflicted, "b.txt"),
            ],
        };
        assert_eq!(
            render(&report),
            "Branch: (none)\n2 changed, 1 conflicted\n  \
             modified  a.txt\n  conflicted  b.txt"
        );
    }

    #[test]
    fn every_change_type_renders_its_label() {
        let report = StatusReport {
            branch: Vec::new(),
            changes: vec![
                change(ChangeType::Added, "1"),
                change(ChangeType::Modified, "2"),
                change(ChangeType::Deleted, "3"),
                change(ChangeType::Untracked, "4"),
                change(ChangeType::Conflicted, "5"),
            ],
        };
        assert_eq!(
            render(&report),
            "Branch: (none)\n5 changed, 1 conflicted\n  \
             added  1\n  modified  2\n  deleted  3\n  \
             untracked  4\n  conflicted  5"
        );
    }
}
