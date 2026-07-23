//! Differential parity: the Rust linkage extractor must agree, record for
//! record, with the live bash `linkage-parser.sh` over a fixture corpus.
//!
//! The bash script is the oracle. An absent script or bash hard-fails rather
//! than skipping: Rust's harness has no skip primitive, so a silent early
//! return would register as a green PASS.
#![cfg(feature = "bash-parity")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use common::{
    doc_type_table, launcher_binary, require_file, require_script, TestError,
};
use corpus::DocTypeKey;

static COUNTER: AtomicU64 = AtomicU64::new(0);

const FIXTURES: [(&str, &str); 8] = [
    (
        "meta/plans/2026-01-01-0001-refs.md",
        "## References\n\
         - Sibling component plans: `meta/plans/2026-01-03-0003-other.md`\n\
         - Source: `meta/work/0063-owning.md`\n\
         - A plain ref `meta/plans/2026-02-03-0065-bare.md`\n\
         - The template `meta/decisions/ADR-NNNN-description.md` is not a link\n",
    ),
    (
        "meta/work/0050-deps.md",
        "## Dependencies\n\
         - Blocks: 0061\n\
         - Blocked by: 0062\n\
         - Related: 0030\n\
         - The code-block rendering is unrelated to dependencies\n",
    ),
    (
        "meta/decisions/ADR-0067-hist.md",
        "## Historical Context\n\
         - Supersedes `meta/decisions/ADR-0026-old.md`\n",
    ),
    (
        "meta/plans/2026-02-04-0066-research.md",
        "## Related Research\n\
         - `meta/research/codebase/2026-02-04-rr.md`\n\
         - `meta/research/issues/2026-02-05-issue.md`\n",
    ),
    (
        "meta/reviews/prs/42-review-1.md",
        "## References\n- Reviews `pr:42` and `pr:43`\n",
    ),
    (
        "meta/work/0081-note.md",
        "## References\n\
         - `meta/notes/2026-01-01-some-note.md`\n\
         ## Source References\n\
         - Source: https://example.com/spec\n",
    ),
    (
        "meta/notes/2026-03-01-a-note.md",
        "## Summary\n\
         - This section does not qualify: `meta/work/0099-ignored.md`\n\
         ## References\n\
         - But this one does: `meta/work/0098-counted.md`\n",
    ),
    (
        "meta/plans/2026-04-01-0070-multi.md",
        "## References\n\
         - Supersedes `meta/decisions/ADR-0026-old.md` and ADR-0026 again\n\
         - Two refs: `meta/work/0001-a.md` and `meta/work/0002-b.md`\n",
    ),
];

fn tempdir() -> Result<PathBuf, TestError> {
    let dir = std::env::temp_dir().join(format!(
        "corpus-parity-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn bash_records(script: &Path, file: &Path) -> Result<Vec<String>, TestError> {
    // linkage-parser.sh sources doc-type-table.sh, whose resolver is
    // `${ACCELERATOR_BIN:-…/bin/accelerator} config paths --doc-types`. Point it
    // at the compiled launcher so the oracle never falls through to the
    // bootstrap (which needs CLAUDE_PLUGIN_ROOT or a signed release).
    let output = Command::new(script)
        .arg(file)
        .env("ACCELERATOR_BIN", launcher_binary()?)
        .output()
        .map_err(|error| {
            format!(
                "could not run linkage-parser.sh (is bash present?): {error}"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "linkage-parser.sh failed for {}: {}",
            file.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .map(str::to_owned)
        .collect())
}

fn rust_records(
    file: &Path,
    content: &str,
    table: &[(DocTypeKey, PathBuf)],
) -> Vec<String> {
    let path = file.to_string_lossy();
    let source_type =
        corpus::linkage::type_from_path(&path, table).unwrap_or("unknown");
    corpus::linkage::parse_document(source_type, content, table)
        .into_iter()
        .map(|record| {
            format!(
                "{}\t{}\t{}\t{}\t{}",
                record.source_type,
                record.key,
                record.target_ref,
                record.anchor,
                record.band.as_str()
            )
        })
        .collect()
}

/// Compiles a work-item id pattern into its scan regex using the real bash DSL
/// compiler — the regex `corpus` takes by injection.
fn compile_scan(pattern: &str, project: &str) -> Result<String, TestError> {
    let script = require_script("skills/work/scripts/work-item-pattern.sh")?;
    let output = Command::new(&script)
        .arg("--compile-scan")
        .arg(pattern)
        .arg(project)
        .output()
        .map_err(|error| {
            format!("could not run work-item-pattern.sh: {error}")
        })?;
    if !output.status.success() {
        return Err(format!(
            "--compile-scan failed for {pattern}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[test]
#[allow(clippy::literal_string_with_formatting_args)]
fn the_compiled_scan_regex_drives_slug_and_id_extraction(
) -> Result<(), TestError> {
    use corpus::{DocTypeKey, WorkItemIdScheme};
    use corpus_adapters::RegexScanner;

    let scanner = RegexScanner::compile(&compile_scan("{number:04d}", "")?)?;
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

    let scanner = RegexScanner::compile(&compile_scan(
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
    fs::remove_dir_all(&root)?;

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

#[test]
fn linkage_extraction_matches_the_bash_parser() -> Result<(), TestError> {
    let script = require_script("scripts/linkage-parser.sh")?;
    let table = doc_type_table()?;
    let root = tempdir()?;

    let mut compared = 0usize;
    for (relative, content) in FIXTURES {
        let file = root.join(relative);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file, content)?;

        let expected = bash_records(&script, &file)?;
        let actual = rust_records(&file, content, &table);
        assert_eq!(
            actual,
            expected,
            "linkage drift for {relative}\n  rust: {actual:#?}\n  bash: {expected:#?}"
        );
        compared += 1;
    }

    assert_eq!(compared, FIXTURES.len(), "every fixture must be compared");
    fs::remove_dir_all(&root)?;
    Ok(())
}
