//! One `research topic report-path` call: the path a new report takes.

use std::path::Path;

use corpus::scan::DirReader;
use research::topic::layout::slug::ReportSlug;
use research_adapters::topic::list_reports;

use crate::topic_command::Printed;

/// The question as one string, its words joined with single spaces.
///
/// # Errors
///
/// `E_TOPIC_RESEARCH_QUESTION` when no word holds more than whitespace.
pub fn parse_question(words: &[String]) -> Result<String, String> {
    let question = words.join(" ");
    if question.trim().is_empty() {
        return Err("E_TOPIC_RESEARCH_QUESTION: report-path needs a question"
            .to_owned());
    }
    Ok(question)
}

/// Prints the absolute path a new report on `question` takes in the set.
///
/// # Errors
///
/// A message when the set's `reports/` cannot be listed, so no path is
/// printed that could not be checked.
pub fn run_report_path<F: DirReader>(
    set_root: &Path,
    question: &str,
    fs: &F,
) -> Result<Printed, String> {
    let taken =
        list_reports(set_root, fs).map_err(|error| error.to_string())?;
    let path = ReportSlug::from(question).free_report_path(&taken);
    Ok(Printed {
        stdout: format!("{}\n", set_root.join(path).to_string_lossy()),
        stderr: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use corpus::scan::DirReader;

    use super::run_report_path;

    struct UnlistableReports;

    impl DirReader for UnlistableReports {
        fn list(
            &self,
            _dir: &Path,
        ) -> Result<Option<Vec<String>>, kernel::Error> {
            Err(kernel::Error::Failed("reports/ is unreadable".to_owned()))
        }
    }

    #[test]
    fn a_report_path_fails_when_reports_cannot_be_listed() {
        let printed =
            run_report_path(Path::new("/set"), "A?", &UnlistableReports);

        assert_eq!(printed.err().as_deref(), Some("reports/ is unreadable"));
    }
}
