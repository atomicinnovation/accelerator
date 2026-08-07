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
    /// Body-section typed-linkage extraction.
    Linkage {
        #[command(subcommand)]
        action: LinkageAction,
    },
    /// Structural and referential frontmatter conformance checking.
    Frontmatter {
        #[command(subcommand)]
        action: FrontmatterAction,
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

#[derive(Subcommand)]
pub enum LinkageAction {
    /// Every typed-linkage record in a document's body sections, as TSV:
    /// `source_type<TAB>key<TAB>target_ref<TAB>anchor<TAB>band`.
    Extract {
        /// The document to extract linkage from.
        file: PathBuf,
        /// The source document's own type, overriding path-based inference.
        ///
        /// A named flag rather than a second bare positional (bash's
        /// `lp_parse_file <file> [source_type_override]` shape) — this CLI's
        /// argument shape is not held to bash parity, and a named flag is
        /// more discoverable in `--help` and harder to invoke by mistake.
        #[arg(long)]
        source_type: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum FrontmatterAction {
    /// Every in-scope file's structural and referential-integrity
    /// violations.
    Validate {
        /// A directory to walk for markdown files, in addition to `--file`.
        ///
        /// When both `--dir` and `--file` are omitted, every file under the
        /// configured doc-type table is validated.
        #[arg(long)]
        dir: Vec<PathBuf>,
        /// A single file to validate, in addition to `--dir`.
        #[arg(long)]
        file: Vec<PathBuf>,
        /// Which check categories to run.
        #[arg(
            long,
            value_delimiter = ',',
            default_value = "structure,references"
        )]
        checks: Vec<CheckKind>,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum CheckKind {
    Structure,
    References,
}
