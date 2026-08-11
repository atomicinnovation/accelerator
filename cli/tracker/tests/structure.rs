//! The crate ships no behaviour and no dependencies, and has no adapter
//! sibling. None of that is visible to rustdoc JSON, so it is checked here.

use std::error::Error;
use std::path::PathBuf;

type TestError = Box<dyn Error>;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sources() -> Result<Vec<PathBuf>, TestError> {
    let mut paths = Vec::new();
    let mut pending = vec![manifest_dir().join("src")];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                paths.push(path);
            }
        }
    }
    if paths.is_empty() {
        return Err("found no sources under src/".into());
    }
    paths.sort();
    Ok(paths)
}

fn declares_a_test_module(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("#[cfg("))
        .any(|line| line.contains("test"))
}

#[test]
fn the_crate_declares_no_test_module() -> Result<(), TestError> {
    for path in sources()? {
        let source = std::fs::read_to_string(&path)?;
        assert!(
            !declares_a_test_module(&source),
            "{} declares a test module; the crate's tests live in tests/",
            path.display()
        );
    }
    Ok(())
}

#[test]
fn the_manifest_declares_no_dependencies() -> Result<(), TestError> {
    let manifest = std::fs::read_to_string(manifest_dir().join("Cargo.toml"))?;
    for table in ["[dependencies]", "[dev-dependencies]"] {
        assert!(
            !manifest.contains(table),
            "tracker declares {table}; the crate's whole value is that a \
             client of this port acquires nothing else with it"
        );
    }
    Ok(())
}

#[test]
fn the_workspace_lists_no_adapter_sibling() -> Result<(), TestError> {
    let workspace = manifest_dir().join("..").join("Cargo.toml");
    let manifest = std::fs::read_to_string(&workspace)?;
    assert!(
        !manifest.contains("tracker-adapters"),
        "the port is deliberately the workspace's first domain crate with no \
         -adapters sibling; provider clients live in their own crates"
    );
    Ok(())
}
