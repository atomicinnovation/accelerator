//! Two builds of one script project equal states, and a state differs when
//! any one thing the projection covers differs.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use vcs_test_support::colocation::JjColocation;
use vcs_test_support::hermetic::Hermetic;
use vcs_test_support::repository_state::RepositoryKind;
use vcs_test_support::repository_state::RepositoryState;
use vcs_test_support::workbench::Workbench;

type TestError = Box<dyn std::error::Error>;

static BUILDS: AtomicU64 = AtomicU64::new(0);

/// One run of a script in a workbench of its own, committing at a time no
/// other build in this binary uses, so two builds' git commits differ in ID.
struct Build {
    _workbench: Workbench,
    hermetic: Hermetic,
    root: PathBuf,
}

impl Build {
    fn of(
        script: impl FnOnce(&Path, &Hermetic) -> Result<(), TestError>,
    ) -> Result<Self, TestError> {
        let workbench = Workbench::for_this_test()?;
        let date = format!(
            "{} +0000",
            1_700_000_000 + BUILDS.fetch_add(1, Ordering::Relaxed)
        );
        let hermetic = workbench
            .hermetic()
            .clone()
            .with_variable("GIT_AUTHOR_DATE", &date)
            .with_variable("GIT_COMMITTER_DATE", &date);
        let root = workbench.path().join("repository");
        fs::create_dir(&root)?;
        script(&root, &hermetic)?;
        Ok(Self {
            _workbench: workbench,
            hermetic,
            root,
        })
    }

    fn then(
        self,
        step: impl FnOnce(&Path, &Hermetic) -> Result<(), TestError>,
    ) -> Result<Self, TestError> {
        step(&self.root, &self.hermetic)?;
        Ok(self)
    }

    fn state(&self) -> Result<RepositoryState, TestError> {
        Ok(RepositoryState::of(&self.root, &self.hermetic)?)
    }

    fn git(&self, args: &[&str]) -> Result<String, TestError> {
        Ok(self.hermetic.git(args, &self.root)?)
    }

    fn jj(&self, args: &[&str]) -> Result<String, TestError> {
        Ok(self.hermetic.jj(args, &self.root)?)
    }

    fn jj_commit_id(&self, revision: &str) -> Result<String, TestError> {
        self.jj(&[
            "--ignore-working-copy",
            "log",
            "--no-graph",
            "-r",
            revision,
            "-T",
            "commit_id",
        ])
    }
}

fn write(root: &Path, relative: &str, contents: &str) -> Result<(), TestError> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn git_history(root: &Path, env: &Hermetic) -> Result<(), TestError> {
    env.git(&["init", "-q"], root)?;
    write(root, "a.txt", "one\n")?;
    env.git(&["add", "-A"], root)?;
    env.git(&["commit", "-q", "-m", "first"], root)?;
    write(root, "nested/b.txt", "two\n")?;
    env.git(&["add", "-A"], root)?;
    env.git(&["commit", "-q", "-m", "second"], root)?;
    env.git(&["commit", "-q", "--allow-empty", "-m", "third"], root)?;
    env.git(&["tag", "v1", "HEAD~2"], root)?;
    env.git(&["branch", "extra", "HEAD"], root)?;
    Ok(())
}

fn jj_history(
    colocation: JjColocation,
) -> impl FnOnce(&Path, &Hermetic) -> Result<(), TestError> {
    move |root, env| {
        env.jj(&["git", "init", colocation.flag()], root)?;
        write(root, "a.txt", "one\n")?;
        env.jj(&["commit", "-m", "first"], root)?;
        env.jj(&["commit", "-m", "second"], root)?;
        write(root, "nested/b.txt", "two\n")?;
        env.jj(&["commit", "-m", "third"], root)?;
        env.jj(&["bookmark", "create", "main", "-r", "@-"], root)?;
        Ok(())
    }
}

fn assert_rebuilds_equal(
    script: impl Fn(&Path, &Hermetic) -> Result<(), TestError>,
    commit_id: impl Fn(&Build) -> Result<String, TestError>,
) -> Result<(), TestError> {
    let first = Build::of(&script)?;
    let second = Build::of(&script)?;

    assert_ne!(commit_id(&first)?, commit_id(&second)?);
    assert_eq!(first.state()?, second.state()?);
    Ok(())
}

#[test]
fn two_git_builds_of_one_script_project_equal_states() -> Result<(), TestError>
{
    assert_rebuilds_equal(git_history, |build| {
        build.git(&["rev-parse", "HEAD"])
    })
}

#[test]
fn two_pure_jj_builds_of_one_script_project_equal_states(
) -> Result<(), TestError> {
    assert_rebuilds_equal(
        |root, env| jj_history(JjColocation::Pure)(root, env),
        |build| build.jj_commit_id("@-"),
    )
}

#[test]
fn two_colocated_jj_builds_of_one_script_project_equal_states(
) -> Result<(), TestError> {
    assert_rebuilds_equal(
        |root, env| jj_history(JjColocation::Colocated)(root, env),
        |build| build.jj_commit_id("@-"),
    )
}

#[test]
fn the_projection_holds_each_commit_s_files_description_and_parents(
) -> Result<(), TestError> {
    let state = Build::of(jj_history(JjColocation::Pure))?.state()?;

    let descriptions: Vec<&str> = state
        .commits
        .iter()
        .map(|commit| commit.description.as_str())
        .collect();
    assert_eq!(descriptions, ["first\n", "second\n", "third\n", ""]);
    let third = &state.commits[2];
    assert_eq!(third.parents, [state.commits[1].position]);
    assert_eq!(
        third.files.get("nested/b.txt").map(String::as_str),
        Some("two\n")
    );
    assert_eq!(state.references.get("main"), Some(&third.position));
    assert_eq!(state.working_copy_parents, [third.position]);
    assert_eq!(state.kind, RepositoryKind::Jj(JjColocation::Pure));
    Ok(())
}

#[test]
fn a_different_description_differs() -> Result<(), TestError> {
    let base = Build::of(jj_history(JjColocation::Pure))?.state()?;
    let varied = Build::of(jj_history(JjColocation::Pure))?
        .then(|root, env| {
            env.jj(&["describe", "-r", "@-", "-m", "other"], root)?;
            Ok(())
        })?
        .state()?;

    assert_ne!(base.commits, varied.commits);
    assert_eq!(base.working_copy, varied.working_copy);
    Ok(())
}

#[test]
fn a_different_file_content_differs() -> Result<(), TestError> {
    let base = Build::of(git_history)?.state()?;
    let varied = Build::of(git_history)?
        .then(|root, env| {
            write(root, "a.txt", "changed\n")?;
            env.git(&["commit", "-q", "-a", "--amend", "--no-edit"], root)?;
            Ok(())
        })?
        .state()?;

    assert_ne!(base.commits, varied.commits);
    Ok(())
}

#[test]
fn different_parents_differ() -> Result<(), TestError> {
    let base = Build::of(jj_history(JjColocation::Pure))?.state()?;
    let varied = Build::of(jj_history(JjColocation::Pure))?
        .then(|root, env| {
            env.jj(
                &[
                    "rebase",
                    "-s",
                    "description(exact:\"second\n\")",
                    "-d",
                    "root()",
                ],
                root,
            )?;
            Ok(())
        })?
        .state()?;

    assert_ne!(base.commits, varied.commits);
    assert_eq!(base.references, varied.references);
    Ok(())
}

#[test]
fn a_different_git_reference_differs() -> Result<(), TestError> {
    let base = Build::of(git_history)?.state()?;
    let varied = Build::of(git_history)?
        .then(|root, env| {
            env.git(&["branch", "-f", "extra", "HEAD~1"], root)?;
            Ok(())
        })?
        .state()?;

    assert_ne!(base.references, varied.references);
    assert_eq!(base.commits, varied.commits);
    Ok(())
}

#[test]
fn a_different_head_differs() -> Result<(), TestError> {
    let base = Build::of(git_history)?.state()?;
    let varied = Build::of(git_history)?
        .then(|root, env| {
            env.git(&["checkout", "-q", "--detach", "HEAD~1"], root)?;
            Ok(())
        })?
        .state()?;

    assert_ne!(base.references, varied.references);
    assert_ne!(base.working_copy_parents, varied.working_copy_parents);
    assert_eq!(base.working_copy, varied.working_copy);
    Ok(())
}

#[test]
fn a_different_bookmark_differs() -> Result<(), TestError> {
    let base = Build::of(jj_history(JjColocation::Pure))?.state()?;
    let varied = Build::of(jj_history(JjColocation::Pure))?
        .then(|root, env| {
            env.jj(
                &["bookmark", "set", "main", "-r", "@--", "--allow-backwards"],
                root,
            )?;
            Ok(())
        })?
        .state()?;

    assert_ne!(base.references, varied.references);
    assert_eq!(base.commits, varied.commits);
    Ok(())
}

#[test]
fn a_different_working_copy_parent_differs() -> Result<(), TestError> {
    let base = Build::of(jj_history(JjColocation::Pure))?.state()?;
    let varied = Build::of(jj_history(JjColocation::Pure))?
        .then(|root, env| {
            env.jj(&["new", "@--"], root)?;
            Ok(())
        })?
        .state()?;

    assert_ne!(base.working_copy_parents, varied.working_copy_parents);
    Ok(())
}

#[test]
fn a_different_colocation_differs() -> Result<(), TestError> {
    let pure = Build::of(jj_history(JjColocation::Pure))?.state()?;
    let colocated = Build::of(jj_history(JjColocation::Colocated))?.state()?;

    assert_eq!(pure.kind, RepositoryKind::Jj(JjColocation::Pure));
    assert_eq!(colocated.kind, RepositoryKind::Jj(JjColocation::Colocated));
    assert_ne!(pure, colocated);
    Ok(())
}

const RUN_ID: &str = ".accelerator/state/migrations-run.id";

fn recording_run_ids(
    revisions: &'static [&'static str],
) -> impl Fn(&Path, &Hermetic) -> Result<(), TestError> {
    move |root, env| {
        jj_history(JjColocation::Pure)(root, env)?;
        let ids = revisions
            .iter()
            .map(|revision| {
                Ok(env.jj(
                    &[
                        "--ignore-working-copy",
                        "log",
                        "--no-graph",
                        "-r",
                        revision,
                        "-T",
                        "commit_id",
                    ],
                    root,
                )?)
            })
            .collect::<Result<Vec<_>, TestError>>()?;
        write(root, RUN_ID, &format!("{}\n", ids.join("+")))
    }
}

#[test]
fn a_file_naming_a_commit_compares_by_its_position() -> Result<(), TestError> {
    let first = Build::of(recording_run_ids(&["@-"]))?.state()?;
    let second = Build::of(recording_run_ids(&["@-"]))?.state()?;
    let elsewhere = Build::of(recording_run_ids(&["@--"]))?.state()?;

    assert_eq!(first, second);
    assert_eq!(
        first.working_copy.get(RUN_ID).map(String::as_str),
        Some("<commit 2>\n")
    );
    assert_ne!(first, elsewhere);
    Ok(())
}

#[test]
fn a_file_naming_joined_commits_compares_by_their_positions_in_any_order(
) -> Result<(), TestError> {
    let first = Build::of(recording_run_ids(&["@-", "@--"]))?.state()?;
    let swapped = Build::of(recording_run_ids(&["@--", "@-"]))?.state()?;
    let elsewhere = Build::of(recording_run_ids(&["@-", "@---"]))?.state()?;

    assert_eq!(first, swapped);
    assert_ne!(first, elsewhere);
    Ok(())
}

#[test]
fn a_jj_keep_reference_does_not_break_equality() -> Result<(), TestError> {
    let base = Build::of(git_history)?;
    let kept = Build::of(git_history)?;
    let head = kept.git(&["rev-parse", "HEAD~1"])?;
    kept.git(&["update-ref", &format!("refs/jj/keep/{head}"), &head])?;

    assert_eq!(base.state()?, kept.state()?);
    Ok(())
}

fn siblings_described_alike(
    bookmarked: Option<&'static str>,
) -> impl Fn(&Path, &Hermetic) -> Result<(), TestError> {
    move |root, env| {
        env.jj(&["git", "init", "--no-colocate"], root)?;
        env.jj(&["commit", "-m", "base"], root)?;
        env.jj(&["new", "--no-edit", "-m", "s1", "@-"], root)?;
        env.jj(&["new", "--no-edit", "-m", "s2", "@-"], root)?;
        if let Some(sibling) = bookmarked {
            let revision = format!("description(exact:\"{sibling}\n\")");
            env.jj(&["bookmark", "create", "left", "-r", &revision], root)?;
        }
        env.jj(
            &[
                "describe",
                "-r",
                "description(exact:\"s1\n\")",
                "-r",
                "description(exact:\"s2\n\")",
                "-m",
                "sibling",
            ],
            root,
        )?;
        Ok(())
    }
}

#[test]
fn siblings_told_apart_by_a_bookmark_take_the_same_order_in_both_builds(
) -> Result<(), TestError> {
    let first_bookmarked = Build::of(siblings_described_alike(Some("s1")))?;
    let second_bookmarked = Build::of(siblings_described_alike(Some("s2")))?;

    assert_eq!(first_bookmarked.state()?, second_bookmarked.state()?);
    Ok(())
}

#[test]
fn siblings_nothing_tells_apart_are_refused() -> Result<(), TestError> {
    let build = Build::of(siblings_described_alike(None))?;

    let error = build.state().err().ok_or("it should be refused")?;

    assert!(error.to_string().contains("nothing tells"), "{error}");
    Ok(())
}

#[test]
fn reading_jj_never_snapshots_the_working_copy() -> Result<(), TestError> {
    let build = Build::of(jj_history(JjColocation::Pure))?;
    write(&build.root, "unsnapshotted.txt", "edited\n")?;
    let operations = || {
        build.jj(&[
            "--ignore-working-copy",
            "op",
            "log",
            "--no-graph",
            "-T",
            "id",
        ])
    };
    let before = operations()?;

    let state = build.state()?;

    assert_eq!(operations()?, before);
    assert_eq!(
        state
            .working_copy
            .get("unsnapshotted.txt")
            .map(String::as_str),
        Some("edited\n")
    );
    assert!(state
        .commits
        .iter()
        .all(|commit| !commit.files.contains_key("unsnapshotted.txt")));
    Ok(())
}
