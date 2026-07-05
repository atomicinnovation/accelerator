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

/// Route the parsed command: built-ins run in-process, an external subcommand
/// resolves and execs (replacing this process on success).
///
/// # Errors
///
/// A [`kernel::Error`] when an external subcommand cannot be resolved or exec'd.
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
            // A successful exec never returns, so reaching here is always error.
            Err(run_external(resolver, executor, &command).into())
        }
    }
}
