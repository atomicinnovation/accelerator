//! The `accelerator-linear` command surface.

use std::path::PathBuf;

use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(name = "accelerator-linear", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create an issue from a work-item file or an explicit title.
    Create(CreateArgs),
    /// Update an issue's fields.
    Update(UpdateArgs),
    /// Show an issue as JSON.
    Show(ShowArgs),
    /// Search issues, rendering a JSON envelope.
    Search(SearchArgs),
    /// Comment on an issue.
    Comment {
        #[command(subcommand)]
        action: CommentAction,
    },
    /// Transition an issue to a named workflow state.
    Transition(TransitionArgs),
    /// Attach a link or a file to an issue.
    Attach(AttachArgs),
    /// Verify credentials, list teams, or discover a team's states.
    Init {
        #[command(subcommand)]
        action: InitAction,
    },
}

#[derive(Args)]
pub struct CreateArgs {
    /// A work-item file whose frontmatter and body seed the issue.
    pub file: Option<PathBuf>,
    /// The issue title (the no-file mode).
    #[arg(long)]
    pub title: Option<String>,
    /// A Markdown body file for the no-file mode.
    #[arg(long)]
    pub body_file: Option<PathBuf>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct UpdateArgs {
    pub id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub assignee_id: Option<String>,
    #[arg(long)]
    pub priority: Option<i64>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct ShowArgs {
    pub id: String,
    /// Include the most recent N comments.
    #[arg(long)]
    pub comments: Option<usize>,
}

#[derive(Args)]
pub struct SearchArgs {
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub assignee: Option<String>,
    #[arg(long)]
    pub label: Option<String>,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum CommentAction {
    /// Add a Markdown comment to an issue.
    Add(CommentAddArgs),
}

#[derive(Args)]
pub struct CommentAddArgs {
    pub id: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<PathBuf>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct TransitionArgs {
    pub id: String,
    /// The target state name, positional or via `--state`.
    pub state: Option<String>,
    #[arg(long = "state", value_name = "STATE")]
    pub state_flag: Option<String>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct AttachArgs {
    pub id: String,
    #[arg(long)]
    pub url: Option<String>,
    #[arg(long)]
    pub file: Option<PathBuf>,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum InitAction {
    /// Verify credentials without printing them.
    Verify,
    /// List the teams the token can see.
    ListTeams,
    /// Discover a team's workflow states.
    Discover {
        #[arg(long)]
        team_id: String,
    },
}
