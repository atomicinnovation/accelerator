//! Holding the leases of the trees `cache ensure` resolved.
//!
//! A binary of its own because it spawns no children: a child spawned while a
//! lease is held would inherit it and race every "free after drop" assertion.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::path::PathBuf;

use design_adapters::ensure::ResolvedTree;
use design_adapters::HeldTrees;
use design_adapters::TreeLeases;
use design_adapters::Unleasable;
use tree_lease::observed_held;

type TestError = Box<dyn std::error::Error>;

fn lease_of(tree: &Path) -> PathBuf {
    let mut lease = OsString::from(tree.as_os_str());
    lease.push(".lease");
    PathBuf::from(lease)
}

fn resolved(artifact: &str, path: PathBuf) -> ResolvedTree {
    ResolvedTree {
        artifact: artifact.to_owned(),
        lease: lease_of(&path),
        path,
    }
}

fn present(root: &Path, artifact: &str) -> Result<ResolvedTree, TestError> {
    let path = root.join(format!("{artifact}-tree"));
    fs::create_dir_all(&path)?;
    Ok(resolved(artifact, path))
}

fn artifacts(trees: &[ResolvedTree]) -> Vec<&str> {
    trees.iter().map(|tree| tree.artifact.as_str()).collect()
}

fn unleasable_artifacts(held: &HeldTrees) -> Vec<&str> {
    held.unleasable
        .iter()
        .map(|(tree, _)| tree.artifact.as_str())
        .collect()
}

#[test]
fn holding_the_leases_of_resolved_trees_pins_them() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let trees = [
        present(work.path(), "driver")?,
        present(work.path(), "browser")?,
    ];

    let held = TreeLeases::hold(&trees);

    for tree in &trees {
        assert!(observed_held(&tree.lease)?, "{}", tree.artifact);
    }
    drop(held);
    for tree in &trees {
        assert!(!observed_held(&tree.lease)?, "{}", tree.artifact);
    }
    Ok(())
}

#[test]
fn an_unleasable_tree_is_listed_unleasable_and_the_rest_are_held(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let driver = present(work.path(), "driver")?;
    let browser = present(work.path(), "browser")?;
    fs::create_dir_all(&browser.lease)?;

    let held = TreeLeases::hold(&[driver.clone(), browser]);

    assert!(observed_held(&driver.lease)?);
    assert_eq!(unleasable_artifacts(&held), ["browser"]);
    assert!(matches!(held.unleasable[0].1, Unleasable::Lease(_)));
    assert!(held.gone.is_empty());
    Ok(())
}

#[test]
fn a_tree_gone_before_its_lease_is_listed_gone() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let tree = resolved("driver", work.path().join("reclaimed-tree"));

    let held = TreeLeases::hold(std::slice::from_ref(&tree));

    assert!(!observed_held(&tree.lease)?);
    assert!(!tree.lease.exists(), "a stray lease file was left behind");
    assert_eq!(artifacts(&held.gone), ["driver"]);
    assert!(held.unleasable.is_empty());
    Ok(())
}

#[test]
fn a_tree_whose_cache_has_gone_is_listed_gone() -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let tree = resolved("driver", work.path().join("gone-cache/driver-tree"));

    let held = TreeLeases::hold(&[tree]);

    assert_eq!(artifacts(&held.gone), ["driver"]);
    assert!(held.unleasable.is_empty());
    Ok(())
}

#[test]
fn a_lease_file_another_holder_holds_outlives_its_gone_tree(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let tree = resolved("driver", work.path().join("reclaimed-tree"));
    let _other_holder = tree_lease::hold_shared_lease(&tree.lease)?;

    let held = TreeLeases::hold(std::slice::from_ref(&tree));

    assert_eq!(artifacts(&held.gone), ["driver"]);
    assert!(tree.lease.exists());
    assert!(observed_held(&tree.lease)?);
    Ok(())
}

/// Restores a directory's mode before the `TempDir` holding it drops, so
/// cleanup succeeds even when an assertion fails.
struct Unlocked<'a>(&'a Path);

impl Drop for Unlocked<'_> {
    fn drop(&mut self) {
        let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
    }
}

#[test]
fn an_unstattable_tree_is_listed_unleasable_not_gone() -> Result<(), TestError>
{
    if rustix::process::geteuid().is_root() {
        eprintln!("skipping: root ignores directory permissions");
        return Ok(());
    }
    let work = tempfile::tempdir()?;
    let blocked = work.path().join("blocked");
    let tree = present(&blocked, "driver")?;
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000))?;
    let _unlocked = Unlocked(&blocked);

    let held = TreeLeases::hold(&[tree]);

    assert_eq!(unleasable_artifacts(&held), ["driver"]);
    assert!(matches!(held.unleasable[0].1, Unleasable::Lease(_)));
    assert!(held.gone.is_empty());
    Ok(())
}

#[test]
fn a_lease_path_outside_the_tree_layout_is_left_untouched(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let unrelated = work.path().join("unrelated");
    fs::write(&unrelated, "keep me")?;
    let tree = ResolvedTree {
        artifact: "driver".to_owned(),
        path: work.path().join("reclaimed-tree"),
        lease: unrelated.clone(),
    };

    let held = TreeLeases::hold(&[tree]);

    assert_eq!(fs::read_to_string(&unrelated)?, "keep me");
    assert_eq!(unleasable_artifacts(&held), ["driver"]);
    assert!(matches!(
        held.unleasable[0].1,
        Unleasable::OutsideTreeLayout
    ));
    Ok(())
}

#[test]
fn a_lease_path_outside_the_tree_layout_is_never_created(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let mut tree = present(work.path(), "driver")?;
    let elsewhere = work.path().join("elsewhere.lease");
    tree.lease = elsewhere.clone();

    let held = TreeLeases::hold(&[tree]);

    assert!(!elsewhere.exists());
    assert_eq!(unleasable_artifacts(&held), ["driver"]);
    assert!(matches!(
        held.unleasable[0].1,
        Unleasable::OutsideTreeLayout
    ));
    Ok(())
}
