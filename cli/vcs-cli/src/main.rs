//! `accelerator-vcs` — the `vcs detect|status|log|guard` sub-binary,
//! dispatched by the `accelerator` launcher.

mod cli;
mod detect;

use std::process::ExitCode;

use clap::Parser as _;
use vcs_adapters::library::InProcessProbe;

use crate::cli::Cli;
use crate::cli::Command;

fn current_dir() -> Result<std::path::PathBuf, kernel::Error> {
    std::env::current_dir().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        ))
    })
}

fn run_detect(descriptive: bool, fail_safe: bool) -> Result<(), kernel::Error> {
    let start = current_dir()?;
    let probe = InProcessProbe;
    if let Some(output) = detect::run(&start, &probe, descriptive, fail_safe)? {
        println!("{output}");
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
    let result = match cli.command {
        Command::Detect {
            format: _,
            descriptive,
            fail_safe,
        } => run_detect(descriptive, fail_safe),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}
