//! An insecure `config.local.md` through the binary: readers run on team
//! values with one warning, and every writer refuses and writes nothing.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

const ITEM: &str = "---\ntitle: Test\nstatus: draft\n---\nbody\n";

fn insecure_repo() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-insecure-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        "---\nwork:\n  integration: jira\n---\n",
    )?;
    let personal = dir.path().join(".accelerator/config.local.md");
    fs::write(&personal, "---\nwork:\n  integration: linear\n---\n")?;
    fs::set_permissions(&personal, fs::Permissions::from_mode(0o644))?;
    fs::write(dir.path().join("meta/work/0001-test.md"), ITEM)?;
    Ok(dir)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    common::scrub_provider_env(&mut command);
    Ok(command.output()?)
}

fn work_items(dir: &Path) -> Result<Vec<String>, TestError> {
    let mut names = fs::read_dir(dir.join("meta/work"))?
        .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;
    names.sort();
    Ok(names)
}

#[test]
fn every_writer_refuses_and_writes_nothing() -> Result<(), TestError> {
    let cases: [&[&str]; 4] = [
        &["create", "Test item", "task", "medium"],
        &["update", "meta/work/0001-test.md", "--set", "status=ready"],
        &["sync", "--push-only"],
        &["sync", "--pull-only"],
    ];
    for args in cases {
        let repo = insecure_repo()?;

        let output = run(repo.path(), args)?;

        let stderr = String::from_utf8(output.stderr)?;
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stderr}");
        assert!(
            stderr.contains("E_LOCAL_PERMS_INSECURE"),
            "{args:?}: {stderr}"
        );
        assert_eq!(work_items(repo.path())?, ["0001-test.md"], "{args:?}");
        assert_eq!(
            fs::read_to_string(repo.path().join("meta/work/0001-test.md"))?,
            ITEM,
            "{args:?}"
        );
    }
    Ok(())
}

#[test]
fn a_reader_runs_on_team_values_with_one_warning() -> Result<(), TestError> {
    let repo = insecure_repo()?;

    let output = run(repo.path(), &["list"])?;

    let stderr = String::from_utf8(output.stderr)?;
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        stderr.matches("warning: E_LOCAL_PERMS_INSECURE").count(),
        1,
        "{stderr}"
    );
    Ok(())
}
