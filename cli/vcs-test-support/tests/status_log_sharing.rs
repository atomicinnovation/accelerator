use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use vcs_test_support::family::CopyableFamily;
use vcs_test_support::family::SharedFamily;
use vcs_test_support::status_log::States;
use vcs_test_support::status_log::StatusLogPart;
use vcs_test_support::status_log::ADVERSARIAL_STATE;
use vcs_test_support::status_log_manifest::StatusLogManifest;

type TestError = Box<dyn std::error::Error>;

#[test]
fn a_written_manifest_reads_back_unchanged() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let written = StatusLogManifest {
        states: BTreeMap::from([
            ("clean-git".to_owned(), directory.path().join("clean-git")),
            ("dirty-jj".to_owned(), directory.path().join("dirty-jj")),
        ]),
    };
    written.write(directory.path())?;

    let read = StatusLogManifest::read(directory.path())?;

    assert_eq!(read.states, written.states);
    Ok(())
}

#[test]
fn reading_a_directory_without_a_manifest_fails_naming_it(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;

    let error = StatusLogManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(
        error
            .to_string()
            .contains(&directory.path().display().to_string()),
        "{error}"
    );
    Ok(())
}

#[test]
fn reading_a_malformed_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("status-log-states.tsv"),
        "clean-git\n",
    )?;

    let error = StatusLogManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("malformed"), "{error}");
    Ok(())
}

#[test]
fn reading_an_empty_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("status-log-states.tsv"), "")?;

    let error = StatusLogManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("empty"), "{error}");
    Ok(())
}

#[test]
fn a_manifest_names_each_state_on_its_own_line() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    StatusLogManifest {
        states: BTreeMap::from([(
            "clean-git".to_owned(),
            PathBuf::from("/fixtures/clean-git"),
        )]),
    }
    .write(directory.path())?;

    assert_eq!(
        fs::read_to_string(directory.path().join("status-log-states.tsv"))?,
        "clean-git\t/fixtures/clean-git\n"
    );
    Ok(())
}

fn written_states(base: &Path) -> Result<StatusLogManifest, TestError> {
    let manifest = StatusLogManifest {
        states: BTreeMap::from([
            ("clean-git".to_owned(), base.join("clean-git")),
            (ADVERSARIAL_STATE.to_owned(), base.join(ADVERSARIAL_STATE)),
        ]),
    };
    manifest.write(base)?;
    Ok(manifest)
}

#[test]
fn adopting_a_tree_reads_its_manifest() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;
    let written = written_states(&base)?;

    let adopted = States::adopt(directory.path())?;

    assert_eq!(adopted.base, base);
    assert_eq!(adopted.states, written.states);
    Ok(())
}

#[test]
fn adopting_a_tree_without_a_manifest_fails_naming_it() -> Result<(), TestError>
{
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;

    let error = States::adopt(&base).err().ok_or("it should fail")?;

    assert!(
        error.to_string().contains(&base.display().to_string()),
        "{error}"
    );
    Ok(())
}

#[test]
fn adopting_a_malformed_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("status-log-states.tsv"),
        "clean-git\n",
    )?;

    let error = States::adopt(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("malformed"), "{error}");
    Ok(())
}

#[test]
fn adopting_an_empty_manifest_fails() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("status-log-states.tsv"), "")?;

    let error = States::adopt(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("empty"), "{error}");
    Ok(())
}

#[test]
fn the_states_with_goldens_leave_out_the_adversarial_state(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    written_states(directory.path())?;
    let states = States::adopt(directory.path())?;

    let with_goldens: Vec<&str> =
        states.with_goldens().map(|(key, _)| key).collect();

    assert_eq!(with_goldens, ["clean-git"]);
    Ok(())
}

#[test]
fn a_state_is_found_by_its_key() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;
    written_states(&base)?;
    let states = States::adopt(&base)?;

    assert_eq!(states.get("clean-git")?, base.join("clean-git"));
    let error = states.get("dirty-jj").err().ok_or("it should fail")?;
    assert!(error.to_string().contains("dirty-jj"), "{error}");
    Ok(())
}

#[test]
fn everything_is_every_entry_but_the_manifest_and_the_hermetic_environment(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    for state in ["clean-git", "seed", "hermetic-home", "hermetic-xdg"] {
        fs::create_dir(directory.path().join(state))?;
    }
    fs::write(directory.path().join("hermetic-jj.toml"), "")?;
    written_states(directory.path())?;

    let entries = StatusLogPart::Everything.entries(directory.path())?;

    assert_eq!(entries, ["clean-git", "seed"]);
    Ok(())
}

#[test]
fn the_plain_git_and_conflict_parts_name_their_states() -> Result<(), TestError>
{
    let directory = tempfile::tempdir()?;

    assert_eq!(
        StatusLogPart::PlainGit.entries(directory.path())?,
        ["clean-git", "dirty-git", "detached-head-git"]
    );
    assert_eq!(
        StatusLogPart::Conflicts.entries(directory.path())?,
        ["conflict-git", "conflict-jj"]
    );
    Ok(())
}

#[test]
fn rebased_states_keep_only_the_copied_entries_at_their_new_base(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;
    written_states(&base)?;
    let onto = PathBuf::from("/copy");

    let rebased =
        States::adopt(&base)?.rebased(&base, &onto, &["clean-git".to_owned()]);

    assert_eq!(rebased.base, onto);
    assert_eq!(
        rebased.states,
        BTreeMap::from([("clean-git".to_owned(), onto.join("clean-git"))])
    );
    Ok(())
}
