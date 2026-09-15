//! Migration 0010 (`strip-research-title-prefix`) driven end to end against
//! the compiled binary, with the ledger pre-seeded through 0009 so 0010 is the
//! sole pending migration.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-migrate");

fn write(dir: &Path, relative: &str, content: &str) -> Result<(), TestError> {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().ok_or("no parent")?)?;
    fs::write(path, content)?;
    Ok(())
}

fn already_applied_except_0010(dir: &Path) -> Result<(), TestError> {
    write(
        dir,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n\
         0009-split-work-key-from-tracker-scope-key\n",
    )
}

fn run(dir: &Path) -> std::io::Result<std::process::Output> {
    Command::new(BIN).current_dir(dir).output()
}

fn applied(dir: &Path) -> Result<String, TestError> {
    Ok(fs::read_to_string(
        dir.join(".accelerator/state/migrations-applied"),
    )?)
}

#[test]
fn strips_both_the_frontmatter_title_and_the_body_h1() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/research/codebase/2026-01-01-foo.md",
        "---\ntype: \"codebase-research\"\ntitle: \"Research: Foo\"\n---\n\n\
         # Research: Foo\n\n## Research Question\n",
    )?;
    already_applied_except_0010(root)?;

    let output = run(root)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    assert_eq!(
        fs::read_to_string(
            root.join("meta/research/codebase/2026-01-01-foo.md")
        )?,
        "---\ntype: \"codebase-research\"\ntitle: \"Foo\"\n---\n\n\
         # Foo\n\n## Research Question\n"
    );
    assert!(applied(root)?.contains("0010-strip-research-title-prefix"));
    Ok(())
}

#[test]
fn strips_a_frontmatter_title_only_document() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/research/codebase/a.md",
        "---\ntitle: \"Research: Only Title\"\n---\n\n# Only Title\n",
    )?;
    already_applied_except_0010(root)?;

    assert_eq!(run(root)?.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(root.join("meta/research/codebase/a.md"))?,
        "---\ntitle: \"Only Title\"\n---\n\n# Only Title\n"
    );
    Ok(())
}

#[test]
fn strips_a_body_h1_only_document() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/research/codebase/b.md",
        "---\ntitle: \"Already Clean\"\n---\n\n# Research: Only H1\n",
    )?;
    already_applied_except_0010(root)?;

    assert_eq!(run(root)?.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(root.join("meta/research/codebase/b.md"))?,
        "---\ntitle: \"Already Clean\"\n---\n\n# Only H1\n"
    );
    Ok(())
}

#[test]
fn leaves_a_research_heading_after_the_first_h2_untouched(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    let doc = "---\ntitle: \"Clean\"\n---\n\n## Section\n\n# Research: Later\n";
    write(root, "meta/research/codebase/c.md", doc)?;
    already_applied_except_0010(root)?;

    assert_eq!(run(root)?.status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(root.join("meta/research/codebase/c.md"))?,
        doc
    );
    Ok(())
}

#[test]
fn a_repo_with_no_research_codebase_directory_still_records_applied(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    already_applied_except_0010(root)?;

    let output = run(root)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        applied(root)?.contains("0010-strip-research-title-prefix"),
        "0010 must record applied even with no corpus directory"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("research_codebase directory does not exist"),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_dangerous_research_codebase_value_is_refused_not_walked(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        ".accelerator/config.md",
        "---\npaths:\n  research_codebase: ../escape\n---\n",
    )?;
    already_applied_except_0010(root)?;

    let output = run(root)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(
            "refusing dangerous paths.research_codebase value: ../escape"
        ),
        "{stderr}"
    );
    assert!(applied(root)?.contains("0010-strip-research-title-prefix"));
    Ok(())
}

#[test]
fn a_stripped_corpus_is_byte_stable_on_re_application() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/research/codebase/foo.md",
        "---\ntitle: \"Research: Foo\"\n---\n\n# Research: Foo\n",
    )?;
    already_applied_except_0010(root)?;

    run(root)?;
    let after_first =
        fs::read_to_string(root.join("meta/research/codebase/foo.md"))?;

    // Drop 0010 from the ledger so apply re-executes rather than exercising
    // the ledger gate, mirroring m0006's byte-stability test.
    already_applied_except_0010(root)?;
    let output = run(root)?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        fs::read_to_string(root.join("meta/research/codebase/foo.md"))?,
        after_first,
        "a second application must be byte-identical"
    );
    Ok(())
}
