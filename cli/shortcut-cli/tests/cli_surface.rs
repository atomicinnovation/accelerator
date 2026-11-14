//! Freezes the whole `--help` surface against one committed golden, so a
//! flag rename is a visible diff.

#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

const COMMANDS: &[&[&str]] = &[
    &[],
    &["show"],
    &["search"],
    &["create"],
    &["update"],
    &["comment"],
    &["comment", "add"],
    &["transition"],
    &["attach"],
    &["init"],
    &["init", "verify"],
    &["init", "list-teams"],
    &["init", "discover"],
];

fn help(args: &[&str]) -> Result<String, TestError> {
    let output = Command::new(env!("CARGO_BIN_EXE_accelerator-shortcut"))
        .args(args)
        .arg("--help")
        .output()?;
    assert!(output.status.success(), "{args:?} --help did not exit 0");
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn the_full_cli_surface_matches_the_committed_golden() -> Result<(), TestError>
{
    let mut actual = String::new();
    for args in COMMANDS {
        let name = std::iter::once("accelerator-shortcut")
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        write!(actual, "=== {name} --help ===\n{}\n", help(args)?)?;
    }
    let golden_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cli_surface.golden"
    );
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(golden_path, &actual)?;
        return Ok(());
    }
    let golden = std::fs::read_to_string(golden_path)?;
    assert_eq!(
        actual, golden,
        "the CLI surface changed; re-run with UPDATE_GOLDEN=1 to accept"
    );
    Ok(())
}
