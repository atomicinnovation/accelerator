//! The `accelerator-shortcut` command surface.

use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(name = "accelerator-shortcut", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show a story or an epic as JSON.
    Show(ShowArgs),
    /// Search the base team's stories, rendering a JSON envelope.
    Search(SearchArgs),
    /// Verify the token, list teams, or discover the workspace's workflows.
    Init {
        #[command(subcommand)]
        action: InitAction,
    },
}

#[derive(Args)]
pub struct Quiet {
    /// Print no notices on stderr.
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct ShowArgs {
    /// `sc-<id>`, `epic-<id>`, or a story or epic URL in this workspace.
    pub reference: String,
    /// Include the most recent N comments.
    #[arg(long)]
    pub comments: Option<usize>,
    #[command(flatten)]
    pub quiet: Quiet,
}

#[derive(Args)]
pub struct SearchArgs {
    /// A workflow state name.
    #[arg(long)]
    pub state: Option<String>,
    /// A member's mention name.
    #[arg(long)]
    pub owner: Option<String>,
    /// A label name.
    #[arg(long)]
    pub label: Option<String>,
    /// A story type (feature, bug, chore) or a local kind.
    #[arg(long = "type", value_name = "TYPE")]
    pub story_type: Option<String>,
    /// A phrase the story's text holds.
    #[arg(long)]
    pub text: Option<String>,
    /// Print at most this many stories.
    #[arg(long)]
    pub limit: Option<usize>,
    #[command(flatten)]
    pub quiet: Quiet,
}

impl SearchArgs {
    pub fn filter_pairs(&self) -> Vec<(String, String)> {
        [
            ("state", &self.state),
            ("owner", &self.owner),
            ("label", &self.label),
            ("type", &self.story_type),
            ("text", &self.text),
        ]
        .into_iter()
        .filter_map(|(field, value)| {
            value.clone().map(|value| (field.to_owned(), value))
        })
        .collect()
    }
}

#[derive(Subcommand)]
pub enum InitAction {
    /// Verify the token without printing it.
    Verify(Quiet),
    /// List the teams the token can see, by team key.
    ListTeams(Quiet),
    /// Discover every team and workflow, and write catalogue.json.
    Discover {
        /// The base team's key: its mention name.
        #[arg(long)]
        team: String,
        /// Replace a different `shortcut.team_key` already set.
        #[arg(long)]
        force: bool,
        #[command(flatten)]
        quiet: Quiet,
    },
}
