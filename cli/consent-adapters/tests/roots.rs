//! The consent policy's repository roots: the config root beside every root
//! of the repositories enclosing the working directory.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use consent_adapters::repository_roots;

#[test]
fn outside_any_repository_the_config_root_alone_is_inside() {
    let scratch = tempfile::Builder::new()
        .prefix("consent-roots-")
        .tempdir()
        .unwrap();
    let base = scratch.path().canonicalize().unwrap();
    let config_root = base.join("project");
    let cwd = base.join("elsewhere");
    std::fs::create_dir_all(&config_root).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();

    let roots = repository_roots(&config_root, &cwd);

    assert!(roots.is_complete());
    assert!(roots.contains(&config_root.join("bin")));
    assert!(!roots.contains(&cwd));
    assert!(!roots.contains(Path::new("/usr/bin")));
}

#[test]
fn a_config_root_that_cannot_be_resolved_leaves_the_roots_incomplete() {
    let scratch = tempfile::Builder::new()
        .prefix("consent-roots-")
        .tempdir()
        .unwrap();

    let roots =
        repository_roots(&scratch.path().join("missing"), scratch.path());

    assert!(!roots.is_complete());
}
