//! `InProcessProbe` through the `vcs::RepositoryProbe` port, against real git
//! and jj repositories, read through the fixture binary under a hermetic
//! environment.
#![cfg(feature = "bash-parity")]

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use support::TestError;
use vcs_test_support::fixtures::pure_jj;
use vcs_test_support::hermetic::Hermetic;

fn answer(
    env: &Hermetic,
    name: &str,
    start: &Path,
) -> Result<String, TestError> {
    let output = support::query(env, name, start)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    let listing = String::from_utf8(output.stdout)?;
    Ok(listing
        .trim_end()
        .strip_prefix(&format!("{name}\t"))
        .ok_or_else(|| format!("no {name} line in {listing:?}"))?
        .to_owned())
}

fn git_repository(
    base: &Path,
    env: &Hermetic,
) -> Result<std::path::PathBuf, TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let root = base.join("repo");
    fs::create_dir_all(root.join("meta/work"))?;
    env.git(&["init", "--quiet"], &root)?;
    env.git(&["config", "--local", "user.name", "Git Author"], &root)?;
    fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
    env.git(&["add", "meta"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;
    Ok(root.canonicalize()?)
}

#[test]
fn git_facts_name_the_repository_and_its_revision() -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let root = git_repository(base.path(), &env)?;
    let head = support::revision(&env, &root)?;

    assert_eq!(
        answer(&env, "facts_at", &root.join("meta/work"))?,
        format!("{} repo git {head}", root.display())
    );
    Ok(())
}

#[test]
fn jj_facts_name_the_repository() -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let root = pure_jj(&base.path().join("repo"), &env)?.canonicalize()?;
    let revision = support::revision(&env, &root)?;

    assert_eq!(
        answer(&env, "facts_at", &root)?,
        format!("{} repo jj {revision}", root.display())
    );
    Ok(())
}

#[test]
fn the_user_name_is_read_from_the_discovered_repository(
) -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let git = git_repository(base.path(), &env)?;
    let jj = pure_jj(&base.path().join("jj"), &env)?;

    assert_eq!(
        answer(&env, "user_name_at", &git.join("meta/work"))?,
        "Git Author"
    );
    assert_eq!(answer(&env, "user_name_at", &jj)?, "Fixture");
    Ok(())
}

#[test]
fn outside_a_repository_there_are_no_facts_no_user_name_and_no_changes(
) -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let loose = base.path().join("loose");
    fs::create_dir_all(&loose)?;
    vcs_test_support::hermetic::assert_no_repository_ancestor(&loose)?;
    fs::write(loose.join("item.md"), "x\n")?;

    assert_eq!(answer(&env, "facts_at", &loose)?, "absent");
    assert_eq!(answer(&env, "user_name_at", &loose)?, "absent");
    let state = support::working_copy_state(&env, &loose)?;
    assert!(state.base_commits.is_empty());
    assert!(state.dirty_paths.is_empty());
    Ok(())
}

#[test]
fn git_working_copy_state_is_head_and_the_changed_paths(
) -> Result<(), TestError> {
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let root = git_repository(base.path(), &env)?;
    let head = support::revision(&env, &root)?;
    fs::write(root.join("meta/work/0001-a.md"), "two\n")?;
    fs::write(root.join("meta/work/0002-new.md"), "new\n")?;

    let state = support::working_copy_state(&env, &root)?;

    assert_eq!(state.base_commits, [head]);
    assert_eq!(
        state.dirty_paths,
        BTreeSet::from([
            "meta/work/0001-a.md".to_owned(),
            "meta/work/0002-new.md".to_owned(),
        ])
    );
    Ok(())
}

#[test]
fn jj_working_copy_state_is_the_parent_and_the_changed_paths(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let base = tempfile::tempdir()?;
    let env = Hermetic::rooted_at(base.path())?;
    let root = base.path().join("repo");
    fs::create_dir_all(root.join("meta/work"))?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
    env.jj(&["commit", "-m", "init"], &root)?;
    let parent =
        env.jj(&["log", "--no-graph", "-r", "@-", "-T", "commit_id"], &root)?;
    fs::write(root.join("meta/work/0002-new.md"), "new\n")?;

    let state = support::working_copy_state(&env, &root)?;

    assert_eq!(state.base_commits, [parent.trim().to_owned()]);
    assert_eq!(
        state.dirty_paths,
        BTreeSet::from(["meta/work/0002-new.md".to_owned()])
    );
    Ok(())
}
