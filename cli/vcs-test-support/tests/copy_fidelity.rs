//! A copy of the real status-log states reads exactly as its source does.
//!
//! Each test adopts the set in one workbench and copies it into another with
//! `copy_part`, the mechanism under `Workbench::writable`, so a copy never
//! lands on the set it came from.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;

use filetime::FileTime;
use vcs_test_support::copies::copy_part;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::status_log::StatusLogPart;
use vcs_test_support::status_log::ADVERSARIAL_STATE;
use vcs_test_support::workbench::Workbench;
use walkdir::WalkDir;

type TestError = Box<dyn std::error::Error>;

/// The states copied into `target`, keyed as in the source.
fn copy_everything(
    source_base: &Path,
    states: &BTreeMap<String, PathBuf>,
    target: &Workbench,
) -> Result<BTreeMap<String, PathBuf>, TestError> {
    let entries = StatusLogPart::Everything.entries(source_base)?;
    let names: Vec<&str> = entries.iter().map(String::as_str).collect();
    let copy = copy_part(source_base, &names, target.path())?;
    assert!(copy.is_clean(), "{:?}", copy.named_by);
    states
        .iter()
        .map(|(key, path)| {
            Ok((
                key.clone(),
                target.path().join(path.strip_prefix(source_base)?),
            ))
        })
        .collect()
}

fn modification_times(
    tree: &Path,
) -> Result<BTreeMap<PathBuf, FileTime>, TestError> {
    let mut times = BTreeMap::new();
    for entry in WalkDir::new(tree) {
        let entry = entry?;
        let metadata = entry.path().symlink_metadata()?;
        times.insert(
            entry.path().strip_prefix(tree)?.to_path_buf(),
            FileTime::from_last_modification_time(&metadata),
        );
    }
    Ok(times)
}

fn is_jj(state: &Path) -> bool {
    state.join(".jj").is_dir()
}

fn is_git(state: &Path) -> bool {
    state.join(".git").exists()
}

#[test]
fn a_copy_of_every_state_keeps_every_modification_time() -> Result<(), TestError>
{
    let source = Workbench::for_this_test()?;
    let states = source.status_log_states()?;
    let target = Workbench::for_this_test()?;

    let copied = copy_everything(&states.base, &states.states, &target)?;

    for (key, state) in &states.states {
        let copy = copied.get(key).ok_or("a state was not copied")?;
        assert_eq!(
            modification_times(copy)?,
            modification_times(state)?,
            "{key}"
        );
    }
    let tree_state = Path::new(".jj/working_copy/tree_state");
    assert!(
        copied
            .values()
            .filter(|copy| is_jj(copy))
            .all(|copy| copy.join(tree_state).is_file()),
        "every jj state keeps its tree state"
    );
    Ok(())
}

#[test]
fn a_copy_reports_the_status_its_source_reports() -> Result<(), TestError> {
    let source = Workbench::for_this_test()?;
    let states = source.status_log_states()?;
    let target = Workbench::for_this_test()?;

    let copied = copy_everything(&states.base, &states.states, &target)?;

    let read = |env: &Hermetic, state: &Path| -> Result<String, TestError> {
        let mut report = String::new();
        if is_git(state) {
            report += &env.git(
                &["--no-optional-locks", "status", "--porcelain"],
                state,
            )?;
        }
        if is_jj(state) {
            report += &env.jj(&["--ignore-working-copy", "status"], state)?;
        }
        Ok(report)
    };
    for (key, state) in &states.states {
        if key == ADVERSARIAL_STATE {
            continue;
        }
        let copy = copied.get(key).ok_or("a state was not copied")?;
        assert_eq!(
            read(target.hermetic(), copy)?,
            read(source.hermetic(), state)?,
            "{key}"
        );
    }
    Ok(())
}

#[test]
fn a_snapshot_in_a_copy_sees_exactly_the_edits_the_builder_made(
) -> Result<(), TestError> {
    let source = Workbench::for_this_test()?;
    let states = source.status_log_states()?;
    let target = Workbench::for_this_test()?;
    let copied = copy_everything(&states.base, &states.states, &target)?;
    let env = target.hermetic();
    let operations = |state: &Path| {
        env.jj(
            &[
                "--ignore-working-copy",
                "op",
                "log",
                "--no-graph",
                "-T",
                "id ++ \"\\n\"",
            ],
            state,
        )
    };

    for (key, copy) in copied.iter().filter(|(_, copy)| is_jj(copy)) {
        let before = operations(copy)?;
        let summary = env.jj(&["diff", "--summary", "-r", "@"], copy)?;
        let expected = match key.as_str() {
            "dirty-jj" => "A new-file.txt",
            "deleted-jj" => "D gone.txt",
            "rename-jj" => "R {old.txt => new.txt}",
            _ => "",
        };
        assert_eq!(summary, expected, "{key}");
        if expected.is_empty() {
            assert_eq!(operations(copy)?, before, "{key} recorded a snapshot");
        }
    }
    Ok(())
}

#[test]
fn a_copy_of_a_real_state_planted_with_its_path_names_the_file(
) -> Result<(), TestError> {
    let planted = Workbench::for_this_test()?;
    let conflicts =
        planted.writable_status_log_states(StatusLogPart::Conflicts)?;
    let config = conflicts.get("conflict-git")?.join(".git/config");
    writeln!(
        OpenOptions::new().append(true).open(&config)?,
        "[remote \"planted\"]\n\turl = {}",
        planted.path().display()
    )?;
    let target = Workbench::for_this_test()?;

    let copy = copy_part(planted.path(), &["conflict-git"], target.path())?;

    assert_eq!(
        copy.named_by,
        vec![target.path().join("conflict-git/.git/config")]
    );
    Ok(())
}
