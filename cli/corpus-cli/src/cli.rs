//! The clap inbound adapter: the `accelerator-corpus` command-line surface.

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;
use corpus::FilenameTimestampFormat;

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
    Derive {
        /// Which shape the filename timestamp is rendered in.
        #[arg(long, value_enum, default_value_t = FilenameTimestampFormatArg::DateTimeUnderscored)]
        filename_timestamp_format: FilenameTimestampFormatArg,
    },
}

/// The CLI-local mirror of `corpus::FilenameTimestampFormat`.
///
/// The domain crate cannot derive `ValueEnum`: its import rule permits only
/// std, `kernel::Error` and `crate`.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilenameTimestampFormatArg {
    DateTimeUnderscored,
    CompactTime,
    DateOnly,
}

impl From<FilenameTimestampFormatArg> for FilenameTimestampFormat {
    fn from(arg: FilenameTimestampFormatArg) -> Self {
        match arg {
            FilenameTimestampFormatArg::DateTimeUnderscored => {
                Self::DateTimeUnderscored
            }
            FilenameTimestampFormatArg::CompactTime => Self::CompactTime,
            FilenameTimestampFormatArg::DateOnly => Self::DateOnly,
        }
    }
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
        /// A named flag rather than a second bare positional (the retired
        /// bash implementation's own shape) — this CLI's argument shape is
        /// not held to bash parity, and a named flag is more discoverable
        /// in `--help` and harder to invoke by mistake.
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

#[cfg(test)]
mod tests {
    use clap::Parser as _;
    use corpus::FilenameTimestampFormat;

    use super::Cli;
    use super::Command;
    use super::FilenameTimestampFormatArg;
    use super::MetadataAction;

    #[test]
    fn each_argument_selects_its_own_variant() {
        assert_eq!(
            FilenameTimestampFormat::from(
                FilenameTimestampFormatArg::DateTimeUnderscored
            ),
            FilenameTimestampFormat::DateTimeUnderscored
        );
        assert_eq!(
            FilenameTimestampFormat::from(
                FilenameTimestampFormatArg::CompactTime
            ),
            FilenameTimestampFormat::CompactTime
        );
        assert_eq!(
            FilenameTimestampFormat::from(FilenameTimestampFormatArg::DateOnly),
            FilenameTimestampFormat::DateOnly
        );
    }

    fn parse_derive(
        arguments: &[&str],
    ) -> Result<FilenameTimestampFormatArg, Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from(arguments)?;
        let Command::Metadata {
            action:
                MetadataAction::Derive {
                    filename_timestamp_format,
                },
        } = cli.command
        else {
            return Err("expected metadata derive".into());
        };
        Ok(filename_timestamp_format)
    }

    #[test]
    fn the_format_defaults_to_the_shape_existing_callers_receive(
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            parse_derive(&["accelerator-corpus", "metadata", "derive"])?,
            FilenameTimestampFormatArg::DateTimeUnderscored
        );
        Ok(())
    }

    #[test]
    fn the_date_only_format_is_selectable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            parse_derive(&[
                "accelerator-corpus",
                "metadata",
                "derive",
                "--filename-timestamp-format",
                "date-only",
            ])?,
            FilenameTimestampFormatArg::DateOnly
        );
        Ok(())
    }

    #[test]
    fn the_compact_time_format_is_selectable(
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            parse_derive(&[
                "accelerator-corpus",
                "metadata",
                "derive",
                "--filename-timestamp-format",
                "compact-time",
            ])?,
            FilenameTimestampFormatArg::CompactTime
        );
        Ok(())
    }
}
