//! Shells the live `work-item-normalise.sh`/`work-item-project-remote.sh`
//! for each corpus case and asserts both the Rust recipe and the committed
//! `expected.json` match what bash actually outputs right now.
//!
//! Without this, the corpus's bash provenance rests on a manual step: after
//! `regenerate.sh` is deleted at cutover, nothing would otherwise catch a
//! corpus quietly regenerated from the Rust recipe, where the oracle would
//! agree with itself and the whole test become vacuous. Gated on
//! `bash-parity`, which `--all-features` turns on in CI, matching every
//! other bash-shellout suite in this crate.
#![cfg(feature = "bash-parity")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use serde_json::Value;
use work_adapters::project_remote;
use work_adapters::sync::digest;

type TestError = Box<dyn std::error::Error>;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn corpus_root() -> Result<PathBuf, TestError> {
    Ok(repo_root()?
        .join("skills/work/scripts/test-fixtures/work-item-sync-baseline"))
}

fn scripts_dir() -> Result<PathBuf, TestError> {
    Ok(repo_root()?.join("skills/work/scripts"))
}

fn case_dirs() -> Result<Vec<PathBuf>, TestError> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(corpus_root()?)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .is_some_and(|name| name.starts_with("case-"))
        })
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn hash_sha256_stdin(input: &str) -> Result<String, TestError> {
    let mut child = Command::new("bash")
        .arg("-c")
        .arg("sha256sum 2>/dev/null || shasum -a 256")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(input.as_bytes())?;
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout.split_whitespace().next().unwrap_or("").to_owned())
}

fn run_normalise(
    scripts: &Path,
    args: &[&str],
    stdin: Option<&str>,
) -> Result<Option<String>, TestError> {
    let mut command = Command::new("bash");
    command.arg(scripts.join("work-item-normalise.sh"));
    command.args(args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    if let Some(input) = stdin {
        command.stdin(Stdio::piped());
        let mut child = command.spawn()?;
        child
            .stdin
            .take()
            .expect("stdin piped")
            .write_all(input.as_bytes())?;
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8(output.stdout)?))
    } else {
        let output = command.output()?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8(output.stdout)?))
    }
}

fn run_project(
    scripts: &Path,
    integration: &str,
    op: &str,
    stdin: &str,
) -> Result<String, TestError> {
    let mut child = Command::new("bash")
        .arg(scripts.join("work-item-project-remote.sh"))
        .arg("--integration")
        .arg(integration)
        .arg(op)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(stdin.as_bytes())?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "work-item-project-remote.sh failed"
    );
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn every_case_matches_live_bash_right_now() -> Result<(), TestError> {
    let scripts = scripts_dir()?;

    for dir in case_dirs()? {
        let name = dir
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .expect("case has a name")
            .to_owned();
        let expected: Value = serde_json::from_str(&std::fs::read_to_string(
            dir.join("expected.json"),
        )?)?;

        if let Some(local_path) =
            Some(dir.join("local.md")).filter(|p| p.exists())
        {
            let live =
                run_normalise(&scripts, &[local_path.to_str().unwrap()], None)?;
            match live {
                None => {
                    assert_eq!(
                        expected["error"].as_bool(),
                        Some(true),
                        "case {name}: bash failed live but expected.json carries a hash"
                    );
                }
                Some(normalised) => {
                    let live_hash = hash_sha256_stdin(&normalised)?;
                    let expected_hash = expected["local_hash"]
                        .as_str()
                        .unwrap_or_else(|| panic!("case {name}: bash succeeded live but expected.json records an error"));
                    assert_eq!(
                        live_hash, expected_hash,
                        "case {name}: live bash vs expected.json"
                    );

                    let content = std::fs::read_to_string(&local_path)?;
                    let rust_hash = digest::local(&content)?;
                    assert_eq!(
                        rust_hash, live_hash,
                        "case {name}: rust vs live bash"
                    );
                }
            }
            continue;
        }

        let remote_path = dir.join("remote.json");
        if !remote_path.exists() {
            continue;
        }
        let integration = expected["integration"]
            .as_str()
            .unwrap_or_else(|| panic!("case {name} names no integration"));
        let remote_content = std::fs::read_to_string(&remote_path)?;

        let live_updated =
            run_project(&scripts, integration, "updated", &remote_content)?
                .trim_end()
                .to_owned();
        assert_eq!(
            live_updated,
            expected["updated"]
                .as_str()
                .expect("expected.updated present"),
            "case {name}: live bash updated vs expected.json"
        );

        let live_body =
            run_project(&scripts, integration, "body", &remote_content)?;
        let normalised =
            run_normalise(&scripts, &["--stdin"], Some(&live_body))?.expect(
                "normalise --stdin never fails on a project_remote body",
            );
        let live_remote_hash = hash_sha256_stdin(&normalised)?;
        let expected_remote_hash = expected["remote_hash"]
            .as_str()
            .expect("expected.remote_hash present");
        assert_eq!(
            live_remote_hash, expected_remote_hash,
            "case {name}: live bash remote_hash vs expected.json"
        );

        let remote_json: Value = serde_json::from_str(&remote_content)?;
        let parsed_integration = project_remote::parse_integration(integration)
            .expect("recognised integration");
        let rust_body = project_remote::project(
            parsed_integration,
            project_remote::Op::Body,
            &remote_json,
        );
        let rust_remote_hash = digest::remote_body(&rust_body);
        assert_eq!(
            rust_remote_hash, live_remote_hash,
            "case {name}: rust vs live bash"
        );
    }
    Ok(())
}
