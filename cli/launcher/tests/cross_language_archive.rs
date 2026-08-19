//! The Python producer and the Rust consumer agree on the archive contract.
//!
//! `tasks/vendor/archive.py` builds a deterministic `.tar.gz` with the `.files`
//! table as its first member; the launcher's `extract_archive` verifies every
//! member against that table as it extracts. This test runs the real Python
//! producer over a synthetic tree and extracts the result with the real Rust
//! consumer, so a table-format disagreement fails here rather than in a
//! container fixture after both halves have merged.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use accelerator::launch::core::tree_entry::ExtractionLimits;
use accelerator::launch::outbound::resolve::tree::extract::extract_archive;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is cli/launcher; the repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn python() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| [dir.join("python3"), dir.join("python")])
        .find(|candidate| candidate.is_file())
}

/// Build an archive from `tree` into `dest` via the real Python assembly path.
fn python_build_archive(python: &Path, tree: &Path, dest: &Path) -> bool {
    let script = "from pathlib import Path; import sys; \
                  from tasks.vendor.archive import write_deterministic_archive; \
                  write_deterministic_archive(Path(sys.argv[1]), Path(sys.argv[2]))";
    let status = Command::new(python)
        .arg("-c")
        .arg(script)
        .arg(tree)
        .arg(dest)
        .current_dir(repo_root())
        .status()
        .expect("run python archive builder");
    status.success()
}

#[test]
fn a_python_built_archive_extracts_under_the_rust_contract() {
    let Some(python) = python() else {
        assert!(
            std::env::var_os("CI").is_none(),
            "python is required under CI"
        );
        eprintln!("skipping: python not on PATH");
        return;
    };

    let workdir = tempfile::tempdir().expect("tempdir");
    let tree = workdir.path().join("tree");
    std::fs::create_dir_all(tree.join("lib")).unwrap();
    std::fs::write(tree.join("lib/data.pak"), b"resource bytes").unwrap();
    let shell = tree.join("node");
    std::fs::write(&shell, b"#!/bin/sh\necho v20\n").unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755))
        .unwrap();
    std::os::unix::fs::symlink("data.pak", tree.join("lib/current")).unwrap();

    let archive = workdir.path().join("out.tar.gz");
    if !python_build_archive(&python, &tree, &archive) {
        assert!(
            std::env::var_os("CI").is_none(),
            "the python archive build failed under CI"
        );
        eprintln!("skipping: python could not import tasks.vendor.archive");
        return;
    }

    let dest = workdir.path().join("extracted");
    std::fs::create_dir_all(&dest).unwrap();
    let file = std::fs::File::open(&archive).unwrap();
    let extracted = extract_archive(
        flate2::read::GzDecoder::new(file),
        &dest,
        &ExtractionLimits {
            uncompressed_size: 1 << 30,
            entry_count: 10_000,
        },
    )
    .expect("the Rust consumer extracts the Python archive");

    // Every member landed and matched its table row (extraction verifies the
    // digests inline, so reaching here already proves agreement).
    assert_eq!(extracted.entry_count, 4);
    assert_eq!(
        std::fs::read(dest.join("lib/data.pak")).unwrap(),
        b"resource bytes"
    );
    let node_mode = std::fs::metadata(dest.join("node"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(node_mode, 0o755);
    assert_eq!(
        std::fs::read_link(dest.join("lib/current")).unwrap(),
        Path::new("data.pak")
    );
}
