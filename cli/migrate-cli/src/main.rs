//! `accelerator-migrate` — applies pending meta-directory schema migrations,
//! dispatched by the `accelerator` launcher.

mod cli;

use std::process::ExitCode;

use clap::Parser as _;

use crate::cli::Cli;

/// The state-mutating and enumeration flags all route through machinery later
/// phases add (the ledger, the interactive engine); with an empty registry
/// there is nothing yet for any of them to do, so each names itself rather
/// than silently behaving like a default run.
fn run(cli: &Cli) -> Result<(), kernel::Error> {
    if cli.skip.is_some()
        || cli.unskip.is_some()
        || cli.unapply.is_some()
        || cli.list
        || cli.decisions_file.is_some()
    {
        return Err(kernel::Error::Failed("not yet implemented".to_owned()));
    }
    if migrate::registry::registry().is_empty() {
        println!("No pending migrations.");
    }
    Ok(())
}

fn report(error: &kernel::Error) -> ExitCode {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("{message}");
    }
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}
