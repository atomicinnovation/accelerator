//! `vcs::classify::classify` against the full fixture matrix — every shape
//! `classify_checkout` had to tell apart, replayed through the library-backed
//! `InProcessProbe`.
#![cfg(feature = "bash-parity")]

use std::path::Path;
use std::path::PathBuf;

use vcs::classify::classify;
use vcs::classify::Classification;
use vcs_adapters::library::InProcessProbe;
use vcs_test_support::fixtures::Matrix;

type TestError = Box<dyn std::error::Error>;

/// A parent directory the matrix builds but does not record as its own
/// keyed fixture (`Matrix::start` cannot name it).
fn unkeyed(matrix: &Matrix, name: &str) -> Result<PathBuf, TestError> {
    Ok(matrix.base.join(name).canonicalize()?)
}

fn classified(start: &Path) -> Result<Classification, TestError> {
    classify(start, &InProcessProbe).map_err(|error| error.to_string().into())
}

fn assert_plain_single_vcs_shapes(matrix: &Matrix) -> Result<(), TestError> {
    assert_eq!(classified(matrix.start("PG-r")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("PG-s")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("WT-m")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("BARE")?)?, Classification::None);
    assert_eq!(classified(matrix.start("NONE")?)?, Classification::None);
    assert_eq!(classified(matrix.start("PJ")?)?, Classification::Main);

    assert_eq!(
        classified(matrix.start("WT-l")?)?,
        Classification::GitWorktree {
            boundary: matrix.start("WT-l")?.to_path_buf(),
            git_parent: matrix.start("WT-m")?.to_path_buf(),
        }
    );
    Ok(())
}

fn assert_jj_secondary_shapes(matrix: &Matrix) -> Result<(), TestError> {
    let jjmain_colocated = unkeyed(matrix, "jjmain-colocated")?;

    assert_eq!(
        classified(matrix.start("JS-r")?)?,
        Classification::JjSecondary {
            boundary: matrix.start("JS-r")?.to_path_buf(),
            jj_parent: jjmain_colocated,
        }
    );
    assert_eq!(
        classified(matrix.start("JS-s")?)?,
        Classification::JjSecondary {
            boundary: matrix.start("JS-r")?.to_path_buf(),
            jj_parent: unkeyed(matrix, "jjmain-colocated")?,
        }
    );
    assert_eq!(
        classified(matrix.start("PJS")?)?,
        Classification::JjSecondary {
            boundary: matrix.start("PJS")?.to_path_buf(),
            jj_parent: matrix.start("PJ")?.to_path_buf(),
        }
    );

    // A jj secondary nested inside its own colocated main: the jj and git
    // main roots coincide (both the host), so the nested-jj-in-git
    // inequality check does not fire even though `in_git` is true —
    // classification degrades to `jj-secondary`, not `nested-jj-in-git`.
    assert_eq!(
        classified(matrix.start("JS-in")?)?,
        Classification::JjSecondary {
            boundary: matrix.start("JS-in")?.to_path_buf(),
            jj_parent: unkeyed(matrix, "JSIN")?,
        }
    );
    Ok(())
}

fn assert_colocated_shapes(matrix: &Matrix) -> Result<(), TestError> {
    // Colocated, real (main jj + main git, not secondary/worktree) is
    // `main`, not `colocated` — that arm also needs a jj secondary
    // workspace and a git linked worktree at the same location.
    assert_eq!(classified(matrix.start("CR")?)?, Classification::Main);

    // The only shape classifying as `colocated`: one path that is both.
    assert_eq!(
        classified(matrix.start("CG")?)?,
        Classification::Colocated {
            boundary: matrix.start("CG")?.to_path_buf(),
            jj_parent: unkeyed(matrix, "jjparent")?,
            git_parent: unkeyed(matrix, "gitparent-cg")?,
        }
    );
    Ok(())
}

fn assert_nested_jj_in_git_shapes(matrix: &Matrix) -> Result<(), TestError> {
    assert_eq!(
        classified(matrix.start("NJG-i")?)?,
        Classification::NestedJjInGit {
            boundary: matrix.start("NJG-i")?.to_path_buf(),
            jj_parent: unkeyed(matrix, "jjmain-colocated")?,
            git_parent: matrix.start("NJG-o")?.to_path_buf(),
        }
    );
    assert_eq!(classified(matrix.start("NJG-o")?)?, Classification::Main);

    assert_eq!(
        classified(matrix.start("PJG-i")?)?,
        Classification::NestedJjInGit {
            boundary: matrix.start("PJG-i")?.to_path_buf(),
            jj_parent: unkeyed(matrix, "jjmain-pure")?,
            git_parent: matrix.start("PJG-o")?.to_path_buf(),
        }
    );
    assert_eq!(classified(matrix.start("PJG-o")?)?, Classification::Main);
    Ok(())
}

fn assert_nested_git_in_jj_shapes(matrix: &Matrix) -> Result<(), TestError> {
    assert_eq!(
        classified(matrix.start("NGJ-i")?)?,
        Classification::NestedGitInJj {
            boundary: matrix.start("NGJ-i")?.to_path_buf(),
            jj_parent: matrix.start("NGJ-o")?.to_path_buf(),
            git_parent: unkeyed(matrix, "gitparent-ngj")?,
        }
    );
    assert_eq!(classified(matrix.start("NGJ-o")?)?, Classification::Main);

    assert_eq!(
        classified(matrix.start("NGPJ-i")?)?,
        Classification::NestedGitInJj {
            boundary: matrix.start("NGPJ-i")?.to_path_buf(),
            jj_parent: matrix.start("NGPJ-o")?.to_path_buf(),
            git_parent: unkeyed(matrix, "gitparent-ngpj")?,
        }
    );
    assert_eq!(classified(matrix.start("NGPJ-o")?)?, Classification::Main);
    Ok(())
}

fn assert_submodule_shapes(matrix: &Matrix) -> Result<(), TestError> {
    // classify() has no submodule-specific arm (that is `superproject()`'s
    // concern, unused by this cascade), so every submodule shape that is
    // not itself a linked worktree classifies as an ordinary independent
    // `main` git repository.
    assert_eq!(classified(matrix.start("SM-1")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("SM-2")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("SM-s")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("SO")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("SM-m")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("SM-wt")?)?, Classification::Main);

    // A linked worktree of a submodule reports no resolvable
    // `main_worktree_root` (the submodule's relocated git-dir, under
    // `.git/modules/`, has no "main" gix can point back to), so
    // `git-worktree` cannot be constructed without a parent — degrades to
    // `main` rather than a boundary-carrying arm with a fabricated parent.
    assert_eq!(classified(matrix.start("SM-w")?)?, Classification::Main);
    Ok(())
}

fn assert_format_and_hostile_shapes(matrix: &Matrix) -> Result<(), TestError> {
    // Format/hostile-config variants do not change the topology.
    assert_eq!(classified(matrix.start("RF")?)?, Classification::Main);
    assert_eq!(classified(matrix.start("HOSTILE")?)?, Classification::Main);
    Ok(())
}

#[test]
fn every_fixture_classifies_as_expected() -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("vcs-classify-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;

    assert_plain_single_vcs_shapes(&matrix)?;
    assert_jj_secondary_shapes(&matrix)?;
    assert_colocated_shapes(&matrix)?;
    assert_nested_jj_in_git_shapes(&matrix)?;
    assert_nested_git_in_jj_shapes(&matrix)?;
    assert_submodule_shapes(&matrix)?;
    assert_format_and_hostile_shapes(&matrix)?;

    Ok(())
}

#[test]
fn an_unsupported_object_format_fails_rather_than_misclassifies(
) -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("vcs-classify-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    assert!(classify(matrix.start("S256")?, &InProcessProbe).is_err());
    Ok(())
}

#[test]
fn a_broken_jj_store_pointer_fails_rather_than_misclassifies(
) -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("vcs-classify-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    assert!(classify(matrix.start("D1")?, &InProcessProbe).is_err());
    Ok(())
}

#[test]
fn a_removed_gitdir_behind_a_dot_git_file_is_absence_not_failure(
) -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("vcs-classify-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    assert_eq!(classified(matrix.start("D2")?)?, Classification::None);
    Ok(())
}

#[test]
fn a_jj_repo_pointer_to_a_non_store_directory_fails_rather_than_misclassifies(
) -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("vcs-classify-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    assert!(classify(matrix.start("D3")?, &InProcessProbe).is_err());
    Ok(())
}

#[test]
fn the_matrix_still_covers_every_key_this_test_exercises(
) -> Result<(), TestError> {
    let base = tempfile::Builder::new().prefix("vcs-classify-").tempdir()?;
    let matrix = Matrix::build_in(base.path())?;
    let expected = [
        "CR", "CG", "JS-r", "JS-s", "PG-r", "PG-s", "NJG-i", "NJG-o", "NGJ-i",
        "NGJ-o", "WT-l", "WT-m", "SM-1", "SM-2", "SM-s", "SO", "BARE", "NONE",
        "PJ", "PJS", "NGPJ-i", "NGPJ-o", "PJG-i", "PJG-o", "JS-in", "SM-m",
        "SM-w", "SM-wt", "RF", "S256", "HOSTILE", "D1", "D2", "D3",
    ];
    assert_eq!(matrix.fixtures.len(), expected.len());
    for key in expected {
        matrix.start(key)?;
    }
    Ok(())
}
