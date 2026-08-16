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

        let status = VcsWorkingCopyStatus::probed_from(&root);

        assert_eq!(
            status.is_dirty(&root.join("meta/work/0002-b.md")),
            Dirtiness::Dirty
        );
        assert_eq!(
            status.is_dirty(&root.join("meta/work/0001-a.md")),
            Dirtiness::Clean
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

        let status = VcsWorkingCopyStatus::probed_from(&root);

        assert_eq!(
            status.is_dirty(&root.join("meta/work/0001-a.md")),
            Dirtiness::Dirty
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
}
