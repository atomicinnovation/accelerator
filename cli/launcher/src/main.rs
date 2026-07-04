//! The accelerator launcher binary — the composition root: it installs the TLS
//! crypto provider, initialises logging, wires the concrete adapters to the
//! ports, parses the CLI, and dispatches (built-ins in-process, external
//! subcommands via resolve + exec).

use std::process::ExitCode;

use clap::Parser as _;

use launcher::launch::dispatch;
use launcher::launch::inbound::cli::Cli;
use launcher::launch::outbound::exec::UnixExec;
use launcher::launch::outbound::resolver::EnvOverrideResolver;
use launcher::launch::outbound::tls::install_crypto_provider;
use launcher::version::core::VersionReporter;
use launcher::version::outbound::build_metadata::VergenBuildMetadata;

fn run(cli: &Cli) -> Result<(), kernel::Error> {
    kernel::logging::init()?;
    let reporter = VersionReporter::new(VergenBuildMetadata);
    let resolver = EnvOverrideResolver;
    let executor = UnixExec;
    dispatch(cli, &reporter, &resolver, &executor)
}

fn main() -> ExitCode {
    if let Err(error) = install_crypto_provider() {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
