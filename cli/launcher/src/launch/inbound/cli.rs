//! The clap inbound adapter: the top-level `accelerator` command tree.

use std::ffi::OsString;

use clap::{Parser, Subcommand};

/// The `accelerator` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print the version, commit SHA, build date, and target triple.
    Version,
    /// Any unknown subcommand and its args, forwarded to the resolved binary.
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
        let cli = Cli::try_parse_from(["accelerator", "version"])?;
        assert!(matches!(cli.command, Command::Version));
        Ok(())
    }
}
