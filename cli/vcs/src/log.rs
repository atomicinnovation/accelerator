//! The backend-neutral log model and its pure renderer.
//!
//! Both adapters populate a [`LogReport`] of up to five entries, newest first;
//! this module owns the ADR-0066 wording for the empty and empty-subject cases.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub short_id: String,
    pub subject: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogReport {
    pub entries: Vec<LogEntry>,
}

#[must_use]
pub fn render(report: &LogReport) -> String {
    if report.entries.is_empty() {
        return "No commits".to_owned();
    }
    report
        .entries
        .iter()
        .map(|entry| {
            let subject = if entry.subject.is_empty() {
                "(no description)"
            } else {
                entry.subject.as_str()
            };
            format!("{} {subject}", entry.short_id)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::render;
    use super::LogEntry;
    use super::LogReport;

    fn entry(short_id: &str, subject: &str) -> LogEntry {
        LogEntry {
            short_id: short_id.to_owned(),
            subject: subject.to_owned(),
        }
    }

    #[test]
    fn no_commits_renders_the_empty_literal() {
        assert_eq!(render(&LogReport::default()), "No commits");
    }

    #[test]
    fn an_entry_renders_its_id_and_subject() {
        let report = LogReport {
            entries: vec![entry("abc123def456", "init")],
        };
        assert_eq!(render(&report), "abc123def456 init");
    }

    #[test]
    fn an_empty_subject_renders_the_placeholder() {
        let report = LogReport {
            entries: vec![entry("abc123def456", "")],
        };
        assert_eq!(render(&report), "abc123def456 (no description)");
    }

    #[test]
    fn five_entries_render_newest_first_one_per_line() {
        let report = LogReport {
            entries: vec![
                entry("id5", "commit-5"),
                entry("id4", "commit-4"),
                entry("id3", "commit-3"),
                entry("id2", "commit-2"),
                entry("id1", "commit-1"),
            ],
        };
        assert_eq!(
            render(&report),
            "id5 commit-5\nid4 commit-4\nid3 commit-3\n\
             id2 commit-2\nid1 commit-1"
        );
    }
}
