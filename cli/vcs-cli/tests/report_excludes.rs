//! `accelerator-vcs status` on jj lists the changes `jj status` does: files
//! the git excludes hide, and new files over `snapshot.max-new-file-size`, are
//! left out, and everything else is listed.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use vcs_test_support::hermetic::Hermetic;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");
const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];

struct Repo {
    work: tempfile::TempDir,
    env: Hermetic,
    root: PathBuf,
    colocation: &'static str,
}

impl Repo {
    fn new(colocation: &'static str) -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempfile::Builder::new()
            .prefix("vcs-report-excludes-")
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

    fn backing_exclude(&self) -> PathBuf {
        if self.colocation == "--colocate" {
            self.root.join(".git/info/exclude")
        } else {
            self.root.join(".jj/repo/store/git/info/exclude")
        }
    }

    fn with_global_excludes(
        &self,
        patterns: &str,
    ) -> Result<Hermetic, TestError> {
        let excludes = self.work.path().join("global-ignore");
        fs::write(&excludes, patterns)?;
        let config = self.work.path().join("global.gitconfig");
        fs::write(
            &config,
            format!("[core]\n\texcludesFile = {}\n", excludes.display()),
        )?;
        Ok(self.env.clone().with_git_global_config(config))
    }

    fn status(&self, env: &Hermetic) -> Result<String, TestError> {
        let mut command = Command::new(BIN);
        env.apply(&mut command);
        let output = command.arg("status").current_dir(&self.root).output()?;
        assert!(output.status.success());
        let rendered = String::from_utf8(output.stdout)?;
        assert!(!rendered.contains("(status unavailable)"), "{rendered}");
        Ok(rendered)
    }
}

fn write(path: &Path, content: &str) -> Result<(), TestError> {
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn assert_hidden_beside_a_listed_file(
    repo: &Repo,
    env: &Hermetic,
) -> Result<(), TestError> {
    write(&repo.root.join("meta/ignored.md"), "ignored")?;
    write(&repo.root.join("meta/visible.md"), "visible")?;
    let rendered = repo.status(env)?;
    assert!(!rendered.contains("meta/ignored.md"), "{rendered}");
    assert!(rendered.contains("meta/visible.md"), "{rendered}");
    Ok(())
}

#[test]
fn a_file_the_global_excludes_file_hides_is_not_listed() -> Result<(), TestError>
{
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        let env = repo.with_global_excludes("ignored.md\n")?;
        assert_hidden_beside_a_listed_file(&repo, &env)?;
    }
    Ok(())
}

#[test]
fn a_file_the_backing_info_exclude_hides_is_not_listed() -> Result<(), TestError>
{
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        write(&repo.backing_exclude(), "ignored.md\n")?;
        assert_hidden_beside_a_listed_file(&repo, &repo.env)?;
    }
    Ok(())
}

#[test]
fn a_file_the_git_excludes_do_not_hide_is_listed() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        write(&repo.root.join("meta/tracked.md"), "one")?;
        repo.env.jj(&["commit", "-m", "tracked"], &repo.root)?;
        let env = repo.with_global_excludes("unrelated.md\n")?;
        let xdg = repo.env.jj_user_config_dir()?;
        write(&xdg.with_file_name("git").join("ignore"), "ignored.md\n")?;
        write(&repo.backing_exclude(), "tracked.md\n")?;
        write(&repo.root.join("meta/ignored.md"), "ignored")?;
        write(&repo.root.join("meta/tracked.md"), "two")?;

        let rendered = repo.status(&env)?;

        assert!(rendered.contains("meta/ignored.md"), "{rendered}");
        assert!(rendered.contains("meta/tracked.md"), "{rendered}");
    }
    Ok(())
}

#[test]
fn a_new_file_over_the_limit_is_not_listed() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        repo.env.jj(
            &[
                "config",
                "set",
                "--repo",
                "snapshot.max-new-file-size",
                "1KiB",
            ],
            &repo.root,
        )?;
        fs::write(repo.root.join("meta/two"), vec![b'x'; 2048])?;
        fs::write(repo.root.join("meta/one"), vec![b'x'; 1024])?;
        fs::write(repo.root.join("meta/half"), vec![b'x'; 512])?;

        let rendered = repo.status(&repo.env)?;

        assert!(!rendered.contains("meta/two"), "{rendered}");
        assert!(rendered.contains("meta/one"), "{rendered}");
        assert!(rendered.contains("meta/half"), "{rendered}");
    }
    Ok(())
}

#[test]
fn a_new_file_over_the_default_limit_is_not_listed() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let repo = Repo::new(colocation)?;
        fs::write(repo.root.join("meta/two"), vec![b'x'; 2 * 1024 * 1024])?;
        fs::write(repo.root.join("meta/half"), vec![b'x'; 512])?;

        let rendered = repo.status(&repo.env)?;

        assert!(!rendered.contains("meta/two"), "{rendered}");
        assert!(rendered.contains("meta/half"), "{rendered}");
    }
    Ok(())
}

#[test]
fn an_invalid_limit_is_warned_about_and_every_new_file_listed(
) -> Result<(), TestError> {
    let repo = Repo::new("--no-colocate")?;
    repo.env.jj(
        &[
            "config",
            "set",
            "--repo",
            "snapshot.max-new-file-size",
            "\"abc\"",
        ],
        &repo.root,
    )?;
    fs::write(repo.root.join("meta/a.md"), "a")?;
    fs::write(repo.root.join("meta/two"), vec![b'x'; 2 * 1024 * 1024])?;

    let mut command = Command::new(BIN);
    repo.env.apply(&mut command);
    let output = command
        .arg("status")
        .env("ACCELERATOR_LOG", "warn")
        .current_dir(&repo.root)
        .output()?;

    let rendered = String::from_utf8(output.stdout)?;
    assert!(!rendered.contains("(status unavailable)"), "{rendered}");
    assert!(rendered.contains("meta/a.md"), "{rendered}");
    assert!(rendered.contains("meta/two"), "{rendered}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("WARN"), "{stderr}");
    assert!(stderr.contains("snapshot.max-new-file-size"), "{stderr}");
    Ok(())
}
