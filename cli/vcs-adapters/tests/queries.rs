//! The six taxonomy queries against every (fixture, start directory) pair,
//! compared against values measured from the live `git`/`jj` CLIs.
//!
//! Values are compared as rendered strings so a failure names the cell rather
//! than dumping a nested `Debug`.
//!
//! One test over the whole table rather than one per pair: nextest is
//! process-per-test with no sharing, so a test per pair would rebuild the
//! ~34-fixture matrix 34 times, each build driving dozens of `git`/`jj`
//! subprocesses. Every mismatch is collected and reported together, so per-cell
//! traceability is kept.
#![cfg(feature = "bash-parity")]

use std::fmt::Write as _;
use std::path::Path;

use vcs_adapters::library::DualRoots;
use vcs_adapters::library::Error;
use vcs_adapters::library::InProcessProbe;
use vcs_adapters::library::JjRepositoryFacts;
use vcs_adapters::library::JjWorkspaceRole;
use vcs_adapters::library::WorktreeFacts;
use vcs_test_support::fixtures::Matrix;
use vcs_test_support::hermetic::assert_git_is_recent_enough;

type TestError = Box<dyn std::error::Error>;

/// The expected rendering of all six queries for one pair. `$B` stands for the
/// matrix base directory.
struct Cells {
    key: &'static str,
    is_bare: &'static str,
    worktree: &'static str,
    superproject: &'static str,
    jj_workspace_root: &'static str,
    jj_repository: &'static str,
    dual_roots: &'static str,
}

const EXPECTED: &[Cells] = &[
    Cells {
        key: "PG-r",
        is_bare: "false",
        worktree: "linked=false git=$B/PG/.git common=$B/PG/.git main=$B/PG",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/PG jj=absent",
    },
    Cells {
        key: "PG-s",
        is_bare: "false",
        worktree: "linked=false git=$B/PG/.git common=$B/PG/.git main=$B/PG",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/PG jj=absent",
    },
    Cells {
        key: "WT-l",
        is_bare: "false",
        worktree: "linked=true git=$B/main/.git/worktrees/WT common=$B/main/.git main=$B/main",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/WT jj=absent",
    },
    Cells {
        key: "WT-m",
        is_bare: "false",
        worktree: "linked=false git=$B/main/.git common=$B/main/.git main=$B/main",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/main jj=absent",
    },
    Cells {
        key: "BARE",
        is_bare: "true",
        worktree: "linked=false git=$B/bare common=$B/bare main=absent",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=absent jj=absent",
    },
    Cells {
        key: "NONE",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=absent jj=absent",
    },
    Cells {
        key: "CR",
        is_bare: "false",
        worktree: "linked=false git=$B/CR/.git common=$B/CR/.git main=$B/CR",
        superproject: "absent",
        jj_workspace_root: "$B/CR",
        jj_repository: "main $B/CR",
        dual_roots: "git=$B/CR jj=$B/CR",
    },
    Cells {
        key: "CG",
        is_bare: "false",
        worktree: "linked=true git=$B/gitparent-cg/.git/worktrees/CG common=$B/gitparent-cg/.git main=$B/gitparent-cg",
        superproject: "absent",
        jj_workspace_root: "$B/CG",
        jj_repository: "secondary $B/jjparent",
        dual_roots: "git=$B/CG jj=$B/CG",
    },
    Cells {
        key: "JS-r",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "$B/JS",
        jj_repository: "secondary $B/jjmain-colocated",
        dual_roots: "git=absent jj=$B/JS",
    },
    Cells {
        key: "JS-s",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "$B/JS",
        jj_repository: "secondary $B/jjmain-colocated",
        dual_roots: "git=absent jj=$B/JS",
    },
    Cells {
        key: "PJ",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "$B/PJ/a/b/c",
        jj_repository: "main $B/PJ/a/b/c",
        dual_roots: "git=absent jj=$B/PJ/a/b/c",
    },
    Cells {
        key: "PJS",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "$B/PJS",
        jj_repository: "secondary $B/PJ/a/b/c",
        dual_roots: "git=absent jj=$B/PJS",
    },
    Cells {
        key: "JS-in",
        is_bare: "false",
        worktree: "linked=false git=$B/JSIN/.git common=$B/JSIN/.git main=$B/JSIN",
        superproject: "absent",
        jj_workspace_root: "$B/JSIN/workspaces/inner",
        jj_repository: "secondary $B/JSIN",
        dual_roots: "git=$B/JSIN jj=$B/JSIN/workspaces/inner",
    },
    Cells {
        key: "NJG-i",
        is_bare: "false",
        worktree: "linked=false git=$B/NJG/.git common=$B/NJG/.git main=$B/NJG",
        superproject: "absent",
        jj_workspace_root: "$B/NJG/sub",
        jj_repository: "secondary $B/jjmain-colocated",
        dual_roots: "git=$B/NJG jj=$B/NJG/sub",
    },
    Cells {
        key: "NJG-o",
        is_bare: "false",
        worktree: "linked=false git=$B/NJG/.git common=$B/NJG/.git main=$B/NJG",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/NJG jj=absent",
    },
    Cells {
        key: "NGJ-i",
        is_bare: "false",
        worktree: "linked=true git=$B/gitparent-ngj/.git/worktrees/sub common=$B/gitparent-ngj/.git main=$B/gitparent-ngj",
        superproject: "absent",
        jj_workspace_root: "$B/NGJ",
        jj_repository: "main $B/NGJ",
        dual_roots: "git=$B/NGJ/sub jj=$B/NGJ",
    },
    Cells {
        key: "NGJ-o",
        is_bare: "false",
        worktree: "linked=false git=$B/NGJ/.git common=$B/NGJ/.git main=$B/NGJ",
        superproject: "absent",
        jj_workspace_root: "$B/NGJ",
        jj_repository: "main $B/NGJ",
        dual_roots: "git=$B/NGJ jj=$B/NGJ",
    },
    Cells {
        key: "NGPJ-i",
        is_bare: "false",
        worktree: "linked=true git=$B/gitparent-ngpj/.git/worktrees/sub common=$B/gitparent-ngpj/.git main=$B/gitparent-ngpj",
        superproject: "absent",
        jj_workspace_root: "$B/NGPJ",
        jj_repository: "main $B/NGPJ",
        dual_roots: "git=$B/NGPJ/sub jj=$B/NGPJ",
    },
    Cells {
        key: "NGPJ-o",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "$B/NGPJ",
        jj_repository: "main $B/NGPJ",
        dual_roots: "git=absent jj=$B/NGPJ",
    },
    Cells {
        key: "PJG-i",
        is_bare: "false",
        worktree: "linked=false git=$B/PJG/.git common=$B/PJG/.git main=$B/PJG",
        superproject: "absent",
        jj_workspace_root: "$B/PJG/sub",
        jj_repository: "secondary $B/jjmain-pure",
        dual_roots: "git=$B/PJG jj=$B/PJG/sub",
    },
    Cells {
        key: "PJG-o",
        is_bare: "false",
        worktree: "linked=false git=$B/PJG/.git common=$B/PJG/.git main=$B/PJG",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/PJG jj=absent",
    },
    Cells {
        key: "SM-1",
        is_bare: "false",
        worktree: "linked=false git=$B/super/.git/modules/mid common=$B/super/.git/modules/mid main=$B/super/mid",
        superproject: "$B/super",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/super/mid jj=absent",
    },
    Cells {
        key: "SM-2",
        is_bare: "false",
        worktree: "linked=false git=$B/super/.git/modules/mid/modules/leaf common=$B/super/.git/modules/mid/modules/leaf main=$B/super/mid/leaf",
        superproject: "$B/super/mid",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/super/mid/leaf jj=absent",
    },
    Cells {
        key: "SM-s",
        is_bare: "false",
        worktree: "linked=false git=$B/super/.git common=$B/super/.git main=$B/super",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/super jj=absent",
    },
    Cells {
        key: "SO",
        is_bare: "false",
        worktree: "linked=false git=$B/old/inner/.git common=$B/old/inner/.git main=$B/old/inner",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/old/inner jj=absent",
    },
    Cells {
        key: "SM-m",
        is_bare: "false",
        worktree: "linked=false git=$B/supm/.git/modules/modules/foo common=$B/supm/.git/modules/modules/foo main=$B/supm/modules/foo",
        superproject: "$B/supm",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/supm/modules/foo jj=absent",
    },
    Cells {
        key: "SM-w",
        is_bare: "false",
        worktree: "linked=true git=$B/supw/.git/modules/mid/worktrees/smw common=$B/supw/.git/modules/mid main=absent",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/smw jj=absent",
    },
    Cells {
        key: "SM-wt",
        is_bare: "false",
        worktree: "linked=false git=$B/supwt/.git/worktrees/supwt-wt/modules/sub common=$B/supwt/.git/worktrees/supwt-wt/modules/sub main=$B/supwt-wt/sub",
        superproject: "$B/supwt-wt",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/supwt-wt/sub jj=absent",
    },
    Cells {
        key: "RF",
        is_bare: "false",
        worktree: "linked=false git=$B/RF/.git common=$B/RF/.git main=$B/RF",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/RF jj=absent",
    },
    Cells {
        key: "S256",
        is_bare: "error",
        worktree: "error",
        superproject: "error",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=error jj=absent",
    },
    Cells {
        key: "HOSTILE",
        is_bare: "false",
        worktree: "linked=false git=$B/HOSTILE/.git common=$B/HOSTILE/.git main=$B/HOSTILE",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=$B/HOSTILE jj=absent",
    },
    Cells {
        key: "D1",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "error",
        jj_repository: "error",
        dual_roots: "git=absent jj=error",
    },
    Cells {
        key: "D2",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "absent",
        jj_repository: "absent",
        dual_roots: "git=absent jj=absent",
    },
    Cells {
        key: "D3",
        is_bare: "absent",
        worktree: "absent",
        superproject: "absent",
        jj_workspace_root: "$B/D3",
        jj_repository: "error",
        dual_roots: "git=absent jj=$B/D3",
    },
];

fn simple<T: std::fmt::Display>(value: &Result<Option<T>, Error>) -> String {
    match value {
        Ok(Some(inner)) => inner.to_string(),
        Ok(None) => "absent".to_owned(),
        Err(_) => "error".to_owned(),
    }
}

fn path_cell(value: &Result<Option<std::path::PathBuf>, Error>) -> String {
    match value {
        Ok(Some(path)) => path.display().to_string(),
        Ok(None) => "absent".to_owned(),
        Err(_) => "error".to_owned(),
    }
}

fn worktree_cell(value: &Result<Option<WorktreeFacts>, Error>) -> String {
    match value {
        Ok(Some(facts)) => format!(
            "linked={} git={} common={} main={}",
            facts.linked,
            facts.git_dir.display(),
            facts.common_dir.display(),
            facts.main_worktree_root.as_ref().map_or_else(
                || "absent".to_owned(),
                |path| path.display().to_string()
            )
        ),
        Ok(None) => "absent".to_owned(),
        Err(_) => "error".to_owned(),
    }
}

fn repository_cell(value: &Result<Option<JjRepositoryFacts>, Error>) -> String {
    match value {
        Ok(Some(facts)) => {
            let role = match facts.role {
                JjWorkspaceRole::Main => "main",
                JjWorkspaceRole::Secondary => "secondary",
            };
            format!("{role} {}", facts.main_root.display())
        }
        Ok(None) => "absent".to_owned(),
        Err(_) => "error".to_owned(),
    }
}

fn dual_cell(roots: &DualRoots) -> String {
    format!("git={} jj={}", path_cell(&roots.git), path_cell(&roots.jj))
}

#[test]
fn every_query_matches_the_recorded_oracle_mapping() -> Result<(), TestError> {
    assert_git_is_recent_enough()?;
    let base = tempfile::Builder::new().prefix("vcs-queries-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    let probe = InProcessProbe;
    let root = matrix.base.display().to_string();

    let mut failures = String::new();
    for cells in EXPECTED {
        let start = matrix.start(cells.key)?;
        let mut check = |query: &str, actual: &str, expected: &str| {
            let expected = expected.replace("$B", &root);
            if actual != expected {
                let _ = writeln!(
                    failures,
                    "  {}/{query}\n    expected {expected}\n    actual   {actual}",
                    cells.key
                );
            }
        };

        check("is_bare", &simple(&probe.is_bare(start)), cells.is_bare);
        check(
            "worktree",
            &worktree_cell(&probe.worktree(start)),
            cells.worktree,
        );
        check(
            "superproject",
            &path_cell(&probe.superproject(start)),
            cells.superproject,
        );
        check(
            "jj_workspace_root",
            &path_cell(&probe.jj_workspace_root(start)),
            cells.jj_workspace_root,
        );
        check(
            "jj_repository",
            &repository_cell(&probe.jj_repository(start)),
            cells.jj_repository,
        );
        check(
            "dual_roots",
            &dual_cell(&probe.dual_roots(start)),
            cells.dual_roots,
        );
    }

    assert!(failures.is_empty(), "query mismatches:\n{failures}");
    assert_eq!(
        EXPECTED.len(),
        matrix.fixtures.len(),
        "the matrix and the expectation table have drifted apart"
    );
    Ok(())
}

/// The relative-start precondition, one test per walk.
///
/// `gix::discover` absolutises against the process cwd internally while the
/// marker walk is purely lexical, so without the choke point a relative start
/// makes a colocated checkout report a git root and no jj root — the wrong arm.
/// The matrix cannot catch this, because every fixture path is absolute.
#[test]
fn a_relative_start_resolves_the_same_as_an_absolute_one(
) -> Result<(), TestError> {
    assert_git_is_recent_enough()?;
    let base = tempfile::Builder::new().prefix("vcs-relative-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    let probe = InProcessProbe;

    // A colocated checkout is the discriminating shape: both walks must find it.
    let absolute = matrix.start("CR")?;
    let parent = absolute.parent().ok_or("CR has no parent")?;
    let name = absolute.file_name().ok_or("CR has no final component")?;

    let previous = std::env::current_dir()?;
    std::env::set_current_dir(parent)?;
    let relative = Path::new(name);

    let roots = probe.dual_roots(relative);
    let git_root = path_cell(&roots.git);
    let jj_root = path_cell(&roots.jj);
    let bareness = simple(&probe.is_bare(relative));
    let facts = worktree_cell(&probe.worktree(relative));
    std::env::set_current_dir(previous)?;

    let expected = absolute.display().to_string();
    assert_eq!(
        git_root, expected,
        "the gix walk disagreed on a relative start"
    );
    assert_eq!(
        jj_root, expected,
        "the .jj-only walk disagreed on a relative start"
    );
    assert_eq!(bareness, "false");
    assert!(
        facts.starts_with("linked=false"),
        "worktree disagreed on a relative start: {facts}"
    );
    Ok(())
}

// --- The error channel, asserted specifically ---
//
// The table above renders every failure as the single token `error`, which
// proves a cell is not `Ok` but not *which* failure it is. These pin the
// distinctions the `Result<Option<T>, _>` signature exists to carry.

#[test]
fn failure_and_absence_are_distinguishable() -> Result<(), TestError> {
    assert_git_is_recent_enough()?;
    let base = tempfile::Builder::new().prefix("vcs-errors-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    let probe = InProcessProbe;

    // D3 is the load-bearing degenerate: its `.jj/repo` points at an existing
    // directory that is not a store, so `jj workspace root` *succeeds*. Only
    // the `<main_root>/.jj/repo` post-condition separates a real answer from a
    // plausible wrong one — without it this reports Secondary with a bogus
    // root, which would become RepoFacts.name and stamp artifacts with the
    // wrong repository.
    let wrong_store = matrix.start("D3")?;
    assert!(
        matches!(probe.jj_workspace_root(wrong_store), Ok(Some(_))),
        "D3's workspace root should resolve, matching `jj workspace root`"
    );
    assert!(
        probe.jj_repository(wrong_store).is_err(),
        "D3's store post-condition must fail rather than yield a wrong root"
    );

    // A tree with no repository at all is absence, not failure — the other half
    // of the distinction, without which the assertions above are vacuous.
    let loose = matrix.start("NONE")?;
    assert!(matches!(probe.jj_workspace_root(loose), Ok(None)));
    assert!(matches!(probe.jj_repository(loose), Ok(None)));
    assert!(matches!(probe.is_bare(loose), Ok(None)));
    assert!(matches!(probe.worktree(loose), Ok(None)));

    // D1's `.jj/repo` points at a *deleted* directory. Both CLIs report plain
    // absence, but a `.jj` tree that cannot be read is not the same as no jj
    // repository here, so this is `Err` — a deliberate divergence from the
    // oracle, and the one the partition rule requires.
    let deleted_store = matrix.start("D1")?;
    assert!(
        probe.jj_workspace_root(deleted_store).is_err(),
        "a broken .jj/repo pointer must not read as `no VCS here`"
    );

    // D2's git side genuinely finds no repository, so it is absence.
    let broken_worktree = matrix.start("D2")?;
    assert!(matches!(probe.worktree(broken_worktree), Ok(None)));
    assert!(matches!(probe.jj_workspace_root(broken_worktree), Ok(None)));
    Ok(())
}

#[test]
fn an_unsupported_object_format_fails_rather_than_misreads(
) -> Result<(), TestError> {
    assert_git_is_recent_enough()?;
    let base = tempfile::Builder::new().prefix("vcs-formats-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    let probe = InProcessProbe;

    // gix 0.85 rejects extensions.objectFormat = sha256. Erroring is the
    // correct outcome under the partition rule — a repository IS here and the
    // pinned library cannot answer — and is why the queries carry an error
    // channel at all. Collapsing it to `Ok(None)` would report "no repository"
    // for a perfectly valid checkout.
    let sha256 = matrix.start("S256")?;
    assert!(probe.is_bare(sha256).is_err());
    assert!(probe.worktree(sha256).is_err());
    assert!(probe.dual_roots(sha256).git.is_err());

    // The reftable backend, by contrast, reads normally.
    let reftable = matrix.start("RF")?;
    assert!(matches!(probe.is_bare(reftable), Ok(Some(false))));
    assert!(matches!(probe.worktree(reftable), Ok(Some(_))));
    Ok(())
}

#[test]
fn a_one_sided_failure_leaves_the_other_side_readable() -> Result<(), TestError>
{
    assert_git_is_recent_enough()?;
    let base = tempfile::Builder::new().prefix("vcs-onesided-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;

    // The whole reason dual_roots carries a Result per side: a repository whose
    // git side the pinned library cannot parse must never be observable as
    // "jj only", which is what a whole-struct Result flattened to None would do
    // on the single field that separates colocated from nested.
    let roots = InProcessProbe.dual_roots(matrix.start("S256")?);
    assert!(roots.git.is_err(), "the git side should report its failure");
    assert!(
        matches!(roots.jj, Ok(None)),
        "the jj side should still answer"
    );
    Ok(())
}

/// The declared pins agree by construction — `test_vcs_pin_lockstep.py` asserts
/// that. What nothing else checks is that the `jj` **binary** building these
/// fixtures matches the `jj-lib` the queries read them with. This session's
/// planning hit exactly that skew (a Homebrew jj 0.42 against a jj-lib 0.43
/// design), and it surfaces as an apparently wrong answer in a 34-row
/// expected-value table rather than as a version mismatch.
#[test]
fn the_jj_binary_matches_the_resolved_library() -> Result<(), TestError> {
    // The resolved version, not the declared requirement: the tilde range
    // deliberately permits a patch to float, and the lock is what says which
    // one is actually linked.
    let lock = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.lock"),
    )?;
    let resolved = lock
        .split("[[package]]")
        .find_map(|entry| {
            entry.contains("name = \"jj-lib\"").then(|| {
                entry
                    .lines()
                    .find_map(|line| line.strip_prefix("version = "))
                    .map(|value| value.trim_matches('"').to_owned())
            })?
        })
        .ok_or("jj-lib is not in cli/Cargo.lock")?;

    vcs_test_support::hermetic::assert_jj_matches(&resolved)?;
    Ok(())
}
