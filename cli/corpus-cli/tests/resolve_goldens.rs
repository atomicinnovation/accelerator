//! `accelerator-corpus resolve` black-box CLI coverage: the full exit-code
//! taxonomy (0/1/2/3/4/6) against a set type (`design-inventory`) and a flat
//! dated type (`codebase-research`), including member and slot directory
//! paths, paths the set's shape does not hold, an in-root path resolved
//! through a non-canonical temp dir, and the outside-root refusal.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-corpus");

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("corpus-resolve-golden-{tag}-"))
        .tempdir()?)
}

/// The tempdir's canonicalised path — see `linkage_goldens.rs::canonical_root`
/// for why this matters on macOS.
fn canonical_root(dir: &tempfile::TempDir) -> Result<PathBuf, TestError> {
    Ok(dir.path().canonicalize()?)
}

fn repo(dir: &Path) -> Result<(), TestError> {
    fs::create_dir_all(dir.join(".git"))?;
    Ok(())
}

fn touch(path: &Path) -> Result<(), TestError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "placeholder\n")?;
    Ok(())
}

fn run(dir: &Path, args: &[&str]) -> Result<Output, TestError> {
    Ok(Command::new(BIN).current_dir(dir).args(args).output()?)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const INVENTORIES: &str = "meta/research/design-inventories";
const CODEBASE: &str = "meta/research/codebase";

#[test]
fn a_nested_slug_resolves_to_its_set_directory() -> Result<(), TestError> {
    let dir = tempdir("nested-slug")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let set = root.join(INVENTORIES).join("2026-05-06-135214-current-app");
    touch(&set.join("inventory.md"))?;

    let output = run(
        &root,
        &[
            "resolve",
            "--type",
            "design-inventory",
            "135214-current-app",
        ],
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim(), set.to_str().ok_or("non-utf8")?);
    Ok(())
}

#[test]
fn a_nested_set_directory_path_resolves_to_itself() -> Result<(), TestError> {
    let dir = tempdir("nested-dir")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let rel = format!("{INVENTORIES}/2026-05-06-135214-current-app");
    let set = root.join(&rel);
    touch(&set.join("inventory.md"))?;

    let output = run(&root, &["resolve", "--type", "design-inventory", &rel])?;
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim(), set.to_str().ok_or("non-utf8")?);
    Ok(())
}

#[test]
fn a_nested_sub_document_path_resolves_to_its_set_root() -> Result<(), TestError>
{
    let dir = tempdir("nested-subdoc")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let set = root.join(INVENTORIES).join("2026-05-06-135214-current-app");
    touch(&set.join("inventory.md"))?;
    let shot = set.join("screenshots/home.png");
    touch(&shot)?;
    let rel = format!(
        "{INVENTORIES}/2026-05-06-135214-current-app/screenshots/home.png"
    );

    let output = run(&root, &["resolve", "--type", "design-inventory", &rel])?;
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim(), set.to_str().ok_or("non-utf8")?);
    Ok(())
}

#[test]
fn a_flat_slug_shared_across_dates_is_ambiguous() -> Result<(), TestError> {
    let dir = tempdir("flat-ambiguous")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let slug = "0277-single-round-web-research-engine";
    touch(&root.join(CODEBASE).join(format!("2026-09-08-{slug}.md")))?;
    touch(&root.join(CODEBASE).join(format!("2026-09-09-{slug}.md")))?;

    let output = run(&root, &["resolve", "--type", "codebase-research", slug])?;
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(stderr(&output).contains("E_RESOLVE_AMBIGUOUS"));
    assert!(stderr(&output).contains("2026-09-08"));
    assert!(stderr(&output).contains("2026-09-09"));
    Ok(())
}

#[test]
fn an_empty_input_is_invalid() -> Result<(), TestError> {
    let dir = tempdir("invalid")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;

    let output = run(&root, &["resolve", "--type", "codebase-research", ""])?;
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stderr(&output).contains("E_RESOLVE_INVALID"));
    Ok(())
}

#[test]
fn an_absent_slug_is_not_found() -> Result<(), TestError> {
    let dir = tempdir("not-found")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    fs::create_dir_all(root.join(CODEBASE))?;

    let output = run(
        &root,
        &["resolve", "--type", "codebase-research", "no-such-slug"],
    )?;
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
    assert!(stderr(&output).contains("E_RESOLVE_NOT_FOUND"));
    Ok(())
}

#[test]
fn an_unregistered_type_is_a_distinct_unknown_type_code(
) -> Result<(), TestError> {
    let dir = tempdir("unknown-type")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;

    let output =
        run(&root, &["resolve", "--type", "not-a-doc-type", "any-slug"])?;
    assert_eq!(output.status.code(), Some(4), "{}", stderr(&output));
    assert!(stderr(&output).contains("E_RESOLVE_UNKNOWN_TYPE"));
    Ok(())
}

#[test]
fn a_path_outside_the_type_directory_is_refused() -> Result<(), TestError> {
    let dir = tempdir("outside-root")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    fs::create_dir_all(root.join(CODEBASE))?;
    touch(&root.join("meta/work/0277-single-round-web-research-engine.md"))?;

    let output = run(
        &root,
        &[
            "resolve",
            "--type",
            "codebase-research",
            "meta/work/0277-single-round-web-research-engine.md",
        ],
    )?;
    assert_eq!(output.status.code(), Some(6), "{}", stderr(&output));
    assert!(stderr(&output).contains("E_RESOLVE_OUTSIDE_ROOT"));
    Ok(())
}

#[test]
fn an_in_root_path_resolves_through_a_non_canonical_temp_dir(
) -> Result<(), TestError> {
    // Run with the tempdir's own (symlinked, non-`/private`) path as cwd so the
    // candidate canonicalises to `/private/...` while a raw project-root join
    // would not — regression cover for canonicalising the containment root.
    let dir = tempdir("in-root-symlink")?;
    let start = dir.path();
    repo(start)?;
    let rel = format!("{CODEBASE}/2026-09-08-slugged-topic.md");
    touch(&start.join(&rel))?;

    let output = run(start, &["resolve", "--type", "codebase-research", &rel])?;
    assert!(output.status.success(), "{}", stderr(&output));
    let resolved = stdout(&output);
    assert!(
        resolved.trim().ends_with("2026-09-08-slugged-topic.md"),
        "{resolved}"
    );
    Ok(())
}

fn assert_not_found(output: &Output) {
    assert_eq!(output.status.code(), Some(3), "{}", stderr(output));
    assert!(stderr(output).contains("E_RESOLVE_NOT_FOUND"));
}

#[test]
fn a_rootless_set_directory_does_not_resolve_by_slug_or_path(
) -> Result<(), TestError> {
    let dir = tempdir("rootless")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let rel = format!("{INVENTORIES}/2026-05-06-135214-current-app");
    touch(&root.join(&rel).join("screenshots/home.png"))?;

    for input in ["135214-current-app", rel.as_str()] {
        let output =
            run(&root, &["resolve", "--type", "design-inventory", input])?;
        assert_not_found(&output);
    }
    Ok(())
}

#[test]
fn a_dot_prefixed_directory_does_not_resolve_by_exact_name_or_path(
) -> Result<(), TestError> {
    let dir = tempdir("dot-prefixed")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let rel = format!("{INVENTORIES}/.current-app.tmp");
    touch(&root.join(&rel).join("inventory.md"))?;

    let inventory = format!("{rel}/inventory.md");
    for input in [".current-app.tmp", rel.as_str(), inventory.as_str()] {
        let output =
            run(&root, &["resolve", "--type", "design-inventory", input])?;
        assert_not_found(&output);
    }
    Ok(())
}

#[test]
fn an_unslotted_file_in_a_set_does_not_resolve() -> Result<(), TestError> {
    let dir = tempdir("unslotted")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let rel = format!("{INVENTORIES}/2026-05-06-135214-current-app");
    touch(&root.join(&rel).join("inventory.md"))?;

    for file in ["notes.txt", "prototype-full/x.html", ".inventory.md.swp"] {
        touch(&root.join(&rel).join(file))?;
        let input = format!("{rel}/{file}");
        let output =
            run(&root, &["resolve", "--type", "design-inventory", &input])?;
        assert_not_found(&output);
    }
    Ok(())
}

#[test]
fn a_stray_file_in_the_type_directory_does_not_resolve_as_a_set(
) -> Result<(), TestError> {
    let dir = tempdir("stray")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let rel = format!("{INVENTORIES}/stray.md");
    touch(&root.join(&rel))?;

    let output = run(&root, &["resolve", "--type", "design-inventory", &rel])?;
    assert_not_found(&output);
    Ok(())
}

#[test]
fn a_path_to_a_slot_directory_resolves_to_its_set() -> Result<(), TestError> {
    let dir = tempdir("slot-dir")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let set = root.join(INVENTORIES).join("2026-05-06-135214-current-app");
    touch(&set.join("inventory.md"))?;
    touch(&set.join("screenshots/home.png"))?;
    let rel =
        format!("{INVENTORIES}/2026-05-06-135214-current-app/screenshots");

    let output = run(&root, &["resolve", "--type", "design-inventory", &rel])?;
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim(), set.to_str().ok_or("non-utf8")?);
    Ok(())
}
