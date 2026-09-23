//! The jj dirty paths honour the git excludes jj-cli layers beneath
//! `.gitignore`, read through the fixture binary under a hermetic
//! environment and compared with `jj status`.
#![cfg(feature = "bash-parity")]

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use support::dirty_paths;
use support::TestError;
use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::jj_status::changed_paths;

const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];
const IGNORED: &str = "meta/ignored.md";
const VISIBLE: &str = "meta/visible.md";
const PATTERN: &str = "ignored.md\n";

struct Excludes {
    work: TempDir,
    env: Hermetic,
    root: PathBuf,
    colocation: &'static str,
}

impl Excludes {
    fn new(colocation: &'static str) -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempfile::Builder::new()
            .prefix("vcs-dirty-excludes-")
            .tempdir()?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(root.join("meta"))?;
        env.jj(&["git", "init", colocation], &root)?;
        fs::write(root.join("meta/base.md"), "base")?;
        env.jj(&["commit", "-m", "base"], &root)?;
        Ok(Self {
            work,
            env,
            root,
            colocation,
        })
    }

    fn global_config(&self, excludes_file: &str) -> Result<PathBuf, TestError> {
        let config = self.work.path().join("global.gitconfig");
        write(
            &config,
            &format!("[core]\n\texcludesFile = {excludes_file}\n"),
        )?;
        Ok(config)
    }

    fn backing_exclude(&self) -> PathBuf {
        if self.colocation == "--colocate" {
            self.root.join(".git/info/exclude")
        } else {
            self.root.join(".jj/repo/store/git/info/exclude")
        }
    }

    fn xdg(&self) -> Result<PathBuf, TestError> {
        Ok(self
            .env
            .jj_user_config_dir()?
            .parent()
            .ok_or("no xdg")?
            .to_path_buf())
    }

    fn untracked(&self) -> Result<(), TestError> {
        write(&self.root.join(IGNORED), "ignored")?;
        write(&self.root.join(VISIBLE), "visible")
    }

    fn listed_under(
        &self,
        env: &Hermetic,
    ) -> Result<BTreeSet<String>, TestError> {
        let output = support::query(env, "dirty_paths", &self.root)?;
        let stderr = String::from_utf8(output.stderr)?;
        assert!(!stderr.contains("WARN"), "{}: {stderr}", self.colocation);
        let listed = dirty_paths(env, &self.root)?;
        assert!(listed.contains(VISIBLE), "{}: {listed:?}", self.colocation);
        assert_eq!(
            listed,
            changed_paths(env, &self.root)?,
            "{}",
            self.colocation
        );
        Ok(listed)
    }
}

fn write(path: &Path, content: &str) -> Result<(), TestError> {
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn for_each_colocation(
    case: impl Fn(&Excludes) -> Result<(), TestError>,
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        case(&Excludes::new(colocation)?)?;
    }
    Ok(())
}

#[test]
fn the_file_core_excludes_file_names_is_honoured() -> Result<(), TestError> {
    for_each_colocation(|repo| {
        let excludes = repo.work.path().join("global-ignore");
        write(&excludes, PATTERN)?;
        let config = repo.global_config(&excludes.display().to_string())?;
        repo.untracked()?;

        let listed = repo
            .listed_under(&repo.env.clone().with_git_global_config(config))?;

        assert!(!listed.contains(IGNORED), "{listed:?}");
        Ok(())
    })
}

#[test]
fn the_backing_repositorys_info_exclude_is_honoured() -> Result<(), TestError> {
    for_each_colocation(|repo| {
        write(&repo.backing_exclude(), PATTERN)?;
        repo.untracked()?;

        let listed = repo.listed_under(&repo.env)?;

        assert!(!listed.contains(IGNORED), "{listed:?}");
        Ok(())
    })
}

#[test]
fn unset_core_excludes_file_falls_back_to_the_xdg_git_ignore(
) -> Result<(), TestError> {
    for_each_colocation(|repo| {
        write(&repo.xdg()?.join("git/ignore"), PATTERN)?;
        repo.untracked()?;

        let listed = repo.listed_under(&repo.env)?;

        assert!(!listed.contains(IGNORED), "{listed:?}");
        Ok(())
    })
}

#[test]
fn unset_core_excludes_file_without_xdg_falls_back_to_home(
) -> Result<(), TestError> {
    for_each_colocation(|repo| {
        write(&repo.env.home().join(".config/git/ignore"), PATTERN)?;
        repo.untracked()?;

        for env in [
            repo.env.clone().without_xdg_config_home(),
            repo.env.clone().with_empty_xdg_config_home(),
        ] {
            let listed = repo.listed_under(&env)?;
            assert!(!listed.contains(IGNORED), "{listed:?}");
        }
        Ok(())
    })
}

#[test]
fn a_relative_core_excludes_file_resolves_against_the_workspace_root(
) -> Result<(), TestError> {
    for_each_colocation(|repo| {
        write(&repo.root.join("ignore-file"), PATTERN)?;
        let config = repo.global_config("ignore-file")?;
        repo.untracked()?;

        let listed = repo
            .listed_under(&repo.env.clone().with_git_global_config(config))?;

        assert!(!listed.contains(IGNORED), "{listed:?}");
        Ok(())
    })
}

#[test]
fn a_tilde_core_excludes_file_resolves_against_home() -> Result<(), TestError> {
    for_each_colocation(|repo| {
        write(&repo.env.home().join("ignore-file"), PATTERN)?;
        let config = repo.global_config("~/ignore-file")?;
        repo.untracked()?;

        let listed = repo
            .listed_under(&repo.env.clone().with_git_global_config(config))?;

        assert!(!listed.contains(IGNORED), "{listed:?}");
        Ok(())
    })
}

#[test]
fn a_configured_excludes_file_replaces_the_xdg_default() -> Result<(), TestError>
{
    for_each_colocation(|repo| {
        let excludes = repo.work.path().join("other-ignore");
        write(&excludes, "unrelated.md\n")?;
        write(&repo.xdg()?.join("git/ignore"), PATTERN)?;
        let config = repo.global_config(&excludes.display().to_string())?;
        repo.untracked()?;

        let listed = repo
            .listed_under(&repo.env.clone().with_git_global_config(config))?;

        assert!(listed.contains(IGNORED), "{listed:?}");
        Ok(())
    })
}

#[test]
fn a_tracked_file_matching_an_exclude_is_still_listed() -> Result<(), TestError>
{
    for_each_colocation(|repo| {
        write(&repo.root.join("meta/tracked.md"), "one")?;
        repo.env.jj(&["commit", "-m", "tracked"], &repo.root)?;
        write(&repo.backing_exclude(), "tracked.md\n")?;
        write(&repo.root.join("meta/tracked.md"), "two")?;
        repo.untracked()?;

        let listed = repo.listed_under(&repo.env)?;

        assert!(listed.contains("meta/tracked.md"), "{listed:?}");
        Ok(())
    })
}

#[cfg(unix)]
#[test]
fn an_unreadable_excludes_file_is_warned_about_and_skipped(
) -> Result<(), TestError> {
    use std::os::unix::fs::PermissionsExt as _;

    for_each_colocation(|repo| {
        let excludes = repo.work.path().join("unreadable-ignore");
        write(&excludes, PATTERN)?;
        fs::set_permissions(&excludes, fs::Permissions::from_mode(0o000))?;
        let config = repo.global_config(&excludes.display().to_string())?;
        repo.untracked()?;
        let env = repo.env.clone().with_git_global_config(config);

        let output = support::query(&env, "dirty_paths", &repo.root)?;
        fs::set_permissions(&excludes, fs::Permissions::from_mode(0o644))?;

        assert!(output.status.success());
        let listed = String::from_utf8(output.stdout)?;
        assert!(listed.lines().any(|line| line == IGNORED), "{listed}");
        let stderr = String::from_utf8(output.stderr)?;
        assert!(stderr.contains("WARN"), "{stderr}");
        assert!(stderr.contains("unreadable-ignore"), "{stderr}");
        Ok(())
    })
}

#[test]
fn without_a_git_backend_the_global_excludes_file_is_honoured_as_jj_status_does(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempfile::Builder::new()
        .prefix("vcs-dirty-excludes-simple-")
        .tempdir()?;
    let base = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    base.jj(&["debug", "init-simple", "repo"], work.path())?;
    let excludes = work.path().join("global-ignore");
    fs::write(&excludes, PATTERN)?;
    let config = work.path().join("global.gitconfig");
    fs::write(
        &config,
        format!("[core]\n\texcludesFile = {}\n", excludes.display()),
    )?;
    fs::create_dir_all(root.join("meta"))?;
    fs::write(root.join(IGNORED), "ignored")?;
    fs::write(root.join(VISIBLE), "visible")?;
    let env = base.with_git_global_config(config);

    let listed = dirty_paths(&env, &root)?;

    assert!(listed.contains(VISIBLE), "{listed:?}");
    assert!(!listed.contains(IGNORED), "{listed:?}");
    assert_eq!(listed, changed_paths(&env, &root)?);
    Ok(())
}
