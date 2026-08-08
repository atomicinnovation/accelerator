//! Freezes `accelerator-work`'s whole CLI surface: every `Command`
//! variant's flag names, arity, and help text, against one committed
//! golden. A future accidental change to any command's flags becomes a
//! visible, deliberate diff instead of a silent break for downstream
//! consumers to discover later.

use std::fmt::Write as _;
use std::process::Command;

type TestError = Box<dyn std::error::Error>;

const SUBCOMMANDS: &[&str] = &[
    "resolve",
    "template-hints",
    "show",
    "diff",
    "create",
    "update",
    "canonicalise-id",
    "next-number",
];

fn help(args: &[&str]) -> Result<String, TestError> {
    let output = Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(args)
        .arg("--help")
        .output()?;
    assert!(output.status.success(), "{args:?} --help did not exit 0");
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn the_full_cli_surface_matches_the_committed_golden() -> Result<(), TestError>
{
    let mut actual =
        format!("=== accelerator-work --help ===\n{}\n", help(&[])?);
    for subcommand in SUBCOMMANDS {
        write!(
            actual,
            "=== accelerator-work {subcommand} --help ===\n{}\n",
            help(&[subcommand])?
        )?;
    }

    let golden_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cli_surface.golden"
    );
    let golden = std::fs::read_to_string(golden_path)?;
    assert_eq!(
        actual, golden,
        "the CLI surface changed — update {golden_path} deliberately if \
         this change is intended"
    );
    Ok(())
}
