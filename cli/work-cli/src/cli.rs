//! The clap inbound adapter: the `accelerator-work` command-line surface.

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

/// The `accelerator-work` command-line surface: work-item lifecycle
/// primitives (`create`, `show`, `resolve`, `diff`, `update`) plus small
/// utility subcommands used by the skills that orchestrate them.
#[derive(Parser)]
#[command(name = "accelerator-work", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Resolve a user-supplied work-item identifier (a path, a full ID, or
    /// a bare number) to a canonical file path.
    Resolve {
        /// The identifier to resolve: a path, a full ID (e.g. `PROJ-0042`
        /// or `0042`), or a bare legacy number (e.g. `42`).
        input: String,
    },
    /// Print the work-item template's hint values for a frontmatter field
    /// (e.g. `kind`, `status`, `priority`), one per line. Always exits 0.
    TemplateHints {
        /// The frontmatter field to extract hints for.
        field: String,
    },
    /// Print a work item: the whole file, or a single frontmatter field.
    Show {
        /// The work-item file to read.
        path: PathBuf,
        /// Print only this frontmatter field's raw value instead of the
        /// whole file.
        #[arg(long)]
        field: Option<String>,
    },
}
