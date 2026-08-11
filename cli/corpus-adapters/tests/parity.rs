//! Differential parity: the Rust doc-type matcher must agree with the live
//! bash matcher, which is the oracle. An absent script or bash hard-fails
//! rather than skipping: Rust's harness has no skip primitive, so a silent
//! early return would register as a green PASS.
//!
//! The scan-regex case alongside it has no bash oracle left — the shell
//! implementation it was measured against is gone — so it pins the compiled
//! regex's own behaviour end to end instead.
#![cfg(feature = "bash-parity")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{require_file, TestError};
use corpus::DocTypeKey;
use tempfile::TempDir;

fn tempdir() -> Result<TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix("corpus-parity-")
        .tempdir()?)
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)]
fn the_compiled_scan_regex_drives_slug_and_id_extraction(
) -> Result<(), TestError> {
    use corpus::{DocTypeKey, WorkItemIdScheme};
    use corpus_adapters::compile_scan_regex;
    use corpus_adapters::RegexScanner;

    let scanner =
        RegexScanner::compile(&compile_scan_regex("{number:04d}", "")?)?;
    let scheme = WorkItemIdScheme::numeric();
    assert_eq!(
        corpus::slug::derive(
            DocTypeKey::WorkItems,
            "0001-three-layer-review.md",
            &scheme,
            &scanner
        )
        .as_deref(),
        Some("three-layer-review")
    );
    assert_eq!(
        scheme.extract_id("0042-foo.md", &scanner).as_deref(),
        Some("0042")
    );

    let scanner = RegexScanner::compile(&compile_scan_regex(
        "{project}-{number:04d}",
        "PROJ",
    )?)?;
    let scheme = WorkItemIdScheme {
        id_pattern: "{project}-{number:04d}".to_owned(),
        default_project_code: Some("PROJ".to_owned()),
    };
    assert_eq!(
        corpus::slug::derive(
            DocTypeKey::WorkItems,
            "PROJ-0042-ship-it.md",
            &scheme,
            &scanner
        )
        .as_deref(),
        Some("ship-it")
    );
    // A legacy bare-numeric file the project scan regex rejects still yields a
    // slug via the pure fallback.
    assert_eq!(
        corpus::slug::derive(
            DocTypeKey::WorkItems,
            "0042-legacy.md",
            &scheme,
            &scanner
        )
        .as_deref(),
        Some("legacy")
    );
    Ok(())
}

/// Drives the live bash matcher over an *injected* table, so the corpus can
/// carry shapes the repo's own config does not have — a configured directory
/// nested under another, and two types tied on the same directory. Without
/// those, longest-dir-wins and first-entry-tie pass vacuously on both sides.
fn bash_infer(
    table: &[(DocTypeKey, &str)],
    paths: &[&str],
) -> Result<Vec<String>, TestError> {
    let inference = require_file("scripts/doc-type-inference.sh")?;

    let names = table
        .iter()
        .map(|(kind, _)| {
            format!("'{}'", kind.linkage_type_name().unwrap_or_default())
        })
        .collect::<Vec<_>>()
        .join(" ");
    let dirs = table
        .iter()
        .map(|(_, dir)| format!("'{dir}'"))
        .collect::<Vec<_>>()
        .join(" ");

    // doc-type-inference.sh is a sourced library, not an entry point: it reads
    // the injected table from globals the caller must populate.
    let driver = format!(
        "#!/usr/bin/env bash\n\
         set -euo pipefail\n\
         . '{}'\n\
         DOC_TYPE_INJECTED_NAMES=({names})\n\
         DOC_TYPE_INJECTED_DIRS=({dirs})\n\
         DOC_TYPE_TABLE_INJECTED=1\n\
         for path in \"$@\"; do infer_type_from_path \"$path\"; done\n",
        inference.display()
    );

    let root = tempdir()?;
    let root = root.path().to_path_buf();
    let script = root.join("drive-inference.sh");
    fs::write(&script, driver)?;

    let output = Command::new("bash")
        .arg(&script)
        .args(paths)
        .output()
        .map_err(|error| {
            format!("could not run the inference driver (is bash present?): {error}")
        })?;
    if !output.status.success() {
        return Err(format!(
            "doc-type-inference.sh driver failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::to_owned)
        .collect())
}

#[test]
fn doc_type_inference_matches_the_bash_matcher() -> Result<(), TestError> {
    // `meta/design` is a prefix of `meta/design/inventories`, and the two review
    // types are tied on the same directory — neither shape exists in the repo's
    // own config, so only an injected table can exercise them.
    let table: [(DocTypeKey, &str); 5] = [
        (DocTypeKey::Plans, "meta/plans"),
        (DocTypeKey::DesignGaps, "meta/design"),
        (DocTypeKey::DesignInventories, "meta/design/inventories"),
        (DocTypeKey::PlanReviews, "meta/reviews"),
        (DocTypeKey::PrReviews, "meta/reviews"),
    ];

    let paths = [
        // Plain match.
        "meta/plans/2026-01-01-0001-a.md",
        // The shallower configured dir, where the deeper one does not apply.
        "meta/design/2026-01-01-gap.md",
        // Nested under BOTH configured dirs: the longest must win.
        "meta/design/inventories/2026-01-01-buttons/inventory.md",
        // Same, reached as an interior segment of an absolute path.
        "/tmp/checkout/meta/design/inventories/2026-02-02-forms/inventory.md",
        // Exact-length tie: the first table entry wins, deterministically.
        "meta/reviews/2026-01-01-x-review-1.md",
        // A prefix of a configured dir, but not on a segment boundary.
        "meta/plans-archive/2026-01-01-0001-a.md",
        // Configured nowhere.
        "meta/unconfigured/x.md",
    ];

    let expected = bash_infer(&table, &paths)?;

    let rust_table: Vec<(DocTypeKey, PathBuf)> = table
        .iter()
        .map(|(kind, dir)| (*kind, PathBuf::from(dir)))
        .collect();
    let actual: Vec<String> = paths
        .iter()
        .map(|path| {
            corpus::doc_type::infer(Path::new(path), &rust_table)
                .and_then(DocTypeKey::linkage_type_name)
                .unwrap_or_default()
                .to_owned()
        })
        .collect();

    assert_eq!(
        actual, expected,
        "doc-type inference drift\n  rust: {actual:#?}\n  bash: {expected:#?}"
    );

    // Guard the oracle itself: if bash resolved nothing anywhere, the diff above
    // would agree vacuously.
    assert_eq!(
        expected,
        [
            "plan",
            "design-gap",
            "design-inventory",
            "design-inventory",
            "plan-review",
            "",
            "",
        ],
        "the bash matcher did not resolve the shapes this suite exists to pin"
    );
    Ok(())
}
