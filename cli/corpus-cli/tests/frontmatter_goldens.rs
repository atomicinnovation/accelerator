//! `accelerator-corpus frontmatter validate` black-box CLI coverage: target-
//! set resolution, `--checks` gating, and the exit-code contract, plus the
//! unconditional whole-corpus self-check that is now this validator's
//! required-by-name completion gate.
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

fn tempdir(tag: &str) -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::Builder::new()
        .prefix(&format!("corpus-frontmatter-golden-{tag}-"))
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

fn run(dir: &Path, args: &[&str]) -> Result<Output, TestError> {
    Ok(Command::new(BIN).current_dir(dir).args(args).output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn write_work_item(path: &Path, id: &str) -> Result<(), TestError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        format!(
            "---\ntype: \"work-item\"\nid: \"{id}\"\ntitle: \"t\"\n\
             date: \"2026-01-01T00:00:00Z\"\nauthor: \"a\"\ntags: []\n\
             last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: \"a\"\n\
             schema_version: 1\nstatus: \"draft\"\nkind: \"task\"\n\
             priority: \"normal\"\n---\nbody\n"
        ),
    )?;
    Ok(())
}

#[test]
fn a_clean_whole_corpus_default_run_exits_0() -> Result<(), TestError> {
    let dir = tempdir("default-clean")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    write_work_item(&root.join("meta/work/0001.md"), "0001")?;

    let output = run(&root, &["frontmatter", "validate"])?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn a_violation_in_the_default_whole_corpus_exits_1() -> Result<(), TestError> {
    let dir = tempdir("default-dirty")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("meta/work/0001.md");
    fs::create_dir_all(file.parent().ok_or("no parent")?)?;
    fs::write(&file, "---\ntype: bogus\n---\nbody\n")?;

    let output = run(&root, &["frontmatter", "validate"])?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("INVALID-TYPE"));
    Ok(())
}

#[test]
fn a_bare_string_field_exits_1_and_names_unquoted_string(
) -> Result<(), TestError> {
    let dir = tempdir("unquoted-signal")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("meta/work/0001.md");
    write_work_item(&file, "0001")?;
    let content = fs::read_to_string(&file)?;
    fs::write(&file, content.replace("author: \"a\"", "author: a"))?;

    let output = run(
        &root,
        &[
            "frontmatter",
            "validate",
            "--file",
            file.to_str().ok_or("non-utf8")?,
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("UNQUOTED-STRING"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn dir_only_scopes_the_walk() -> Result<(), TestError> {
    let dir = tempdir("dir-only")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    write_work_item(&root.join("meta/work/0001.md"), "0001")?;
    fs::create_dir_all(root.join("meta/decisions"))?;
    fs::write(
        root.join("meta/decisions/ADR-0001-bad.md"),
        "---\ntype: bogus\n---\nbody\n",
    )?;

    let output =
        run(&root, &["frontmatter", "validate", "--dir", "meta/work"])?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn dir_walking_skips_files_outside_every_configured_doc_type_directory(
) -> Result<(), TestError> {
    let dir = tempdir("dir-out-of-scope")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    write_work_item(&root.join("meta/work/0001.md"), "0001")?;
    fs::create_dir_all(root.join("meta/docs"))?;
    fs::write(
        root.join("meta/docs/guide.md"),
        "---\ntitle: Guide\nfoo: bar\n---\nbody\n",
    )?;

    let output = run(&root, &["frontmatter", "validate", "--dir", "meta"])?;
    assert!(
        output.status.success(),
        "meta/docs/ is not a configured doc-type directory and must be \
         skipped, matching the retired bash validator's out_of_scope gate: {}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn file_only_validates_only_the_named_file() -> Result<(), TestError> {
    let dir = tempdir("file-only")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    write_work_item(&root.join("meta/work/0001.md"), "0001")?;
    fs::create_dir_all(root.join("meta/decisions"))?;
    fs::write(
        root.join("meta/decisions/ADR-0001-bad.md"),
        "---\ntype: bogus\n---\nbody\n",
    )?;

    let output = run(
        &root,
        &["frontmatter", "validate", "--file", "meta/work/0001.md"],
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn dir_and_file_together_validate_the_union() -> Result<(), TestError> {
    let dir = tempdir("dir-and-file")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    write_work_item(&root.join("meta/work/0001.md"), "0001")?;
    fs::create_dir_all(root.join("meta/decisions"))?;
    fs::write(
        root.join("meta/decisions/ADR-0001-bad.md"),
        "---\ntype: bogus\n---\nbody\n",
    )?;

    let output = run(
        &root,
        &[
            "frontmatter",
            "validate",
            "--dir",
            "meta/work",
            "--file",
            "meta/decisions/ADR-0001-bad.md",
        ],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("INVALID-TYPE"));
    Ok(())
}

#[test]
fn checks_structure_only_omits_dangling_ref_violations() -> Result<(), TestError>
{
    let dir = tempdir("checks-structure")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("meta/work/0001.md");
    fs::create_dir_all(file.parent().ok_or("no parent")?)?;
    fs::write(
        &file,
        "---\ntype: \"work-item\"\nid: \"0001\"\ntitle: \"t\"\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: \"a\"\ntags: []\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: \"a\"\n\
         schema_version: 1\nstatus: \"draft\"\nkind: \"task\"\n\
         priority: \"normal\"\nparent: \"work-item:9999\"\n---\nbody\n",
    )?;

    let output =
        run(&root, &["frontmatter", "validate", "--checks", "structure"])?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn checks_references_only_still_flags_a_dangling_ref() -> Result<(), TestError>
{
    let dir = tempdir("checks-references")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("meta/work/0001.md");
    fs::create_dir_all(file.parent().ok_or("no parent")?)?;
    fs::write(
        &file,
        "---\ntype: \"work-item\"\nid: \"0001\"\ntitle: \"t\"\n\
         date: \"2026-01-01T00:00:00Z\"\nauthor: \"a\"\ntags: []\n\
         last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: \"a\"\n\
         schema_version: 1\nstatus: \"draft\"\nkind: \"task\"\n\
         priority: \"normal\"\nparent: \"work-item:9999\"\n---\nbody\n",
    )?;

    let output = run(
        &root,
        &["frontmatter", "validate", "--checks", "references"],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("DANGLING-REF"));
    Ok(())
}

#[test]
fn checks_structure_and_references_explicit_runs_both() -> Result<(), TestError>
{
    let dir = tempdir("checks-both")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    write_work_item(&root.join("meta/work/0001.md"), "0001")?;

    let output = run(
        &root,
        &[
            "frontmatter",
            "validate",
            "--checks",
            "structure,references",
        ],
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn checks_references_only_against_a_fenceless_file_is_skipped_not_clean(
) -> Result<(), TestError> {
    let dir = tempdir("skipped")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("meta/work/0001.md");
    fs::create_dir_all(file.parent().ok_or("no parent")?)?;
    fs::write(&file, "no frontmatter here\n")?;

    let output = run(
        &root,
        &["frontmatter", "validate", "--checks", "references"],
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("SKIPPED"),
        "a references-only run over a broken file must be distinguishable \
         from a genuinely clean corpus: {}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_nonexistent_file_argument_reports_no_fence_not_a_panic(
) -> Result<(), TestError> {
    let dir = tempdir("missing-file")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;

    let output = run(
        &root,
        &["frontmatter", "validate", "--file", "meta/work/missing.md"],
    )?;
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("NO-FENCE"));
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
    write_work_item(&root.join("tickets/0001.md"), "0001")?;

    let output = run(&root, &["frontmatter", "validate"])?;
    assert!(
        output.status.success(),
        "the configured 'tickets' directory must be walked by default: {}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn this_repositorys_own_corpus_is_clean() -> Result<(), TestError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;

    let output = run(&root, &["frontmatter", "validate"])?;
    assert!(
        output.status.success(),
        "this repository's own meta/ tree must carry zero frontmatter \
         violations: {}",
        stderr(&output)
    );
    Ok(())
}

/// The committed hand-authored topic-research set — the shared artifact the
/// 0278 indexer keys on, validated here independent of live web and the
/// co-land.
fn topic_research_set() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/topic-research-set")
}

fn validate_content(tag: &str, content: &str) -> Result<Output, TestError> {
    let dir = tempdir(tag)?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let file = root.join("doc.md");
    fs::write(&file, content)?;
    run(
        &root,
        &[
            "frontmatter",
            "validate",
            "--file",
            file.to_str().ok_or("non-utf8")?,
        ],
    )
}

#[test]
fn the_committed_topic_research_set_validates_clean() -> Result<(), TestError> {
    let set = topic_research_set();
    let files = [
        set.join("manifest.md"),
        set.join("brief.md"),
        set.join("outline.md"),
        set.join("findings/01-example-focus.md"),
        set.join("synthesis.md"),
    ];
    let dir = tempdir("topic-set-clean")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let mut args = vec!["frontmatter".to_owned(), "validate".to_owned()];
    for file in &files {
        args.push("--file".to_owned());
        args.push(file.to_str().ok_or("non-utf8")?.to_owned());
    }
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run(&root, &borrowed)?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn a_manifest_missing_primary_is_a_missing_extra() -> Result<(), TestError> {
    let content = fs::read_to_string(topic_research_set().join("manifest.md"))?;
    let mutated: String = content
        .lines()
        .filter(|line| !line.starts_with("primary:"))
        .collect::<Vec<_>>()
        .join("\n");
    let output = validate_content("topic-missing-primary", &mutated)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("MISSING-EXTRA"),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).contains("primary"), "{}", stderr(&output));
    Ok(())
}

#[test]
fn a_brief_missing_source_profiles_is_a_missing_extra() -> Result<(), TestError>
{
    let content = fs::read_to_string(topic_research_set().join("brief.md"))?;
    let mutated: String = content
        .lines()
        .filter(|line| !line.starts_with("source_profiles:"))
        .collect::<Vec<_>>()
        .join("\n");
    let output = validate_content("topic-missing-profiles", &mutated)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("MISSING-EXTRA"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_topic_research_document_with_a_bogus_kind_is_unknown_kind(
) -> Result<(), TestError> {
    let content = fs::read_to_string(topic_research_set().join("manifest.md"))?;
    let mutated = content.replace("kind: \"manifest\"", "kind: \"bogus\"");
    let output = validate_content("topic-bogus-kind", &mutated)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("UNKNOWN-KIND"),
        "a valid type carrying an unmatched kind must be UNKNOWN-KIND, not \
         INVALID-TYPE: {}",
        stderr(&output)
    );
    assert!(
        !stderr(&output).contains("INVALID-TYPE"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_topic_research_document_missing_its_kind_is_unknown_kind(
) -> Result<(), TestError> {
    let content = fs::read_to_string(topic_research_set().join("manifest.md"))?;
    let mutated: String = content
        .lines()
        .filter(|line| !line.starts_with("kind:"))
        .collect::<Vec<_>>()
        .join("\n");
    let output = validate_content("topic-missing-kind", &mutated)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("UNKNOWN-KIND"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_finding_with_a_draft_status_is_rejected_but_complete_passes(
) -> Result<(), TestError> {
    let complete = fs::read_to_string(
        topic_research_set().join("findings/01-example-focus.md"),
    )?;
    let pass = validate_content("topic-finding-complete", &complete)?;
    assert!(pass.status.success(), "{}", stderr(&pass));

    let draft = complete.replace("status: \"complete\"", "status: \"draft\"");
    let output = validate_content("topic-finding-draft", &draft)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("BAD-STATUS"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_manifest_at_the_briefed_lifecycle_start_validates() -> Result<(), TestError>
{
    let content = fs::read_to_string(topic_research_set().join("manifest.md"))?;
    let briefed =
        content.replace("status: \"synthesised\"", "status: \"briefed\"");
    let output = validate_content("topic-status-briefed", &briefed)?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn a_manifest_carrying_the_retired_research_status_is_obsolete_legacy_key(
) -> Result<(), TestError> {
    let content = fs::read_to_string(topic_research_set().join("manifest.md"))?;
    let mutated = content.replace(
        "status: \"synthesised\"\n",
        "status: \"synthesised\"\nresearch_status: \"synthesised\"\n",
    );
    let output = validate_content("topic-research-status-retired", &mutated)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("OBSOLETE-LEGACY-KEY"),
        "{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("research_status"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn a_manifest_with_an_out_of_vocab_base_status_is_bad_status(
) -> Result<(), TestError> {
    let content = fs::read_to_string(topic_research_set().join("manifest.md"))?;
    let mutated =
        content.replace("status: \"synthesised\"", "status: \"bogus\"");
    let output = validate_content("topic-status-out-of-vocab", &mutated)?;
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("BAD-STATUS"),
        "{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn print_schema_emits_the_three_banks() -> Result<(), TestError> {
    let dir = tempdir("print-schema")?;
    let root = canonical_root(&dir)?;
    repo(&root)?;
    let output = run(&root, &["frontmatter", "print-schema"])?;
    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("\"base_fields\":[\"type\""), "{stdout}");
    assert!(
        stdout.contains("\"provenance_fields\":[\"revision\""),
        "{stdout}"
    );
    assert!(stdout.contains("\"optional_extras\":"), "{stdout}");
    Ok(())
}
