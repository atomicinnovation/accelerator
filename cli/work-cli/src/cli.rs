//! The clap inbound adapter: the `accelerator-work` command-line surface.

use std::path::PathBuf;

use clap::Args;
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
    /// Show a per-section diff between two work-item-shaped files, for
    /// conflict-resolution review. `local` is the `-` side, `remote` is the
    /// `+` side.
    Diff {
        /// The baseline (`-`) file.
        local: PathBuf,
        /// The changed (`+`) file.
        remote: PathBuf,
    },
    /// Atomically create a new work item under the configured pattern,
    /// self-allocating its own ID.
    Create(Box<CreateArgs>),
}

/// `work create`'s flags — a separate [`Args`] struct (boxed at the
/// `Command::Create` call site) so this, by far the largest variant,
/// doesn't inflate every other variant's size.
#[derive(Args)]
pub struct CreateArgs {
    /// The title (a short noun phrase).
    pub title: String,
    /// The kind (e.g. `story`, `epic`, `task`, `bug`, `spike`).
    pub kind: String,
    /// The priority (e.g. `high`, `medium`, `low`).
    pub priority: String,
    /// The initial status.
    #[arg(long, default_value = "draft")]
    pub status: String,
    /// The parent work item reference (`work-item:NNNN`).
    #[arg(long)]
    pub parent: Option<String>,
    /// A tag to add; repeatable.
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    /// A work item this one blocks; repeatable.
    #[arg(long = "block")]
    pub blocks: Vec<String>,
    /// A work item that blocks this one; repeatable.
    #[arg(long = "blocked-by")]
    pub blocked_by: Vec<String>,
    /// A document this item was derived from; repeatable.
    #[arg(long = "derived-from")]
    pub derived_from: Vec<String>,
    /// A related work item; repeatable.
    #[arg(long = "relates-to")]
    pub relates_to: Vec<String>,
    /// The source document reference.
    #[arg(long)]
    pub source: Option<String>,
    /// The project code, when the configured pattern needs one.
    #[arg(long)]
    pub project: Option<String>,
    /// The author. Falls back to the current VCS identity when omitted.
    #[arg(long)]
    pub author: Option<String>,
    /// The producer name recorded in the frontmatter.
    #[arg(long, default_value = "accelerator-work")]
    pub producer: String,
    /// A file whose content becomes the body (with `NNNN` and the title
    /// placeholder substituted), instead of the template's own skeleton
    /// body.
    #[arg(long = "body-file")]
    pub body_file: Option<PathBuf>,
}
