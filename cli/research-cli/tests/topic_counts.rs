//! The counts a committed set's files give, beside those its manifest and
//! synthesis record.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use serde_json::json;
use serde_json::Value;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-research");
const TOPICS: &str = "meta/research/topics";
const MULTIROUND: &str = "topic-research-multiround-set";
const SETS: [&str; 4] = [
    "topic-research-deepened-set",
    "topic-research-multiprofile-set",
    MULTIROUND,
    "topic-research-quarantine-set",
];

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn with_fixture_set(fixture: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix("research-topic-counts-")
            .tempdir()?;
        let root = dir.path().canonicalize()?;
        fs::create_dir_all(root.join(".git"))?;
        let project = Self { _dir: dir, root };
        copy_tree(&fixtures().join(fixture), &project.set(fixture))?;
        Ok(project)
    }

    fn set(&self, slug: &str) -> PathBuf {
        self.root.join(TOPICS).join(slug)
    }

    fn edit(
        &self,
        document: &str,
        from: &str,
        to: &str,
    ) -> Result<(), TestError> {
        let path = self.set(MULTIROUND).join(document);
        let text = fs::read_to_string(&path)?;
        assert!(text.contains(from), "{from:?} in {document}");
        fs::write(path, text.replacen(from, to, 1))?;
        Ok(())
    }

    fn counts_output(&self, slug: &str) -> Result<Output, TestError> {
        Ok(Command::new(BIN)
            .current_dir(&self.root)
            .args(["topic", "counts", slug])
            .output()?)
    }

    fn counts(&self, slug: &str) -> Result<Value, TestError> {
        let output = self.counts_output(slug)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(serde_json::from_slice(&output.stdout)?)
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

fn stored_manifest_count(
    project: &Project,
    slug: &str,
    key: &str,
) -> Result<u64, TestError> {
    let manifest = fs::read_to_string(project.set(slug).join("manifest.md"))?;
    let line = manifest
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
        .ok_or_else(|| format!("{key} in {slug}"))?;
    Ok(line.parse()?)
}

#[test]
fn counts_derives_each_committed_set_s_stored_counts() -> Result<(), TestError>
{
    for slug in SETS {
        let project = Project::with_fixture_set(slug)?;

        let counts = project.counts(slug)?;

        for key in ["round_count", "finding_count"] {
            assert_eq!(
                counts["derived"][key],
                json!(stored_manifest_count(&project, slug, key)?),
                "{slug} {key}"
            );
        }
    }
    Ok(())
}

#[test]
fn counts_reports_a_synthesised_set_fresh() -> Result<(), TestError> {
    for slug in [MULTIROUND, "topic-research-quarantine-set"] {
        let project = Project::with_fixture_set(slug)?;

        let counts = project.counts(slug)?;

        assert_eq!(counts["fresh"], json!(true), "{slug}: {counts}");
        assert_eq!(counts["mismatches"], json!([]), "{slug}");
    }
    Ok(())
}

#[test]
fn counts_reports_a_set_without_a_synthesis_stale() -> Result<(), TestError> {
    for slug in [
        "topic-research-deepened-set",
        "topic-research-multiprofile-set",
    ] {
        let project = Project::with_fixture_set(slug)?;

        let counts = project.counts(slug)?;

        assert_eq!(counts["stored"]["rounds_covered"], Value::Null, "{slug}");
        assert_eq!(counts["fresh"], json!(false), "{slug}");
        assert_eq!(
            counts["mismatches"],
            json!([
                "the findings reach round 1 but synthesis.md covers up to round 0"
            ]),
            "{slug}"
        );
    }
    Ok(())
}

#[test]
fn counts_reads_past_an_unreadable_stored_count() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;
    project.edit("manifest.md", "round_count: 2", "round_count: \"three\"")?;

    let counts = project.counts(MULTIROUND)?;

    assert_eq!(counts["derived"]["round_count"], json!(2));
    assert_eq!(counts["stored"]["round_count"], Value::Null);
    assert_eq!(counts["fresh"], json!(false));
    assert_eq!(
        counts["mismatches"],
        json!(["manifest.md records round_count 'three', which is not a whole number"])
    );
    Ok(())
}

#[test]
fn counts_names_a_missing_stored_count() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;
    project.edit("manifest.md", "finding_count: 3\n", "")?;

    let counts = project.counts(MULTIROUND)?;

    assert_eq!(
        counts["mismatches"],
        json!(["manifest.md records no finding_count"])
    );
    Ok(())
}

#[test]
fn counts_reads_past_an_unreadable_rounds_covered() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;
    project.edit(
        "synthesis.md",
        "rounds_covered: 2",
        "rounds_covered: \"two\"",
    )?;

    let output = project.counts_output(MULTIROUND)?;
    let counts: Value = serde_json::from_slice(&output.stdout)?;

    assert!(output.status.success());
    assert_eq!(counts["stored"]["rounds_covered"], Value::Null);
    assert_eq!(
        counts["mismatches"],
        json!(["synthesis.md records rounds_covered 'two', which is not a whole number"])
    );
    Ok(())
}

#[test]
fn counts_escapes_an_unreadable_value() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;
    project.edit("manifest.md", "round_count: 2", "round_count: \"3\\nx\"")?;

    let counts = project.counts(MULTIROUND)?;

    assert_eq!(
        counts["mismatches"],
        json!(["manifest.md records round_count '3\\nx', which is not a whole number"])
    );
    Ok(())
}

#[test]
fn counts_names_each_mismatch() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;
    project.edit("manifest.md", "round_count: 2", "round_count: 3")?;
    project.edit("manifest.md", "finding_count: 3", "finding_count: 9")?;
    project.edit("synthesis.md", "rounds_covered: 2", "rounds_covered: 1")?;

    let counts = project.counts(MULTIROUND)?;

    assert_eq!(
        counts["mismatches"],
        json!([
            "manifest.md records round_count 3 but the findings reach round 2",
            "manifest.md records finding_count 9 but 3 findings are on disk",
            "the findings reach round 2 but synthesis.md covers up to round 1",
        ])
    );
    Ok(())
}

#[test]
fn counts_derives_the_report_count() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;
    let reports = project.set(MULTIROUND).join("reports");
    fs::create_dir_all(&reports)?;
    fs::write(reports.join("a.md"), "")?;
    fs::write(reports.join("b.md"), "")?;

    let counts = project.counts(MULTIROUND)?;

    assert_eq!(counts["derived"]["report_count"], json!(2));
    assert_eq!(counts["stored"]["report_count"], Value::Null);
    Ok(())
}

#[test]
fn counts_writes_nothing() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;
    let before = tree(&project.root)?;

    project.counts(MULTIROUND)?;

    assert_eq!(tree(&project.root)?, before);
    Ok(())
}

#[test]
fn counts_for_an_unknown_set_fails() -> Result<(), TestError> {
    let project = Project::with_fixture_set(MULTIROUND)?;

    let output = project.counts_output("no-such-set")?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim_end(),
        "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set 'no-such-set'"
    );
    Ok(())
}
