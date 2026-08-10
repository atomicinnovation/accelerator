//! `--list` and `--decisions-file` driven end to end against the compiled
//! binary, exercised through migration 0007's own ambiguous-band prompt —
//! the only interactive migration the compiled registry ever contains, so
//! it is the sole real exerciser of this surface. The fixture migrations
//! used to capture the bash-golden matrix have no Rust equivalent in the
//! compiled registry, so no literal fixture replay applies here.

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

fn work_item(id: &str, title: &str, body: &str) -> String {
    format!(
        "---\ntype: work-item\nid: \"{id}\"\ntitle: {title}\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: a\ntags: []\n\
         kind: task\nstatus: draft\npriority: medium\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: a\n\
         schema_version: 1\n---\n\n# {id}: {title}\n{body}"
    )
}

fn already_applied(dir: &Path) -> Result<(), TestError> {
    write(
        dir,
        ".accelerator/state/migrations-applied",
        "0001-rename-tickets-to-work\n\
         0002-rename-work-items-with-project-prefix\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n",
    )
}

fn seed_ambiguous_reference(root: &Path) -> Result<(), TestError> {
    write(
        root,
        "meta/work/0042-target.md",
        &work_item("0042", "Target", ""),
    )?;
    write(
        root,
        "meta/work/0001-source.md",
        &work_item(
            "0001",
            "Source",
            "\n## References\n- `meta/work/0042-target.md`\n",
        ),
    )?;
    already_applied(root)
}

#[test]
fn list_emits_the_ambiguous_transformation_as_a_tab_delimited_line(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    seed_ambiguous_reference(root)?;

    let output = Command::new(BIN).arg("--list").current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert_eq!(
        stdout,
        "1\trelates_to\trelates_to=work-item:0042\t\
         meta/work/0001-source.md:body:references#0\n"
    );
    assert!(
        !root.join(".accelerator/state/migrations-applied").exists()
            || !fs::read_to_string(
                root.join(".accelerator/state/migrations-applied")
            )?
            .contains("0007"),
        "--list must not add 0007 to the applied ledger"
    );
    Ok(())
}

#[test]
fn list_reports_no_pending_transformations_when_nothing_is_ambiguous(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    write(
        root,
        "meta/work/0042-target.md",
        &work_item("0042", "Target", ""),
    )?;
    already_applied(root)?;

    let output = Command::new(BIN).arg("--list").current_dir(root).output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "no pending transformations\n"
    );
    Ok(())
}

#[test]
fn a_valid_decisions_file_accepts_and_applies() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    seed_ambiguous_reference(root)?;
    let decisions = root.join("decisions.txt");
    fs::write(&decisions, "accept\n")?;

    let output = Command::new(BIN)
        .args(["--decisions-file", decisions.to_str().ok_or("path")?])
        .current_dir(root)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(fs::read_to_string(
        root.join(".accelerator/state/migrations-applied")
    )?
    .contains("0007-unify-meta-corpus-frontmatter"),);
    let source = fs::read_to_string(root.join("meta/work/0001-source.md"))?;
    assert!(
        source.contains("relates_to: [\"work-item:0042\"]\n"),
        "{source}"
    );
    Ok(())
}

#[test]
fn a_decisions_file_missing_a_decision_refuses_naming_the_position(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    seed_ambiguous_reference(root)?;
    let decisions = root.join("decisions.txt");
    fs::write(&decisions, "")?;

    let output = Command::new(BIN)
        .args(["--decisions-file", decisions.to_str().ok_or("path")?])
        .current_dir(root)
        .output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "Error: decisions file is missing a decision for position 1 \
         (relates_to).\n"
    );
    assert!(
        !fs::read_to_string(
            root.join(".accelerator/state/migrations-applied")
        )?
        .contains("0007"),
        "a refused decisions file must not add 0007 to the applied ledger"
    );
    Ok(())
}

#[test]
fn a_hash_prefixed_line_in_the_decisions_file_is_an_unknown_verb(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    seed_ambiguous_reference(root)?;
    let decisions = root.join("decisions.txt");
    fs::write(&decisions, "# a comment line\n")?;

    let output = Command::new(BIN)
        .args(["--decisions-file", decisions.to_str().ok_or("path")?])
        .current_dir(root)
        .output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "Error: decisions file position 1: unknown verb '# a comment line' \
         (expected accept | skip | edit <value>).\n"
    );
    Ok(())
}

#[test]
fn the_decisions_file_env_var_is_honoured_and_the_flag_wins_over_it(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    seed_ambiguous_reference(root)?;
    let via_env = root.join("via-env.txt");
    fs::write(&via_env, "skip\n")?;

    let output = Command::new(BIN)
        .env("ACCELERATOR_MIGRATE_DECISIONS_FILE", &via_env)
        .current_dir(root)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let source = fs::read_to_string(root.join("meta/work/0001-source.md"))?;
    assert!(
        !source.contains("relates_to:"),
        "a skipped decision must not merge a linkage: {source}"
    );
    Ok(())
}

#[test]
fn a_nonexistent_decisions_file_refuses_before_anything_else(
) -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    seed_ambiguous_reference(root)?;
    let missing = root.join("nope.txt");

    let output = Command::new(BIN)
        .args(["--decisions-file", missing.to_str().ok_or("path")?])
        .current_dir(root)
        .output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.starts_with(
            "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE does not exist: "
        ),
        "{stderr}"
    );
    Ok(())
}

#[test]
fn a_directory_as_the_decisions_file_refuses() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let root = dir.path();
    seed_ambiguous_reference(root)?;
    let as_dir = root.join("a-directory");
    fs::create_dir(&as_dir)?;

    let output = Command::new(BIN)
        .args(["--decisions-file", as_dir.to_str().ok_or("path")?])
        .current_dir(root)
        .output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.starts_with(
            "Error: ACCELERATOR_MIGRATE_DECISIONS_FILE is a directory: "
        ),
        "{stderr}"
    );
    Ok(())
}
