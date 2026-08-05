//! The matrix builds, and every shape it claims is the shape it produced.
//!
//! These assertions are structural — markers, file-vs-directory, formats — and
//! deliberately independent of any library that reads them, so a fixture defect
//! fails here rather than as a wrong expected value in a query table.

use std::fs;

use vcs_test_support::fixtures;
use vcs_test_support::fixtures::Matrix;

type TestError = Box<dyn std::error::Error>;

fn matrix() -> Result<(Option<tempfile::TempDir>, Matrix), TestError> {
    let (guard, root) = fixtures::matrix_root()?;
    let built = Matrix::build_or_adopt(&root)?;
    Ok((guard, built))
}

#[test]
fn every_recorded_fixture_key_is_built() -> Result<(), TestError> {
    let (_guard, matrix) = matrix()?;

    let expected = [
        "CR", "CG", "JS-r", "JS-s", "PG-r", "PG-s", "NJG-i", "NJG-o", "NGJ-i",
        "NGJ-o", "WT-l", "WT-m", "SM-1", "SM-2", "SM-s", "SO", "BARE", "NONE",
        "PJ", "PJS", "NGPJ-i", "NGPJ-o", "PJG-i", "PJG-o", "JS-in", "SM-m",
        "SM-w", "SM-wt", "RF", "S256", "HOSTILE", "D1", "D2", "D3",
    ];
    for key in expected {
        matrix.start(key)?;
    }
    assert_eq!(
        matrix.fixtures.len(),
        expected.len(),
        "the matrix grew or shrank without the key list moving with it"
    );
    Ok(())
}

#[test]
fn the_nested_inner_shapes_carry_only_one_marker() -> Result<(), TestError> {
    let (_guard, matrix) = matrix()?;

    // These must carry `.jj` only, or a `.git`-inclusive walk stops at them for
    // the wrong reason and the fixture proves nothing.
    for key in ["NJG-i", "PJG-i", "JS-in"] {
        let start = matrix.start(key)?;
        assert!(start.join(".jj").exists(), "{key} should carry .jj");
        assert!(
            !start.join(".git").exists(),
            "{key} must carry .jj only, or it proves nothing"
        );
    }

    // The git-inside-jj shapes are linked worktrees, so `.git` is a *file*.
    for key in ["NGJ-i", "NGPJ-i"] {
        let start = matrix.start(key)?;
        assert!(
            start.join(".git").is_file(),
            "{key}'s .git should be a worktree file"
        );
        assert!(!start.join(".jj").exists(), "{key} should carry no .jj");
    }
    Ok(())
}

#[test]
fn the_pure_jj_shapes_have_no_git_marker() -> Result<(), TestError> {
    let (_guard, matrix) = matrix()?;
    for key in ["PJ", "NGPJ-o"] {
        let start = matrix.start(key)?;
        assert!(
            !start.join(".git").exists(),
            "{key} was built --no-colocate and must have no .git"
        );
        assert!(start.join(".jj").is_dir(), "{key} should carry .jj");
    }
    Ok(())
}

#[test]
fn the_colocated_shapes_differ_as_recorded() -> Result<(), TestError> {
    let (_guard, matrix) = matrix()?;

    // A real colocated main owns its store, so `.jj/repo` is a directory.
    let real = matrix.start("CR")?;
    assert!(real.join(".jj/repo").is_dir(), "CR should own its store");
    assert!(
        real.join(".git").is_dir(),
        "CR's .git should be a directory"
    );

    // The hand-grafted shape shares a store and is simultaneously a linked
    // worktree, which is what makes it the only `colocated` classification.
    let grafted = matrix.start("CG")?;
    assert!(
        grafted.join(".jj/repo").is_file(),
        "CG should share a store via a .jj/repo file"
    );
    assert!(
        grafted.join(".git").is_file(),
        "CG's .git should be a worktree file"
    );
    let pointer = fs::read_to_string(grafted.join(".jj/repo"))?;
    assert!(
        !pointer.ends_with('\n'),
        "jj reads .jj/repo verbatim, so a trailing newline breaks the pointer"
    );
    Ok(())
}

#[test]
fn the_degenerate_shapes_are_actually_broken() -> Result<(), TestError> {
    let (_guard, matrix) = matrix()?;

    let deleted = matrix.start("D1")?;
    let target = fs::read_to_string(deleted.join(".jj/repo"))?;
    assert!(
        !std::path::Path::new(&target).exists(),
        "D1's pointer target should be gone"
    );

    let broken = matrix.start("D2")?;
    assert!(broken.join(".git").is_file());

    // D3 is the load-bearing one: the target exists but is not a store, so
    // `jj workspace root` succeeds and only the post-condition catches it.
    let wrong = matrix.start("D3")?;
    let target = fs::read_to_string(wrong.join(".jj/repo"))?;
    let target = std::path::Path::new(&target);
    assert!(target.is_dir(), "D3's pointer target should exist");
    assert!(
        !target.join("store").exists(),
        "D3's pointer target should not be a real jj store"
    );
    Ok(())
}

#[test]
fn the_submodule_shapes_nest_as_recorded() -> Result<(), TestError> {
    let (_guard, matrix) = matrix()?;

    assert!(matrix.start("SM-1")?.join(".git").exists());
    assert!(matrix.start("SM-2")?.join(".git").exists());
    // The path-clash shape is what a bare `rposition` over `modules`
    // components misresolves.
    assert!(matrix.start("SM-m")?.ends_with("modules/foo"));
    Ok(())
}
