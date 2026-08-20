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
    /// Atomically apply field/tag/list-field edits to an existing work
    /// item's frontmatter.
    Update(Box<UpdateArgs>),
    /// Canonicalise a work-item ID under the configured pattern (zero-pads
    /// a bare number, prepends the default project when the pattern needs
    /// one).
    CanonicaliseId {
        /// The ID or bare number to canonicalise.
        input: String,
    },
    /// Print the next N sequential IDs the configured pattern would
    /// allocate. Display-only: never writes a file or commits a number.
    NextNumber {
        /// The project code, when the configured pattern needs one.
        #[arg(long)]
        project: Option<String>,
        /// How many sequential IDs to print.
        #[arg(long, default_value_t = 1)]
        count: u32,
    },
    /// List work items from the configured directory, filtered and
    /// rendered as a table — or as a parent/child tree with `--hierarchy`.
    /// Carries a sync-status column when `work.integration` names a
    /// tracker. Read-only: never writes a file and never contacts the
    /// remote to mutate it.
    List(Box<ListArgs>),
    /// Reconcile local work items with the configured remote tracker
    /// (`work.integration`).
    ///
    /// Exit codes: 0 clean; 4 items await a human (unresolved conflicts,
    /// skipped-dirty pulls, remote-absent or indeterminate items); 70 a
    /// read failed or every per-item failure was retryable; 71 any
    /// per-item failure was terminal; 1 an internal error; 2 a usage
    /// error; 5 refused (would exceed --max-pulls/--max-pushes, zero
    /// writes); 72 the configured tracker is recognised but has no client
    /// built yet; 73 work.integration is unset or names an unrecognised
    /// tracker; 74 the configured tracker is wired but its configuration or
    /// credentials are missing (nothing was sent). The report on stdout is
    /// authoritative: check it for `unresolved` lines regardless of exit
    /// code, since a 71 run may also carry conflicts.
    Sync(Box<SyncArgs>),
}

fn parse_key_value(raw: &str) -> Result<(String, String), String> {
    raw.split_once('=').map_or_else(
        || Err(format!("expected KEY=VALUE, got '{raw}'")),
        |(key, value)| Ok((key.to_owned(), value.to_owned())),
    )
}

/// `work list`'s flags, boxed for the same reason as [`CreateArgs`]. Every
/// filter is a conjunct: an item is listed only when it satisfies all of
/// the ones supplied.
#[derive(Args)]
pub struct ListArgs {
    /// Keep only items whose `status` equals this value exactly.
    #[arg(long)]
    pub status: Option<String>,
    /// Keep only items whose `kind` equals this value exactly.
    #[arg(long)]
    pub kind: Option<String>,
    /// Keep only items whose `priority` equals this value exactly.
    #[arg(long)]
    pub priority: Option<String>,
    /// Keep only items whose `parent` canonicalises to this value (short
    /// and long ID forms compare equal).
    #[arg(long)]
    pub parent: Option<String>,
    /// Keep only items carrying this tag; repeatable, and every one
    /// supplied must be present.
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    /// Render the parent/child hierarchy as a tree instead of a table.
    #[arg(long)]
    pub hierarchy: bool,
    /// Keep only items whose title contains this substring
    /// (case-insensitive).
    pub term: Option<String>,
}

/// `work update`'s flags — boxed for the same reason as [`CreateArgs`].
#[derive(Args)]
pub struct UpdateArgs {
    /// The work-item file to update.
    pub path: PathBuf,
    /// A `KEY=VALUE` scalar field to set; repeatable. Rejects `id`/
    /// `work_item_id` and every list-typed field (use `--append`/
    /// `--remove`/`--add-tag`/`--remove-tag` for those instead).
    #[arg(long = "set", value_parser = parse_key_value)]
    pub sets: Vec<(String, String)>,
    /// A tag to add; repeatable.
    #[arg(long = "add-tag")]
    pub add_tags: Vec<String>,
    /// A tag to remove; repeatable.
    #[arg(long = "remove-tag")]
    pub remove_tags: Vec<String>,
    /// A `KEY=VALUE` pair appending `VALUE` to list-typed field `KEY`
    /// (`blocks`, `blocked_by`, `derived_from`, `relates_to`); repeatable.
    #[arg(long = "append", value_parser = parse_key_value)]
    pub appends: Vec<(String, String)>,
    /// A `KEY=VALUE` pair removing `VALUE` from list-typed field `KEY`;
    /// repeatable.
    #[arg(long = "remove", value_parser = parse_key_value)]
    pub removes: Vec<(String, String)>,
    /// Replace the item's remote issue with its whole rendered content
    /// before writing the edit locally. Refused when the item has no
    /// `external_id` — see `create --push` for an unsynced item. Unlike
    /// `create --push`, a failed push leaves the local file untouched:
    /// exit 70 (retryable, unchanged) or 71 (terminal, baseline entry
    /// cleared so the next `sync` reconciles it as a conflict).
    #[arg(long)]
    pub push: bool,
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
    /// Push the new item to the configured remote tracker before writing
    /// it locally. On a retryable failure the create is retried once, then
    /// the item is saved unsynced; a terminal failure is reported and the
    /// item saved unsynced (a remote issue may already exist — see
    /// `exit_codes` for the 70/71 contract). The file is written either way.
    #[arg(long)]
    pub push: bool,
    /// Preview the fields a `--push` create would resolve against the
    /// configured tracker, then exit without writing a file or creating a
    /// remote issue. Prints one tab-separated line the create skill parses;
    /// exits 70 when the tracker is unreachable (the preview could not be
    /// resolved), so the caller can distinguish an unreachable tracker from
    /// an unresolvable configured value (reported as an `unresolvable`
    /// source on the line, exit 0).
    #[arg(long)]
    pub dry_run: bool,
}

/// `work sync`'s flags, boxed for the same reason as [`CreateArgs`].
#[derive(Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct SyncArgs {
    /// Push local changes only; never write a local file.
    #[arg(long, conflicts_with = "pull_only")]
    pub push_only: bool,
    /// Pull remote changes only; never write to the remote.
    #[arg(long)]
    pub pull_only: bool,
    /// Report the actions a run would take without touching any work item.
    /// Remote reads still occur (this is what counts against rate limits)
    /// and gitignored conflict dossiers are still written, but no create,
    /// update or work-item write does.
    #[arg(long)]
    pub preview: bool,
    /// An `<id>=<remote|local|skip>` order for a reported conflict;
    /// repeatable. An unrecognised token is treated as `skip`, with a
    /// warning naming the token and the accepted set.
    #[arg(long = "resolve", value_parser = parse_key_value)]
    pub resolutions: Vec<(String, String)>,
    /// Read each item with its own request instead of one bulk retrieval.
    #[arg(long)]
    pub per_item_reads: bool,
    /// Refuse the run if it would overwrite more than this many local
    /// files from the remote. 0 refuses every pull.
    #[arg(long, default_value_t = 25)]
    pub max_pulls: usize,
    /// Refuse the run if it would replace more than this many remote
    /// issues. 0 refuses every push.
    #[arg(long, default_value_t = 25)]
    pub max_pushes: usize,
}
