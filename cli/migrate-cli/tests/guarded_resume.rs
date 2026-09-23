//! The guarded resume of a stalled run, driven end to end against the
//! compiled binary over real git and jj repositories.
#![cfg(feature = "bash-parity")]

mod common;

use std::fs;

use common::decisions_file_for;
use common::run;
use common::seed_ambiguous_reference;
use common::tempdir;
use common::TestError;
use common::MIGRATION_0007;
use vcs_test_support::hermetic::Hermetic;

#[test]
fn a_git_stall_resumes_after_writing_its_named_decisions_file(
) -> Result<(), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir("git-stall")?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    seed_ambiguous_reference(&root)?;
    env.git(&["add", "-A"], &root)?;
    env.git(&["commit", "--quiet", "-m", "init"], &root)?;

    let stalled = run(&env, &root, &[], &[])?;
    assert_eq!(stalled.code, 1, "{}", stalled.stderr);
    assert!(
        stalled.stderr.contains("MIGRATION STALLED"),
        "{}",
        stalled.stderr
    );

    let decisions = decisions_file_for(MIGRATION_0007);
    fs::write(root.join(&decisions), "accept\n")?;
    env.git(&["status"], &root)?;

    let resumed = run(&env, &root, &["--decisions-file", &decisions], &[])?;

    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    assert!(
        resumed.stderr.starts_with(
            "Resuming over this run's own partial migration output:"
        ),
        "{}",
        resumed.stderr
    );
    assert!(
        resumed.stderr.contains(&format!("  {decisions}\n")),
        "{}",
        resumed.stderr
    );
    assert!(!resumed.stderr.contains("replays"), "{}", resumed.stderr);
    assert!(common::applied_ledger(&root)?.contains(MIGRATION_0007));
    Ok(())
}

fn stalled_on_a_corrupt_jj_operation_store(
    tag: &str,
) -> Result<(tempfile::TempDir, Hermetic, std::path::PathBuf), TestError> {
    vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
    let work = tempdir(tag)?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.jj(&["git", "init", "--no-colocate"], &root)?;
    seed_ambiguous_reference(&root)?;
    env.jj(&["commit", "-m", "init"], &root)?;
    let heads = root.join(".jj/repo/op_heads/heads");
    fs::remove_dir_all(&heads)?;
    fs::create_dir_all(&heads)?;
    Ok((work, env, root))
}

#[test]
fn an_unreadable_jj_working_copy_is_warned_about_and_treated_as_clean(
) -> Result<(), TestError> {
    let (_work, env, root) =
        stalled_on_a_corrupt_jj_operation_store("jj-corrupt")?;

    let outcome = run(&env, &root, &[], &[])?;

    assert!(outcome.stderr.contains("WARN"), "{}", outcome.stderr);
    assert!(
        outcome
            .stderr
            .contains("could not compute the working-copy diff"),
        "{}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("dirty working tree"),
        "{}",
        outcome.stderr
    );
    assert!(
        outcome.stderr.contains("MIGRATION STALLED"),
        "{}",
        outcome.stderr
    );
    Ok(())
}

#[test]
fn force_over_an_unreadable_jj_working_copy_records_no_run_base(
) -> Result<(), TestError> {
    let (_work, env, root) =
        stalled_on_a_corrupt_jj_operation_store("jj-corrupt-force")?;

    let outcome = run(&env, &root, &[], &[("ACCELERATOR_MIGRATE_FORCE", "1")])?;

    assert!(outcome.stderr.contains("WARN"), "{}", outcome.stderr);
    assert!(
        outcome.stderr.contains("MIGRATION STALLED"),
        "{}",
        outcome.stderr
    );
    assert_eq!(fs::read_to_string(root.join(common::RUN_BASE_FILE))?, "\n");
    Ok(())
}

const COLOCATIONS: [&str; 2] = ["--no-colocate", "--colocate"];

enum Base {
    SingleParent,
    MergeWithSibling,
}

/// A jj repository whose 0007 run has stalled, with the committed corpus as
/// `base` and a sibling change beside it that adds `meta/sibling.md`.
struct Stalled {
    _work: tempfile::TempDir,
    env: Hermetic,
    root: std::path::PathBuf,
    decisions: String,
    base: String,
    sibling: String,
}

impl Stalled {
    fn on(colocation: &str, shape: &Base) -> Result<Self, TestError> {
        vcs_test_support::hermetic::assert_jj_matches("0.43.0")?;
        let work = tempdir("jj-stall")?;
        let env = Hermetic::rooted_at(work.path())?;
        let root = work.path().join("repo");
        fs::create_dir_all(&root)?;
        env.jj(&["git", "init", colocation], &root)?;
        common::seed_output_then_ambiguous_reference(&root)?;
        env.jj(&["commit", "-m", "corpus"], &root)?;
        let base = change_id(&env, &root, "@-")?;
        env.jj(&["describe", "-m", "sibling"], &root)?;
        common::write(&root, "meta/sibling.md", "sibling\n")?;
        let sibling = change_id(&env, &root, "@")?;
        match shape {
            Base::SingleParent => env.jj(&["new", &base], &root)?,
            Base::MergeWithSibling => {
                env.jj(&["new", &base, &sibling], &root)?
            }
        };

        let stalled = run(&env, &root, &[], &[])?;
        assert_eq!(stalled.code, 1, "{}", stalled.stderr);
        assert!(
            stalled.stderr.contains("MIGRATION STALLED"),
            "{}",
            stalled.stderr
        );

        Ok(Self {
            _work: work,
            env,
            root,
            decisions: decisions_file_for(MIGRATION_0007),
            base,
            sibling,
        })
    }

    fn jj(&self, args: &[&str]) -> Result<String, TestError> {
        Ok(self.env.jj(args, &self.root)?)
    }

    fn write_decisions(&self) -> Result<(), TestError> {
        fs::write(self.root.join(&self.decisions), "accept\n")?;
        Ok(())
    }

    fn resume(&self) -> Result<common::Outcome, TestError> {
        run(
            &self.env,
            &self.root,
            &["--decisions-file", &self.decisions],
            &[],
        )
    }

    fn recorded_run_base(&self) -> Result<String, TestError> {
        Ok(fs::read_to_string(self.root.join(common::RUN_BASE_FILE))?
            .trim()
            .to_owned())
    }

    fn oracle(&self) -> Result<String, TestError> {
        Ok(vcs_test_support::jj_status::run_base_oracle(
            &self.env, &self.root,
        )?)
    }
}

fn change_id(
    env: &Hermetic,
    root: &std::path::Path,
    revision: &str,
) -> Result<String, TestError> {
    Ok(env.jj(
        &["log", "--no-graph", "-r", revision, "-T", "change_id"],
        root,
    )?)
}

fn assert_resumed(outcome: &common::Outcome) {
    assert!(
        outcome.stderr.starts_with(
            "Resuming over this run's own partial migration output:"
        ),
        "{}",
        outcome.stderr
    );
}

fn assert_refused(outcome: &common::Outcome) {
    assert_eq!(outcome.code, 1, "{}", outcome.stderr);
    assert!(
        outcome.stderr.starts_with("Error: dirty working tree"),
        "{}",
        outcome.stderr
    );
}

const STALE_LINE: &str =
    "A previous migration run's recorded base no longer matches";

#[test]
fn a_jj_stall_resumes_across_jj_status() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        assert_eq!(stalled.recorded_run_base()?, stalled.oracle()?);
        stalled.write_decisions()?;
        stalled.jj(&["status"])?;

        let resumed = stalled.resume()?;

        assert_eq!(resumed.code, 0, "{colocation}: {}", resumed.stderr);
        assert_resumed(&resumed);
        assert!(
            resumed.stderr.contains(common::OWNED_NOTE),
            "{}",
            resumed.stderr
        );
        assert!(common::applied_ledger(&stalled.root)?.contains(MIGRATION_0007));
    }
    Ok(())
}

#[test]
fn a_jj_stall_resumes_across_owned_edits_and_describing_the_working_copy(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        stalled.write_decisions()?;
        fs::write(
            stalled.root.join(common::OWNED_NOTE),
            fs::read_to_string(stalled.root.join(common::OWNED_NOTE))?
                + "\nedited\n",
        )?;
        stalled.jj(&["status"])?;
        stalled.jj(&["describe", "-m", "x"])?;

        let resumed = stalled.resume()?;

        assert_eq!(resumed.code, 0, "{colocation}: {}", resumed.stderr);
        assert_resumed(&resumed);
    }
    Ok(())
}

#[test]
fn a_jj_refusal_lists_only_the_current_runs_unowned_changes(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        common::write(&stalled.root, "meta/a.md", "a\n")?;
        common::write(&stalled.root, ".accelerator/b", "b\n")?;

        let refused = run(&stalled.env, &stalled.root, &[], &[])?;

        assert_refused(&refused);
        assert!(
            refused.stderr.ends_with(
                "Unowned changes (2):\n  .accelerator/b\n  meta/a.md\n"
            ),
            "{colocation}: {}",
            refused.stderr
        );
        assert!(
            !refused.stderr.contains(common::OWNED_NOTE),
            "{}",
            refused.stderr
        );
        assert!(!refused.stderr.contains(STALE_LINE), "{}", refused.stderr);
    }
    Ok(())
}

#[test]
fn rewriting_or_rebasing_the_parent_makes_a_jj_stall_stale(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        for rewrite in [
            &["describe", "@-", "-m", "y"][..],
            &["rebase", "-r", "@", "-d", "SIBLING"][..],
        ] {
            let stalled = Stalled::on(colocation, &Base::SingleParent)?;
            let args: Vec<&str> = rewrite
                .iter()
                .map(|arg| {
                    if *arg == "SIBLING" {
                        stalled.sibling.as_str()
                    } else {
                        arg
                    }
                })
                .collect();
            stalled.jj(&args)?;
            stalled.write_decisions()?;

            let refused = stalled.resume()?;

            assert_refused(&refused);
            assert!(
                refused.stderr.contains(STALE_LINE),
                "{args:?}: {}",
                refused.stderr
            );
            assert!(
                refused
                    .stderr
                    .contains(&format!("  {}\n", common::OWNED_NOTE)),
                "{args:?}: {}",
                refused.stderr
            );
        }
    }
    Ok(())
}

#[test]
fn moving_onto_new_parents_starts_a_fresh_jj_run() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        stalled.jj(&["new"])?;

        let fresh = run(&stalled.env, &stalled.root, &[], &[])?;

        assert!(!fresh.stderr.contains("Resuming"), "{}", fresh.stderr);
        assert!(
            fresh.stderr.contains("MIGRATION STALLED"),
            "{}",
            fresh.stderr
        );
        assert_eq!(stalled.recorded_run_base()?, stalled.oracle()?);
    }
    Ok(())
}

#[test]
fn editing_a_sibling_lists_its_changes_as_unowned() -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        stalled.jj(&["edit", &stalled.sibling])?;

        let refused = run(&stalled.env, &stalled.root, &[], &[])?;

        assert_refused(&refused);
        assert!(
            refused
                .stderr
                .ends_with("Unowned changes (1):\n  meta/sibling.md\n"),
            "{colocation}: {}",
            refused.stderr
        );
    }
    Ok(())
}

/// The run's manifest and run base live in the working-copy change, so a new
/// change on the same parent carries neither, and nothing it holds is owned.
#[test]
fn a_manifested_path_edited_on_a_new_change_with_the_same_parent_is_unowned(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        stalled.jj(&["new", &stalled.base])?;
        common::write(
            &stalled.root,
            common::OWNED_NOTE,
            "rewritten by hand\n",
        )?;

        let refused = run(&stalled.env, &stalled.root, &[], &[])?;

        assert_refused(&refused);
        assert!(
            refused.stderr.ends_with(&format!(
                "Unowned changes (1):\n  {}\n",
                common::OWNED_NOTE
            )),
            "{colocation}: {}",
            refused.stderr
        );
    }
    Ok(())
}

#[test]
fn a_stall_on_a_merge_resumes_until_one_parent_is_rewritten(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::MergeWithSibling)?;
        assert_eq!(stalled.recorded_run_base()?, stalled.oracle()?);
        assert!(stalled.recorded_run_base()?.contains('+'));
        stalled.write_decisions()?;
        stalled.jj(&["status"])?;

        let resumed = run(&stalled.env, &stalled.root, &[], &[])?;
        assert_resumed(&resumed);

        stalled.jj(&["describe", &stalled.sibling, "-m", "sibling two"])?;
        let refused = stalled.resume()?;
        assert_refused(&refused);
        assert!(refused.stderr.contains(STALE_LINE), "{}", refused.stderr);
    }
    Ok(())
}

#[test]
fn a_clean_jj_run_records_the_parents_of_the_working_copy(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        stalled.jj(&["new", "-m", "one"])?;
        let one = change_id(&stalled.env, &stalled.root, "@")?;
        common::write(&stalled.root, "one.txt", "1\n")?;
        stalled.jj(&["new", &stalled.base, "-m", "two"])?;
        let two = change_id(&stalled.env, &stalled.root, "@")?;
        common::write(&stalled.root, "two.txt", "2\n")?;

        for parents in [
            vec![one.as_str(), two.as_str()],
            vec![one.as_str(), two.as_str(), stalled.sibling.as_str()],
        ] {
            let mut args = vec!["new"];
            args.extend(parents.iter());
            stalled.jj(&args)?;
            let outcome = run(&stalled.env, &stalled.root, &[], &[])?;

            assert!(
                outcome.stderr.contains("MIGRATION STALLED"),
                "{}",
                outcome.stderr
            );
            assert_eq!(stalled.recorded_run_base()?, stalled.oracle()?);
            assert_eq!(
                stalled.recorded_run_base()?.matches('+').count(),
                parents.len() - 1
            );
        }
    }
    Ok(())
}

fn files_under(
    root: &std::path::Path,
    relative: &str,
) -> Result<Vec<(String, String)>, TestError> {
    let mut found = Vec::new();
    let mut pending = vec![root.join(relative)];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                found.push((
                    path.strip_prefix(root)?.display().to_string(),
                    fs::read_to_string(&path)?,
                ));
            }
        }
    }
    found.sort();
    Ok(found)
}

#[test]
fn a_stall_recorded_before_the_upgrade_is_refused_until_its_output_is_committed(
) -> Result<(), TestError> {
    for colocation in COLOCATIONS {
        let stalled = Stalled::on(colocation, &Base::SingleParent)?;
        let working_copy_commit = stalled.jj(&[
            "log",
            "--ignore-working-copy",
            "--no-graph",
            "-r",
            "@",
            "-T",
            "commit_id",
        ])?;
        fs::write(
            stalled.root.join(common::RUN_BASE_FILE),
            format!("{working_copy_commit}\n"),
        )?;
        stalled.write_decisions()?;
        let meta_before = files_under(&stalled.root, "meta")?;
        let state_before = files_under(&stalled.root, ".accelerator")?;

        for attempt in ["first", "second"] {
            let refused = stalled.resume()?;
            assert_refused(&refused);
            assert!(!refused.stderr.contains("panicked"), "{}", refused.stderr);
            assert!(
                refused.stderr.contains(STALE_LINE),
                "{attempt}: {}",
                refused.stderr
            );
            assert!(
                refused.stderr.contains(
                    "commit them and re-run without ACCELERATOR_MIGRATE_FORCE"
                ),
                "{}",
                refused.stderr
            );
            assert!(
                refused.stderr.contains(common::OWNED_NOTE),
                "{}",
                refused.stderr
            );
            assert_eq!(files_under(&stalled.root, "meta")?, meta_before);
            assert_eq!(
                files_under(&stalled.root, ".accelerator")?,
                state_before
            );
        }

        common::write(&stalled.root, "src/unrelated.txt", "unrelated\n")?;
        stalled.jj(&[
            "commit",
            "-m",
            "partial migration",
            "meta",
            ".accelerator",
            ".claude",
        ])?;
        assert!(stalled
            .jj(&["diff", "--summary"])?
            .contains("src/unrelated.txt"));

        let resumed = stalled.resume()?;

        assert_eq!(resumed.code, 0, "{colocation}: {}", resumed.stderr);
        assert!(common::applied_ledger(&stalled.root)?.contains(MIGRATION_0007));
        assert!(
            !resumed.stderr.contains(
                "[0006-canonicalise-work-item-id-and-author] running"
            ),
            "{}",
            resumed.stderr
        );
        assert_ne!(
            fs::read_to_string(stalled.root.join("meta/work/0001-source.md"))?,
            common::work_item(
                "0001",
                "Source",
                "\n## References\n- `meta/work/0042-target.md`\n"
            ),
        );
    }
    Ok(())
}
