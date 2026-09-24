//! Black-box tests that `config`'s help lists every recognised key.

use std::error::Error;
use std::process::Command;

use config::catalogue;

const LAUNCHER: &str = env!("CARGO_BIN_EXE_accelerator");

const DEAD_RELEASE_URL: &str = "https://127.0.0.1:1";

fn recognised_keys() -> Vec<String> {
    let defaulted = [
        catalogue::PATH_KEYS,
        catalogue::WORK_KEYS,
        catalogue::REVIEW_KEYS,
        catalogue::RESEARCH_KEYS,
        catalogue::VISUALISER_KEYS,
    ]
    .into_iter()
    .flatten()
    .map(|(key, _)| (*key).to_owned());
    let agents = catalogue::AGENT_KEYS
        .iter()
        .map(|name| format!("agents.{name}"));
    let bare = catalogue::TEMPLATE_KEYS
        .iter()
        .chain(catalogue::EXTRA_KEYS)
        .map(|key| (*key).to_owned());
    defaulted.chain(agents).chain(bare).collect()
}

fn config_help_stdout(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(LAUNCHER)
        .args(args)
        .env("ACCELERATOR_RELEASE_BASE_URL", DEAD_RELEASE_URL)
        .env_remove("ACCELERATOR_LOG")
        .output()?;
    assert!(output.status.success(), "{args:?} did not exit 0");
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn every_help_form_lists_every_recognised_key() -> Result<(), Box<dyn Error>> {
    for args in [["config", "help"], ["config", "--help"]] {
        let stdout = config_help_stdout(&args)?;
        assert!(
            stdout.contains("Recognised keys:"),
            "{args:?} has no recognised-keys block: {stdout}"
        );
        for key in recognised_keys() {
            assert!(
                stdout.lines().any(|line| line.trim() == key),
                "{args:?} does not list {key}: {stdout}"
            );
        }
    }
    Ok(())
}

#[test]
fn the_openalex_credential_keys_are_listed() -> Result<(), Box<dyn Error>> {
    let stdout = config_help_stdout(&["config", "help"])?;
    for key in ["openalex.api_key", "openalex.api_key_cmd"] {
        assert!(
            stdout.lines().any(|line| line.trim() == key),
            "{key} missing: {stdout}"
        );
    }
    Ok(())
}
