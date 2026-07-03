//! The accelerator launcher binary — the composition root: it initialises
//! logging, builds the vergen adapter, injects it into the `version` core,
//! parses the CLI, and dispatches.

use std::process::ExitCode;

use clap::Parser;

use launcher::version::core::VersionReporter;
use launcher::version::inbound::cli::{dispatch, Cli};
use launcher::version::outbound::build_metadata::VergenBuildMetadata;

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    kernel::logging::init()?;
    let reporter = VersionReporter::new(VergenBuildMetadata);
    dispatch(cli, &reporter)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
