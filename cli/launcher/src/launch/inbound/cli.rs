//! The clap inbound (driving) adapter for the whole launcher surface — the
//! top-level command tree.
//!
//! Built-in commands and the external-subcommand growth mechanism live together
//! here; the per-command logic lives in its subdomain.

use std::ffi::OsString;

use clap::{Parser, Subcommand};

/// The `accelerator` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// The launcher's command tree: compiled-in built-ins plus the
/// `external_subcommand` catch-all that grows the CLI with on-demand
/// sub-binaries.
#[derive(Subcommand)]
pub enum Command {
    /// Print the version, commit SHA, build date, and target triple.
    Version,
    /// Any unknown subcommand + its args, forwarded verbatim. `Vec<OsString>`
    /// (not `Vec<String>`) preserves non-UTF-8 arguments through to the child.
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::ffi::OsString;

    use clap::Parser as _;

    use super::{Cli, Command};

    #[test]
    fn an_unknown_subcommand_routes_to_external_with_its_args(
    ) -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from(["accelerator", "frobnicate", "--flag"])?;
        match cli.command {
            Command::External(raw) => assert_eq!(
                raw,
                vec![OsString::from("frobnicate"), OsString::from("--flag")]
            ),
            Command::Version => return Err("routed to Version".into()),
        }
        Ok(())
    }

    #[test]
    fn a_known_subcommand_routes_to_its_builtin() -> Result<(), Box<dyn Error>>
    {
        // The built-in/external boundary: `version` is matched by clap to its
        // built-in variant and never captured by the external catch-all, so a
        // fetched sub-binary named `version` can never shadow it. Adding or
        // removing a built-in without updating this guard fails the test.
        let cli = Cli::try_parse_from(["accelerator", "version"])?;
        assert!(matches!(cli.command, Command::Version));
        Ok(())
    }
}
