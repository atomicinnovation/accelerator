use std::process::Command;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-vcs");

#[test]
fn the_retired_tracking_subcommand_is_unrecognised() -> Result<(), TestError> {
    let work = tempfile::Builder::new()
        .prefix("vcs-retired-tracking-")
        .tempdir()?;

    let output = Command::new(BIN)
        .current_dir(work.path())
        .arg("tracking")
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "error: unrecognized subcommand 'tracking'\n\n\
         Usage: accelerator-vcs <COMMAND>\n\n\
         For more information, try '--help'.\n"
    );
    Ok(())
}
