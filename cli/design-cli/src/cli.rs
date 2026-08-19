//! The clap inbound adapter: the `accelerator-design` command-line surface.

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;
use design::DowngradeReason;

/// The `accelerator-design` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator-design", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Whether a location may be inventoried.
    ValidateSource {
        /// The URL or repository path to inventory.
        location: String,
        /// Permit private, link-local and reserved hosts. Loopback needs no
        /// flag; the unspecified address is refused regardless.
        #[arg(long)]
        allow_internal: bool,
        /// Permit http:// to a public host.
        #[arg(long)]
        allow_insecure_scheme: bool,
    },
    /// The authentication mode the `ACCELERATOR_BROWSER_*` environment
    /// selects.
    ///
    /// The `header` mode is currently inert downstream: the daemon imports
    /// its handler and never calls it, and the origin allowlist that handler
    /// requires is set nowhere. Do not place a live credential in
    /// `ACCELERATOR_BROWSER_AUTH_HEADER` until that is wired up.
    ResolveAuth,
    /// Whether a produced artefact repeats a configured credential.
    ScrubSecrets {
        /// The artefact to scan.
        file: PathBuf,
    },
    /// The notice explaining a fallback to the code-only crawler.
    NotifyDowngrade {
        #[arg(long, value_enum)]
        reason: DowngradeReasonArg,
        /// Accepted for forward compatibility; not used in message
        /// selection.
        #[arg(long)]
        from: Option<String>,
        /// Accepted for forward compatibility; not used in message
        /// selection.
        #[arg(long)]
        to: Option<String>,
    },
    /// Runs a Playwright executor command against a reused or freshly-spawned
    /// daemon.
    ///
    /// Arguments after the command are forwarded to the Node runner verbatim,
    /// so the command itself is checked against an allowlist: the runner
    /// dispatches on the first of them, and its internal `daemon` subcommand
    /// would otherwise start a second foreground daemon over the live one's
    /// state.
    Executor {
        /// The executor command: ping, navigate, snapshot, screenshot,
        /// evaluate, links or daemon-stop.
        command: String,
        /// The command's own arguments, usually a single JSON object.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        arguments: Vec<String>,
    },
    /// Whether every substantive H2 section carries a cue phrase.
    AuditCuePhrases {
        /// The design-gap document to audit.
        file: PathBuf,
    },
}

/// The CLI-local mirror of `design::DowngradeReason`. The domain crate cannot
/// derive `ValueEnum`: its import rule permits only std, `kernel::Error` and
/// `crate`.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DowngradeReasonArg {
    UnsupportedPlatform,
    LoaderUnresolvable,
    GlibcTooOld,
    RuntimeLibrariesMissing,
    ArtifactUnavailable,
    MaterialisationInProgress,
    ExecutorPingFailed,
    CacheUnwritable,
    DiskFloorNotMet,
}

impl From<DowngradeReasonArg> for DowngradeReason {
    fn from(arg: DowngradeReasonArg) -> Self {
        match arg {
            DowngradeReasonArg::UnsupportedPlatform => {
                Self::UnsupportedPlatform
            }
            DowngradeReasonArg::LoaderUnresolvable => Self::LoaderUnresolvable,
            DowngradeReasonArg::GlibcTooOld => Self::GlibcTooOld,
            DowngradeReasonArg::RuntimeLibrariesMissing => {
                Self::RuntimeLibrariesMissing
            }
            DowngradeReasonArg::ArtifactUnavailable => {
                Self::ArtifactUnavailable
            }
            DowngradeReasonArg::MaterialisationInProgress => {
                Self::MaterialisationInProgress
            }
            DowngradeReasonArg::ExecutorPingFailed => Self::ExecutorPingFailed,
            DowngradeReasonArg::CacheUnwritable => Self::CacheUnwritable,
            DowngradeReasonArg::DiskFloorNotMet => Self::DiskFloorNotMet,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;
    use design::DowngradeReason;

    use super::Cli;
    use super::Command;
    use super::DowngradeReasonArg;

    #[test]
    fn each_argument_selects_its_own_reason() {
        for (arg, reason) in [
            (
                DowngradeReasonArg::UnsupportedPlatform,
                DowngradeReason::UnsupportedPlatform,
            ),
            (
                DowngradeReasonArg::LoaderUnresolvable,
                DowngradeReason::LoaderUnresolvable,
            ),
            (
                DowngradeReasonArg::GlibcTooOld,
                DowngradeReason::GlibcTooOld,
            ),
            (
                DowngradeReasonArg::RuntimeLibrariesMissing,
                DowngradeReason::RuntimeLibrariesMissing,
            ),
            (
                DowngradeReasonArg::ArtifactUnavailable,
                DowngradeReason::ArtifactUnavailable,
            ),
            (
                DowngradeReasonArg::MaterialisationInProgress,
                DowngradeReason::MaterialisationInProgress,
            ),
            (
                DowngradeReasonArg::ExecutorPingFailed,
                DowngradeReason::ExecutorPingFailed,
            ),
            (
                DowngradeReasonArg::CacheUnwritable,
                DowngradeReason::CacheUnwritable,
            ),
            (
                DowngradeReasonArg::DiskFloorNotMet,
                DowngradeReason::DiskFloorNotMet,
            ),
        ] {
            assert_eq!(DowngradeReason::from(arg), reason);
        }
    }

    /// The executor emits these keys verbatim, so each must remain the accepted
    /// spelling.
    #[test]
    fn every_reason_key_parses() -> Result<(), Box<dyn std::error::Error>> {
        for reason in DowngradeReason::ALL {
            let cli = Cli::try_parse_from([
                "accelerator-design",
                "notify-downgrade",
                "--reason",
                reason.key(),
            ])?;
            let Command::NotifyDowngrade { reason: parsed, .. } = cli.command
            else {
                return Err("expected notify-downgrade".into());
            };
            assert_eq!(DowngradeReason::from(parsed), reason);
        }
        Ok(())
    }

    #[test]
    fn an_unknown_reason_is_a_parse_failure() {
        assert!(Cli::try_parse_from([
            "accelerator-design",
            "notify-downgrade",
            "--reason",
            "nonsense",
        ])
        .is_err());
    }

    #[test]
    fn the_forward_compatible_flags_are_accepted_and_ignored(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "accelerator-design",
            "notify-downgrade",
            "--reason",
            "unsupported-platform",
            "--from",
            "playwright",
            "--to",
            "code",
        ])?;
        let Command::NotifyDowngrade { from, to, .. } = cli.command else {
            return Err("expected notify-downgrade".into());
        };
        assert_eq!(from.as_deref(), Some("playwright"));
        assert_eq!(to.as_deref(), Some("code"));
        Ok(())
    }

    #[test]
    fn validate_source_takes_both_recovering_flags(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "accelerator-design",
            "validate-source",
            "http://example.com",
            "--allow-internal",
            "--allow-insecure-scheme",
        ])?;
        let Command::ValidateSource {
            location,
            allow_internal,
            allow_insecure_scheme,
        } = cli.command
        else {
            return Err("expected validate-source".into());
        };
        assert_eq!(location, "http://example.com");
        assert!(allow_internal);
        assert!(allow_insecure_scheme);
        Ok(())
    }

    #[test]
    fn an_unknown_flag_is_a_parse_failure() {
        assert!(Cli::try_parse_from([
            "accelerator-design",
            "validate-source",
            "--nonsense",
            "https://example.com",
        ])
        .is_err());
    }
}
