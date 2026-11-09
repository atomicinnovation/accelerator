//! Retention, start-up and the tree prune reach the filesystem only through
//! their injected ports, so a recording or scripted port sees every call.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 3] =
    ["std::fs::", "SystemFilesystem", "ProcessOwnership::default"];

fn source(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(relative)
}

fn guarded_sources() -> Vec<PathBuf> {
    let retention = source("launch/outbound/resolve/retention");
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&retention)
        .expect("retention sources")
        .map(|entry| entry.expect("retention source").path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "rs")
                && path.file_name().is_some_and(|name| name != "testing.rs")
        })
        .collect();
    sources.extend(
        [
            "launch/start_up.rs",
            "launch/outbound/resolve/tree/claims.rs",
            "launch/outbound/resolve/tree/reap.rs",
            "launch/outbound/resolve/tree/prune.rs",
        ]
        .map(source),
    );
    sources
}

/// Everything before the first test module.
fn production_code(source: &str) -> &str {
    source
        .find("\n#[cfg(test)]")
        .into_iter()
        .chain(source.find("\n#[cfg(all(test"))
        .min()
        .map_or(source, |test_module| &source[..test_module])
}

#[test]
fn the_guarded_modules_reach_the_filesystem_only_through_their_ports() {
    let mut offences = Vec::new();
    for path in guarded_sources() {
        let text = std::fs::read_to_string(&path).expect("source");
        for (number, line) in production_code(&text).lines().enumerate() {
            if FORBIDDEN.iter().any(|forbidden| line.contains(forbidden)) {
                offences.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(offences.is_empty(), "{}", offences.join("\n"));
}

#[test]
fn the_guard_covers_the_modules_it_names() {
    let sources = guarded_sources();
    assert!(sources.iter().all(|path| path.is_file()), "{sources:?}");
    assert!(sources.len() > 4, "{sources:?}");
}
