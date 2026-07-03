//! The clap inbound (driving) adapter for `version` — parses, renders the
//! [`VersionReport`], and drives the inbound port. No domain logic.

use clap::{Parser, Subcommand};

use crate::version::core::{ReportVersion, VersionReport};

/// The `accelerator` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// The built-in subcommands compiled into the launcher.
#[derive(Subcommand)]
pub enum Command {
    /// Print the version, commit SHA, build date, and target triple.
    Version,
}

/// Renders a [`VersionReport`] as the human-facing `version` output.
#[must_use]
pub fn render(report: &VersionReport) -> String {
    format!(
        "accelerator {}\ncommit: {}\nbuilt:  {}\ntarget: {}",
        report.version,
        report.commit_sha,
        report.build_date,
        report.target_triple,
    )
}

/// Drives the inbound port for the parsed command and prints the result.
///
/// # Errors
///
/// Returns [`kernel::Error`] to share one fallible contract across
/// subcommands; the `version` arm cannot fail — the reachable error is the
/// composition root's logging init.
pub fn dispatch(
    cli: &Cli,
    reporter: &impl ReportVersion,
) -> Result<(), kernel::Error> {
    match &cli.command {
        Command::Version => {
            tracing::debug!("reporting version");
            println!("{}", render(&reporter.report()));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{dispatch, render, Cli, Command};
    use crate::version::core::{ReportVersion, VersionReport};

    fn sample_report() -> VersionReport {
        VersionReport {
            version: "1.2.3".to_owned(),
            commit_sha: "abc123".to_owned(),
            build_date: "2020-01-02T03:04:05Z".to_owned(),
            target_triple: "x86_64-unknown-linux-gnu".to_owned(),
        }
    }

    struct FakeReporter;

    impl ReportVersion for FakeReporter {
        fn report(&self) -> VersionReport {
            sample_report()
        }
    }

    #[test]
    fn render_produces_four_prefixed_lines_in_order() {
        assert_eq!(
            render(&sample_report()),
            "accelerator 1.2.3\n\
             commit: abc123\n\
             built:  2020-01-02T03:04:05Z\n\
             target: x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn dispatch_version_succeeds() {
        let cli = Cli {
            command: Command::Version,
        };
        assert!(dispatch(&cli, &FakeReporter).is_ok());
    }
}
