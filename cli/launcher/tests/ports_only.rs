//! Retention, start-up and the tree prune reach the filesystem only through
//! their injected ports, so a recording or scripted port sees every call.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

const FORBIDDEN: [&str; 4] = [
    "std::fs::",
    "rustix::fs",
    "SystemFilesystem",
    "ProcessOwnership::default",
];

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
            "launch/outbound/resolve/claims_directory.rs",
            "launch/outbound/resolve/tree/claims.rs",
            "launch/outbound/resolve/tree/reap.rs",
            "launch/outbound/resolve/tree/prune.rs",
        ]
        .map(source),
    );
    sources
}

/// Everything before the first inline test module. A test-only module
/// declared without a body, such as `mod testing;`, lives in its own file,
/// so the code after it is still production code.
fn production_code(source: &str) -> &str {
    let mut offset = 0;
    let mut test_gated_from = None;
    for line in source.split_inclusive('\n') {
        let item = line.trim();
        if gates_on_test(item) {
            test_gated_from.get_or_insert(offset);
        } else if !item.starts_with("#[") {
            if let Some(start) = test_gated_from.take() {
                if opens_an_inline_module(item) {
                    return &source[..start];
                }
            }
        }
        offset += line.len();
    }
    source
}

fn gates_on_test(item: &str) -> bool {
    item.starts_with("#[cfg(test)]") || item.starts_with("#[cfg(all(test")
}

fn opens_an_inline_module(item: &str) -> bool {
    let declaration = item
        .strip_prefix("pub(crate) ")
        .or_else(|| item.strip_prefix("pub "))
        .unwrap_or(item);
    declaration.starts_with("mod ") && declaration.ends_with('{')
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
fn production_code_continues_past_a_test_only_module_declaration() {
    let source = "pub mod ports;\n\
                  #[cfg(test)]\n\
                  pub(crate) mod testing;\n\
                  pub fn reads() { std::fs::read(\"x\"); }\n\
                  \n\
                  #[cfg(test)]\n\
                  #[allow(clippy::expect_used)]\n\
                  mod tests {\n\
                  fn helper() { std::fs::write(\"y\", b\"\"); }\n\
                  }\n";

    let production = production_code(source);

    assert!(production.contains("std::fs::read"), "{production}");
    assert!(!production.contains("std::fs::write"), "{production}");
}

#[test]
fn the_guard_covers_the_modules_it_names() {
    let sources = guarded_sources();
    assert!(sources.iter().all(|path| path.is_file()), "{sources:?}");
    assert!(sources.len() > 4, "{sources:?}");
}
