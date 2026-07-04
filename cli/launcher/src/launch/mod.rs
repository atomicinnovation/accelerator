//! The dispatch boundary: routes the parsed command tree to a built-in handler
//! or to external resolution + exec.

pub mod core;
pub mod help;
pub mod inbound;
pub mod outbound;

use crate::launch::core::{
    run_external, ExecBinary, ExternalCommand, ResolveBinary,
};
use crate::launch::inbound::cli::{Cli, Command};
use crate::version::core::ReportVersion;
use crate::version::inbound::cli as version_cli;

/// Route the parsed command: built-ins run in-process; an external subcommand
/// resolves + execs (replacing this process on success).
///
/// The built-in set is matched first (clap routes `version` to its own variant,
/// never to `External`), so a fetched sub-binary can never shadow a built-in.
///
/// # Errors
///
/// A [`kernel::Error`] when an external subcommand cannot be resolved or exec'd.
/// The `Version` built-in never fails. A successful external exec never returns.
pub fn dispatch(
    cli: &Cli,
    reporter: &impl ReportVersion,
    resolver: &impl ResolveBinary,
    executor: &impl ExecBinary,
) -> Result<(), kernel::Error> {
    match &cli.command {
        Command::Version => {
            version_cli::report(reporter);
            Ok(())
        }
        Command::External(raw) => {
            let command = ExternalCommand::from_raw(raw.clone())?;
            // run_external only returns on failure — a successful exec replaces
            // the process — so reaching here always means an error.
            Err(run_external(resolver, executor, &command).into())
        }
    }
}
