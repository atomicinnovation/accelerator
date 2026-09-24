//! `VcsWorkingCopyStatus` against a marker-less tree and against real
//! repositories, exercised through the `WorkingCopyStatus` port `work sync`
//! plans against.

use work::sync::Dirtiness;
use work_adapters::sync::fetch::WorkingCopyStatus;
use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

type TestError = Box<dyn std::error::Error>;

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("work-working-copy-status-{tag}-"))
        .tempdir()?)
}

#[test]
fn a_tree_with_no_repository_answers_unknown() -> Result<(), TestError> {
    let loose = tempdir("loose")?;
    vcs_test_support::hermetic::assert_no_repository_ancestor(loose.path())?;
    std::fs::write(loose.path().join("item.md"), "x\n")?;

    let status = VcsWorkingCopyStatus::probed_from(loose.path());

    assert_eq!(
        status.is_dirty(&loose.path().join("item.md")),
        Dirtiness::Unknown
    );
    Ok(())
}

#[cfg(feature = "bash-parity")]
mod against_a_real_repository {
    use std::fs;

    use vcs_test_support::hermetic::Hermetic;
    use work::sync::Dirtiness;
    use work_adapters::sync::fetch::WorkingCopyStatus;
    use work_adapters::sync::working_copy_status::VcsWorkingCopyStatus;

    use super::tempdir;
    use super::TestError;

    const FIXTURE: &str = env!("CARGO_BIN_EXE_work-adapters-fixture");

    fn dirtiness(
        env: &Hermetic,
        root: &std::path::Path,
        path: &std::path::Path,
    ) -> Result<String, TestError> {
        let mut command = std::process::Command::new(FIXTURE);
        command.arg(root).arg(path);
        env.apply(&mut command);
        let output = command.output()?;
        let listing = String::from_utf8(output.stdout)?;
        Ok(listing
            .trim_end()
            .rsplit_once('\t')
            .ok_or_else(|| format!("no dirtiness in {listing:?}"))?
            .1
            .to_owned())
    }

    #[test]
    fn git_reports_a_modified_tracked_file_as_dirty() -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let work = tempdir("git-dirty")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(root.join("meta/work"))?;
        env.git(&["init", "--quiet"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
        fs::write(root.join("meta/work/0002-b.md"), "one\n")?;
        env.git(&["add", "meta"], &root)?;
        env.git(&["commit", "--quiet", "-m", "init"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "two\n")?;

        let status = VcsWorkingCopyStatus::probed_from(&root);

        assert_eq!(
            status.is_dirty(&root.join("meta/work/0001-a.md")),
            Dirtiness::Dirty
        );
        assert_eq!(
            status.is_dirty(&root.join("meta/work/0002-b.md")),
            Dirtiness::Clean
        );
        Ok(())
    }

    /// A never-committed work item is the case the pull gate exists for:
    /// no commit holds its content, so overwriting it is unrecoverable.
    #[test]
    fn an_uncommitted_item_is_dirty_under_git() -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let work = tempdir("git-untracked")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(root.join("meta/work"))?;
        env.git(&["init", "--quiet"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
        env.git(&["add", "meta"], &root)?;
        env.git(&["commit", "--quiet", "-m", "init"], &root)?;
        fs::write(root.join("meta/work/0002-new.md"), "new\n")?;

        let status = VcsWorkingCopyStatus::probed_from(&root);

        assert_eq!(
            status.is_dirty(&root.join("meta/work/0002-new.md")),
            Dirtiness::Dirty
        );
        Ok(())
    }

    #[test]
    fn a_path_outside_the_repository_answers_unknown() -> Result<(), TestError>
    {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let work = tempdir("git-outside")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(&root)?;
        env.git(&["init", "--quiet"], &root)?;
        fs::write(work.path().join("elsewhere.md"), "x\n")?;

        let status = VcsWorkingCopyStatus::probed_from(&root);

        assert_eq!(
            status.is_dirty(&work.path().join("elsewhere.md")),
            Dirtiness::Unknown
        );
        Ok(())
    }

    #[test]
    fn jj_reports_an_uncommitted_file_as_dirty() -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempdir("jj-dirty")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(root.join("meta/work"))?;
        env.jj(&["git", "init", "--no-colocate"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
        env.jj(&["commit", "-m", "init"], &root)?;
        fs::write(root.join("meta/work/0002-b.md"), "two\n")?;

        assert_eq!(
            dirtiness(&env, &root, &root.join("meta/work/0002-b.md"))?,
            "dirty"
        );
        assert_eq!(
            dirtiness(&env, &root, &root.join("meta/work/0001-a.md"))?,
            "clean"
        );
        Ok(())
    }

    /// A jj checkout is colocated by default, so both markers are present;
    /// the probe must read it as jj, where git's index would lag the
    /// working-copy commit and report live edits as clean.
    #[test]
    fn a_colocated_checkout_is_read_through_jj() -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let work = tempdir("colocated")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(root.join("meta/work"))?;
        env.jj(&["git", "init", "--colocate"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
        env.jj(&["commit", "-m", "init"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "two\n")?;

        assert_eq!(
            dirtiness(&env, &root, &root.join("meta/work/0001-a.md"))?,
            "dirty"
        );
        Ok(())
    }

    #[test]
    fn a_path_under_a_repository_discovered_from_a_subdirectory_is_answered(
    ) -> Result<(), TestError> {
        vcs_test_support::hermetic::assert_git_is_recent_enough()?;
        let work = tempdir("git-subdir")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(root.join("meta/work"))?;
        env.git(&["init", "--quiet"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
        env.git(&["add", "meta"], &root)?;
        env.git(&["commit", "--quiet", "-m", "init"], &root)?;
        fs::write(root.join("meta/work/0001-a.md"), "two\n")?;

        let status = VcsWorkingCopyStatus::probed_from(&root.join("meta/work"));

        assert_eq!(
            status.is_dirty(&root.join("meta/work/0001-a.md")),
            Dirtiness::Dirty
        );
        Ok(())
    }

    const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];

    struct ExcludesRepo {
        work: tempfile::TempDir,
        env: Hermetic,
        root: std::path::PathBuf,
        colocation: &'static str,
    }

    impl ExcludesRepo {
        fn new(colocation: &'static str) -> Result<Self, TestError> {
            vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
            let work = tempdir("jj-excludes")?;
            let env = Hermetic::rooted_at(work.path())?;
            let root = work.path().join("repo");
            fs::create_dir_all(root.join("meta/work"))?;
            env.jj(&["git", "init", colocation], &root)?;
            fs::write(root.join("meta/work/0001-a.md"), "one\n")?;
            env.jj(&["commit", "-m", "init"], &root)?;
            Ok(Self {
                work,
                env,
                root,
                colocation,
            })
        }

        fn backing_exclude(&self) -> std::path::PathBuf {
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

        fn dirtiness(
            &self,
            env: &Hermetic,
            relative: &str,
        ) -> Result<String, TestError> {
            dirtiness(env, &self.root, &self.root.join(relative))
        }
    }

    fn write(path: &std::path::Path, content: &str) -> Result<(), TestError> {
        fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn assert_hidden_beside_a_shown_item(
        repo: &ExcludesRepo,
        env: &Hermetic,
    ) -> Result<(), TestError> {
        write(&repo.root.join("meta/work/0002-hidden.md"), "x\n")?;
        write(&repo.root.join("meta/work/0003-shown.md"), "x\n")?;
        assert_eq!(
            repo.dirtiness(env, "meta/work/0002-hidden.md")?,
            "clean",
            "{}",
            repo.colocation
        );
        assert_eq!(
            repo.dirtiness(env, "meta/work/0003-shown.md")?,
            "dirty",
            "{}",
            repo.colocation
        );
        Ok(())
    }

    #[test]
    fn a_work_item_the_global_excludes_file_hides_is_clean(
    ) -> Result<(), TestError> {
        for colocation in COLOCATIONS {
            let repo = ExcludesRepo::new(colocation)?;
            let env = repo.with_global_excludes("0002-hidden.md\n")?;
            assert_hidden_beside_a_shown_item(&repo, &env)?;
        }
        Ok(())
    }

    #[test]
    fn a_work_item_the_backing_info_exclude_hides_is_clean(
    ) -> Result<(), TestError> {
        for colocation in COLOCATIONS {
            let repo = ExcludesRepo::new(colocation)?;
            write(&repo.backing_exclude(), "0002-hidden.md\n")?;
            assert_hidden_beside_a_shown_item(&repo, &repo.env)?;
        }
        Ok(())
    }

    #[test]
    fn a_work_item_the_git_excludes_do_not_hide_is_dirty(
    ) -> Result<(), TestError> {
        for colocation in COLOCATIONS {
            let repo = ExcludesRepo::new(colocation)?;
            let env = repo.with_global_excludes("unrelated.md\n")?;
            let xdg = repo.env.jj_user_config_dir()?;
            write(
                &xdg.with_file_name("git").join("ignore"),
                "0002-hidden.md\n",
            )?;
            write(&repo.root.join("meta/work/0002-hidden.md"), "x\n")?;
            write(&repo.backing_exclude(), "0001-a.md\n")?;
            write(&repo.root.join("meta/work/0001-a.md"), "edited\n")?;

            assert_eq!(
                repo.dirtiness(&env, "meta/work/0002-hidden.md")?,
                "dirty"
            );
            assert_eq!(repo.dirtiness(&env, "meta/work/0001-a.md")?, "dirty");
        }
        Ok(())
    }

    fn with_limit(
        repo: &ExcludesRepo,
        level: &str,
        value: &str,
    ) -> Result<Hermetic, TestError> {
        if level == "--user" {
            let config = repo.work.path().join("user.toml");
            fs::write(
                &config,
                format!(
                    "[user]\nname = \"Fixture\"\nemail = \"fixture@example.com\"\n\
                     [snapshot]\nmax-new-file-size = {value}\n"
                ),
            )?;
            return Ok(repo.env.clone().with_jj_config(config));
        }
        repo.env.jj(
            &["config", "set", level, "snapshot.max-new-file-size", value],
            &repo.root,
        )?;
        Ok(repo.env.clone())
    }

    #[test]
    fn a_new_work_item_over_the_limit_is_clean() -> Result<(), TestError> {
        for level in ["--repo", "--workspace", "--user"] {
            let repo = ExcludesRepo::new("--no-colocate")?;
            let env = with_limit(&repo, level, "\"1KiB\"")?;
            fs::write(
                repo.root.join("meta/work/0002-two.md"),
                vec![b'x'; 2048],
            )?;
            fs::write(
                repo.root.join("meta/work/0003-one.md"),
                vec![b'x'; 1024],
            )?;

            assert_eq!(
                repo.dirtiness(&env, "meta/work/0002-two.md")?,
                "clean",
                "{level}"
            );
            assert_eq!(
                repo.dirtiness(&env, "meta/work/0003-one.md")?,
                "dirty",
                "{level}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_new_work_item_under_a_raised_or_absent_limit_is_dirty(
    ) -> Result<(), TestError> {
        for value in ["\"1MiB\"", "0"] {
            let repo = ExcludesRepo::new("--no-colocate")?;
            let env = with_limit(&repo, "--repo", value)?;
            fs::write(
                repo.root.join("meta/work/0002-two.md"),
                vec![b'x'; 2048],
            )?;

            assert_eq!(
                repo.dirtiness(&env, "meta/work/0002-two.md")?,
                "dirty",
                "{value}"
            );
        }
        Ok(())
    }

    #[test]
    fn an_invalid_limit_is_warned_about_and_every_item_is_dirty(
    ) -> Result<(), TestError> {
        let repo = ExcludesRepo::new("--no-colocate")?;
        let env = with_limit(&repo, "--repo", "\"abc\"")?;
        fs::write(repo.root.join("meta/work/0001-a.md"), "edited\n")?;
        fs::write(
            repo.root.join("meta/work/0002-large.md"),
            vec![b'x'; 2 * 1024 * 1024],
        )?;

        let mut command = std::process::Command::new(FIXTURE);
        command
            .arg(&repo.root)
            .arg(repo.root.join("meta/work/0001-a.md"))
            .arg(repo.root.join("meta/work/0002-large.md"));
        env.apply(&mut command);
        let output = command.env("ACCELERATOR_LOG", "warn").output()?;

        let stdout = String::from_utf8(output.stdout)?;
        let dirtiness: Vec<&str> = stdout
            .lines()
            .filter_map(|line| line.rsplit_once('\t').map(|(_, state)| state))
            .collect();
        assert_eq!(dirtiness, ["dirty", "dirty"], "{stdout}");
        let stderr = String::from_utf8(output.stderr)?;
        assert!(stderr.contains("WARN"), "{stderr}");
        assert!(stderr.contains("snapshot.max-new-file-size"), "{stderr}");
        Ok(())
    }
}
