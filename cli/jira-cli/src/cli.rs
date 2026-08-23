//! The `accelerator-jira` command surface.

// A clap args struct is a flat flag record; several independent boolean flags
// on one subcommand are the surface, not a state object to model differently.
#![allow(clippy::struct_excessive_bools)]

use std::path::PathBuf;

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

#[derive(Parser)]
#[command(name = "accelerator-jira", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create an issue.
    Create(CreateArgs),
    /// Update an issue's summary and body.
    Update(UpdateArgs),
    /// Show an issue as JSON, rendering ADF fields to Markdown.
    Show(ShowArgs),
    /// Search issues, rendering Jira's verbatim envelope.
    Search(SearchArgs),
    /// Comment on an issue.
    Comment {
        #[command(subcommand)]
        action: CommentAction,
    },
    /// Transition an issue to a named workflow state.
    Transition(TransitionArgs),
    /// Attach files to an issue.
    Attach(AttachArgs),
    /// Verify credentials, discover the catalogue, or list its parts.
    Init {
        #[command(subcommand)]
        action: InitAction,
    },
    /// Refresh, resolve or list the custom-field cache.
    Fields {
        #[command(subcommand)]
        action: FieldsAction,
    },
    /// Resolve the issue type and project for a create, as a tab-separated line.
    ResolveFields(ResolveFieldsArgs),
}

/// What `create` projects to stdout.
#[derive(Clone, Copy, ValueEnum)]
pub enum CreateEmit {
    /// The full issue JSON envelope (the default).
    Json,
    /// Only the bare validated key, for a frontmatter writeback.
    Key,
}

#[derive(Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long, alias = "issuetype")]
    pub r#type: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub body_file: Option<PathBuf>,
    #[arg(long)]
    pub assignee: Option<String>,
    /// The stdout projection: the full JSON, or the bare key for a writeback.
    #[arg(long, value_enum, default_value = "json")]
    pub emit: CreateEmit,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct UpdateArgs {
    pub key: String,
    #[arg(long)]
    pub summary: Option<String>,
    #[arg(long)]
    pub body_file: Option<PathBuf>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct ShowArgs {
    pub key: String,
    /// Include the most recent N comments (0-100).
    #[arg(long)]
    pub comments: Option<u32>,
    /// The field selection query (default `*all`).
    #[arg(long, default_value = "*all")]
    pub fields: String,
    /// The expand query (default `names,schema,transitions`).
    #[arg(long, default_value = "names,schema,transitions")]
    pub expand: String,
    /// Render ADF fields to Markdown (on by default).
    #[arg(long, overrides_with = "no_render_adf")]
    pub render_adf: bool,
    #[arg(long = "no-render-adf")]
    pub no_render_adf: bool,
}

#[derive(Args)]
pub struct SearchArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub all_projects: bool,
    #[arg(long = "status")]
    pub statuses: Vec<String>,
    #[arg(long = "label")]
    pub labels: Vec<String>,
    #[arg(long = "assignee")]
    pub assignees: Vec<String>,
    #[arg(long = "type")]
    pub types: Vec<String>,
    #[arg(long = "component")]
    pub components: Vec<String>,
    #[arg(long = "reporter")]
    pub reporters: Vec<String>,
    #[arg(long = "parent")]
    pub parents: Vec<String>,
    #[arg(long = "text")]
    pub texts: Vec<String>,
    #[arg(long)]
    pub watching: bool,
    #[arg(long)]
    pub jql: Option<String>,
    #[arg(long = "field")]
    pub fields: Vec<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long)]
    pub page_token: Option<String>,
    /// Render ADF fields in the results to Markdown (off by default).
    #[arg(long = "render-adf")]
    pub render_adf: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum CommentAction {
    /// Add a comment to an issue.
    Add(CommentAddArgs),
    /// List an issue's comments as a paginated envelope.
    List(CommentListArgs),
    /// Edit an existing comment.
    Edit(CommentEditArgs),
    /// Delete a comment.
    Delete(CommentDeleteArgs),
}

#[derive(Args)]
pub struct CommentAddArgs {
    pub key: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<PathBuf>,
    #[arg(long)]
    pub visibility: Option<String>,
    #[arg(long = "no-notify")]
    pub no_notify: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct CommentListArgs {
    pub key: String,
    #[arg(long, default_value_t = 50)]
    pub page_size: u32,
    #[arg(long = "first-page-only")]
    pub first_page_only: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct CommentEditArgs {
    pub key: String,
    pub comment_id: String,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub body_file: Option<PathBuf>,
    #[arg(long)]
    pub visibility: Option<String>,
    #[arg(long = "no-notify")]
    pub no_notify: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct CommentDeleteArgs {
    pub key: String,
    pub comment_id: String,
    #[arg(long = "no-notify")]
    pub no_notify: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct TransitionArgs {
    pub key: String,
    /// The target state name, positional or via `--state`.
    pub state: Option<String>,
    #[arg(long = "state", value_name = "STATE")]
    pub state_flag: Option<String>,
    #[arg(long = "transition-id")]
    pub transition_id: Option<String>,
    #[arg(long)]
    pub resolution: Option<String>,
    #[arg(long)]
    pub comment: Option<String>,
    #[arg(long = "no-notify")]
    pub no_notify: bool,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct AttachArgs {
    pub key: String,
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    #[arg(long, short)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum InitAction {
    /// Verify credentials and cache the site identity, without printing them.
    Verify,
    /// Discover the project and field catalogue, caching it.
    Discover,
    /// Print the resolved default project.
    PromptDefault,
    /// Refresh the custom-field cache.
    RefreshFields,
    /// List the cached projects.
    ListProjects,
    /// List the cached fields.
    ListFields,
}

#[derive(Subcommand)]
pub enum FieldsAction {
    /// Refresh the custom-field cache from the live tracker.
    Refresh,
    /// Resolve one field name or id to its field id.
    Resolve { query: String },
    /// List the cached fields.
    List,
}

#[derive(Args)]
pub struct ResolveFieldsArgs {
    /// A work-item file whose frontmatter seeds the resolution.
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// An explicit kind (story|bug|epic|task|spike).
    #[arg(long)]
    pub kind: Option<String>,
    /// An explicit project key.
    #[arg(long)]
    pub project: Option<String>,
    /// A project-coded id the project can be read from.
    #[arg(long)]
    pub id: Option<String>,
}
