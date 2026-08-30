//! CLI-boundary tests for `work link-external-id`.

use std::fs;
use std::path::Path;
use std::process::Command;

use document::Scalar;
use document::Yaml;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-link-external-id-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    Ok(dir)
}

fn write_fixture(
    dir: &Path,
    name: &str,
    content: &str,
) -> Result<std::path::PathBuf, TestError> {
    let path = dir.join(name);
    fs::write(&path, content)?;
    Ok(path)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    Ok(Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .arg("link-external-id")
        .args(args)
        .current_dir(dir)
        .output()?)
}

fn external_id(content: &str) -> Result<Option<Yaml>, TestError> {
    let yaml = document::parse(content)?;
    let Yaml::Mapping(mapping) = yaml else {
        return Err("expected a mapping".into());
    };
    Ok(mapping.get("external_id").cloned())
}

#[test]
fn insert_writes_the_scalar_and_leaves_everything_else_byte_identical(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let fixture =
        "---\ntitle: \"Test\"\nstatus: \"draft\"\npriority: \"medium\"\n---\nbody\n";
    let path = write_fixture(repo.path(), "f1.md", fixture)?;

    let output =
        run(repo.path(), &[path.to_str().ok_or("non-utf8")?, "PP-195"])?;
    assert!(output.status.success(), "{output:?}");

    let content = fs::read_to_string(&path)?;
    assert_eq!(
        external_id(&content)?,
        Some(Yaml::Scalar(Scalar::String("PP-195".to_owned())))
    );
    let without = content.replace("external_id: \"PP-195\"\n", "");
    assert_eq!(
        without, fixture,
        "only the external_id scalar should differ from the original"
    );
    Ok(())
}

#[test]
fn overwrite_replaces_an_existing_external_id_only() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let fixture =
        "---\ntitle: \"Test\"\nexternal_id: \"OLD-1\"\nstatus: \"draft\"\n---\nbody\n";
    let path = write_fixture(repo.path(), "f1.md", fixture)?;

    let output =
        run(repo.path(), &[path.to_str().ok_or("non-utf8")?, "PP-195"])?;
    assert!(output.status.success(), "{output:?}");

    let content = fs::read_to_string(&path)?;
    assert_eq!(
        external_id(&content)?,
        Some(Yaml::Scalar(Scalar::String("PP-195".to_owned())))
    );
    let restored = content
        .replace("external_id: \"PP-195\"\n", "external_id: \"OLD-1\"\n");
    assert_eq!(
        restored, fixture,
        "only the external_id value should differ from the original"
    );
    Ok(())
}

#[test]
fn non_mapping_frontmatter_exits_non_zero_with_a_message(
) -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let fixture = "---\n- just\n- a\n- sequence\n---\nbody\n";
    let path = write_fixture(repo.path(), "f1.md", fixture)?;

    let output =
        run(repo.path(), &[path.to_str().ok_or("non-utf8")?, "PP-195"])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.is_empty(), "an error message is expected on stderr");
    let unchanged = fs::read_to_string(&path)?;
    assert_eq!(
        unchanged, fixture,
        "a rejected write must not touch the file"
    );
    Ok(())
}

#[test]
fn a_missing_file_exits_non_zero() -> Result<(), TestError> {
    let repo = scratch_repo()?;
    let missing = repo.path().join("does-not-exist.md");
    let output = run(
        repo.path(),
        &[missing.to_str().ok_or("non-utf8")?, "PP-195"],
    )?;
    assert_eq!(output.status.code(), Some(1));
    Ok(())
}
