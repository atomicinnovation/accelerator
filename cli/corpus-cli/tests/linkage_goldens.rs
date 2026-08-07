//! `accelerator-corpus linkage extract` against the 8-fixture corpus already
//! exercised by `cli/corpus-adapters/tests/parity.rs`, proving the CLI
//! wiring (config composition → doc-type table → `parse_document` → TSV
//! rendering) end to end.
//!
//! Unconditional, like the other `*_goldens.rs` suites: no `bash-parity`
//! feature gate.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-corpus");

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

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("corpus-linkage-golden-{tag}-"))
        .tempdir()?)
}

/// The tempdir's canonicalised path. On macOS `TMPDIR` sits under `/var`,
/// itself a symlink to `/private/var`; the compiled binary's own
/// `current_dir()`-based project-root discovery reports the resolved form,
/// so an unresolved path here would never share a prefix with the doc-type
/// table the binary builds, and every path-based inference would silently
/// report `unknown`/`ambiguous`.
fn canonical_root(dir: &tempfile::TempDir) -> Result<PathBuf, TestError> {
    Ok(dir.path().canonicalize()?)
}

fn repo(dir: &Path) -> Result<(), TestError> {
    fs::create_dir_all(dir.join(".git"))?;
    Ok(())
}

fn run(dir: &Path, args: &[&str]) -> Result<Output, TestError> {
    Ok(Command::new(BIN).current_dir(dir).args(args).output()?)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn every_fixture_extracts_correctly() -> Result<(), TestError> {
    let dir = tempdir("fixtures")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;

    for (relative, content) in FIXTURES {
        let file = root.join(relative);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file, content)?;

        let output = run(
            &root,
            &["linkage", "extract", file.to_str().ok_or("non-utf8 path")?],
        )?;
        assert!(
            output.status.success(),
            "{relative} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Every fixture carries at least one qualifying-section reference;
        // a blank result means the CLI wiring (not the fixture itself) is
        // broken.
        assert!(
            !stdout(&output).is_empty(),
            "{relative} produced no linkage records"
        );
    }
    Ok(())
}

#[test]
fn a_dependencies_section_resolves_bare_ids_against_the_pair_table(
) -> Result<(), TestError> {
    let dir = tempdir("deps")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("meta/work/0050-deps.md");
    fs::create_dir_all(file.parent().ok_or("no parent")?)?;
    fs::write(&file, "## Dependencies\n- Blocks: 0061\n")?;

    let output = run(
        &root,
        &["linkage", "extract", file.to_str().ok_or("non-utf8 path")?],
    )?;
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "work-item\tblocks\twork-item:0061\tbody:dependencies#0\tresolved\n"
    );
    Ok(())
}

#[test]
fn an_explicit_source_type_overrides_path_inference() -> Result<(), TestError> {
    let dir = tempdir("override")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("meta/work/0050-deps.md");
    fs::create_dir_all(file.parent().ok_or("no parent")?)?;
    fs::write(&file, "## Dependencies\n- Blocks: 0061\n")?;

    let output = run(
        &root,
        &[
            "linkage",
            "extract",
            file.to_str().ok_or("non-utf8 path")?,
            "--source-type",
            "plan",
        ],
    )?;
    assert!(output.status.success());
    assert!(stdout(&output).starts_with("plan\t"));
    Ok(())
}

#[test]
fn a_missing_file_exits_1() -> Result<(), TestError> {
    let dir = tempdir("missing")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let output = run(&root, &["linkage", "extract", "meta/work/0001-x.md"])?;
    assert_eq!(output.status.code(), Some(1));
    Ok(())
}

#[test]
fn a_configured_doc_type_directory_is_honoured() -> Result<(), TestError> {
    let dir = tempdir("configured")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    fs::create_dir_all(root.join(".accelerator"))?;
    fs::write(
        root.join(".accelerator/config.md"),
        "---\npaths:\n  work: tickets\n---\n",
    )?;
    let file = root.join("tickets/0050-deps.md");
    fs::create_dir_all(file.parent().ok_or("no parent")?)?;
    fs::write(&file, "## Dependencies\n- Blocks: 0061\n")?;

    let output = run(
        &root,
        &["linkage", "extract", file.to_str().ok_or("non-utf8 path")?],
    )?;
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "work-item\tblocks\twork-item:0061\tbody:dependencies#0\tresolved\n",
        "the configured 'tickets' directory must resolve to work-item, not \
         the catalogue default 'meta/work'"
    );
    Ok(())
}
