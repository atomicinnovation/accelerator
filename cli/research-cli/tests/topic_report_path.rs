//! The path a new report on a question takes in a committed set.

use std::fs;
use std::process::Output;

mod topic_project;

use topic_project::tree;
use topic_project::Project;
use topic_project::TestError;

const SET: &str = "topic-research-multiround-set";
const QUESTION: &str = "How do LLM agents plan multi-step tasks?";
const QUESTION_SLUG: &str = "how-do-llm-agents-plan-multi-step-tasks";

impl Project {
    fn with_the_multiround_set() -> Result<Self, TestError> {
        Self::with_fixture_set(SET)
    }

    fn report_path(&self, args: &[&str]) -> Result<Output, TestError> {
        self.run("report-path", args)
    }

    fn printed_path(&self, args: &[&str]) -> Result<String, TestError> {
        let output = self.report_path(args)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(String::from_utf8(output.stdout)?)
    }

    fn report(&self, name: &str) -> String {
        format!("{}/reports/{name}\n", self.set(SET).display())
    }
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_owned()
}

#[test]
fn a_report_path_names_the_question_slug_under_reports() -> Result<(), TestError>
{
    let project = Project::with_the_multiround_set()?;

    let printed = project.printed_path(&[SET, QUESTION])?;

    assert_eq!(printed, project.report(&format!("{QUESTION_SLUG}.md")));
    Ok(())
}

#[test]
fn a_report_path_skips_a_taken_name() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;
    fs::create_dir_all(project.set(SET).join("reports"))?;
    fs::write(
        project.set(SET).join(format!("reports/{QUESTION_SLUG}.md")),
        "",
    )?;

    let printed = project.printed_path(&[SET, QUESTION])?;

    assert_eq!(printed, project.report(&format!("{QUESTION_SLUG}-2.md")));
    Ok(())
}

#[test]
fn a_report_path_counts_any_entry_as_taken() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;
    fs::create_dir_all(
        project.set(SET).join(format!("reports/{QUESTION_SLUG}.md")),
    )?;

    let printed = project.printed_path(&[SET, QUESTION])?;

    assert_eq!(printed, project.report(&format!("{QUESTION_SLUG}-2.md")));
    Ok(())
}

#[test]
fn a_report_path_writes_nothing() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;
    let before = tree(&project.root)?;

    project.printed_path(&[SET, QUESTION])?;

    assert_eq!(tree(&project.root)?, before);
    assert!(!project.set(SET).join("reports").exists());
    Ok(())
}

#[test]
fn a_report_path_joins_its_question_words() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;
    let mut words = vec![SET];
    words.extend(QUESTION.split(' '));

    let printed = project.printed_path(&words)?;

    assert_eq!(printed, project.printed_path(&[SET, QUESTION])?);
    Ok(())
}

#[test]
fn a_report_path_reads_shell_text_literally() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;

    let printed = project.printed_path(&[
        SET,
        "What's the agents' \"best\" planner $(touch pwned)?",
    ])?;

    assert_eq!(
        printed,
        project.report("what-s-the-agents-best-planner-touch-pwned.md")
    );
    Ok(())
}

#[test]
fn a_report_path_without_a_question_fails() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;

    let output = project.report_path(&[SET])?;

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert_eq!(
        stderr_of(&output),
        "E_TOPIC_RESEARCH_QUESTION: report-path needs a question"
    );
    Ok(())
}

#[test]
fn a_report_path_with_a_blank_question_fails() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;

    let output = project.report_path(&[SET, "  "])?;

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert_eq!(
        stderr_of(&output),
        "E_TOPIC_RESEARCH_QUESTION: report-path needs a question"
    );
    Ok(())
}

#[test]
fn a_report_path_takes_a_question_that_starts_with_a_hyphen(
) -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;

    let printed = project
        .printed_path(&[SET, "-ise or -ize: which do agent frameworks use?"])?;

    assert_eq!(
        printed,
        project.report("ise-or-ize-which-do-agent-frameworks-use.md")
    );
    Ok(())
}

#[test]
fn a_report_path_for_an_unknown_set_fails() -> Result<(), TestError> {
    let project = Project::with_the_multiround_set()?;

    let output = project.report_path(&["no-such-set", QUESTION])?;

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert_eq!(
        stderr_of(&output),
        "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set 'no-such-set'"
    );
    Ok(())
}
