//! The path a new report on a question takes in a committed set.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-research");
const TOPICS: &str = "meta/research/topics";
const SET: &str = "topic-research-multiround-set";
const QUESTION: &str = "How do LLM agents plan multi-step tasks?";
const QUESTION_SLUG: &str = "how-do-llm-agents-plan-multi-step-tasks";

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn with_the_multiround_set() -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix("research-topic-report-path-")
            .tempdir()?;
        let root = dir.path().canonicalize()?;
        fs::create_dir_all(root.join(".git"))?;
        let project = Self { _dir: dir, root };
        copy_tree(&fixtures().join(SET), &project.set())?;
        Ok(project)
    }

    fn set(&self) -> PathBuf {
        self.root.join(TOPICS).join(SET)
    }

    fn report_path(&self, args: &[&str]) -> Result<Output, TestError> {
        Ok(Command::new(BIN)
            .current_dir(&self.root)
            .args(["topic", "report-path"])
            .args(args)
            .output()?)
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
        format!("{}/reports/{name}\n", self.set().display())
    }
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../corpus-cli/tests/fixtures")
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), TestError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn tree(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, TestError> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                files.insert(path.clone(), Vec::new());
                pending.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path)?);
            }
        }
    }
    Ok(files)
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
    fs::create_dir_all(project.set().join("reports"))?;
    fs::write(
        project.set().join(format!("reports/{QUESTION_SLUG}.md")),
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
        project.set().join(format!("reports/{QUESTION_SLUG}.md")),
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
    assert!(!project.set().join("reports").exists());
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
