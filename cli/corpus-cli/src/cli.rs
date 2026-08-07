//! The clap inbound adapter: the `accelerator-corpus` command-line surface.

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

/// The `accelerator-corpus` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator-corpus", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Architecture decision record numbering and status.
    Adr {
        #[command(subcommand)]
        action: AdrAction,
    },
    /// Artifact-metadata derivation (the unified frontmatter provenance
    /// block).
    Metadata {
        #[command(subcommand)]
        action: MetadataAction,
    },
}

#[derive(Subcommand)]
pub enum AdrAction {
    /// The next sequential ADR number(s), one per line.
    NextNumber {
        /// How many sequential numbers to output.
        ///
        /// A raw string, hand-validated in the command layer rather than
        /// clap-parsed, so an invalid value reproduces bash's exact error
        /// text and exit code 1 instead of clap's own usage-error exit
        /// code 2.
        #[arg(long, default_value = "1")]
        count: String,
        /// On failure (a bad config, an unreadable decisions directory),
        /// print the diagnostic to stderr and exit 0 with empty stdout
        /// instead of exiting 1 — for a context-injection call site that
        /// must not abort a skill's preamble.
        #[arg(long)]
        fail_safe: bool,
    },
    /// The `status` field from an ADR file's YAML frontmatter.
    ReadStatus {
        /// The ADR file to read.
        ///
        /// A bare optional positional rather than a clap-required argument,
        /// so a missing argument reaches the handler as `None` and is
        /// hand-validated there — reproducing bash's exit code 1 instead of
        /// clap's own required-argument exit code 2.
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum MetadataAction {
    /// The unified artifact-metadata provenance block: the UTC datetime, the
    /// host-local filename timestamp, and (inside a VCS checkout) the
    /// repository name and revision.
    Derive,
}
