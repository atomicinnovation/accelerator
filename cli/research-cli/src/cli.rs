//! The clap inbound adapter: the `accelerator-research` command-line surface.
//!
//! Families, verbs and the limit are taken as raw text so the domain's
//! `FetchRequest::parse` owns every usage message.

use std::path::PathBuf;

use clap::Parser;
use clap::Subcommand;

const FETCH_HELP: &str = "\
Fetch tiered scholarly records from OpenAlex or arXiv.

Families: openalex, arxiv. Verbs: search (free text), lookup (one ID: an
OpenAlex W… ID or DOI, or an arXiv ID).

Prints compact JSON on stdout: {\"status\":\"ok\",\"records\":[…]}, each record
carrying title, authors, url, venue, venue_signals, abstract, tier, retracted
and withdrawn; or {\"status\":\"unavailable\",\"source\":…,\"reason\":…} with a
reason of rate_limited, budget_exhausted or upstream_error when the source
cannot answer now. Both exit 0.

Every call finishes within 100 s of starting. Usage errors exit 2 before any
request; credential refusals and rejected requests exit 1.";

#[derive(Parser)]
#[command(name = "accelerator-research", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Fetch tiered scholarly records from OpenAlex or arXiv.
    #[command(long_about = FETCH_HELP)]
    Fetch {
        /// openalex or arxiv.
        family: String,
        /// search or lookup.
        verb: String,
        /// The search query, or the ID to look up. Several words are joined
        /// with single spaces.
        terms: Vec<String>,
        /// How many records a search returns, 1–25 (default 10).
        #[arg(long)]
        limit: Option<String>,
    },
    /// Conventions of a `topic-research` set.
    Topic {
        #[command(subcommand)]
        action: TopicAction,
    },
    /// Judge a `PreToolUse` hook call from stdin, confining the researcher
    /// subagent to the fetch and to finding files. Exits 2 to block.
    Guard {
        /// Accepted for parity with the launcher, which acts on it.
        #[arg(long)]
        fail_safe: bool,
        /// Accepted for parity with the launcher, which acts on it.
        #[arg(long)]
        non_blocking: bool,
    },
}

#[derive(Subcommand)]
pub enum TopicAction {
    /// The (focus area, profile) pairs a `conduct` round must still research,
    /// each with its finding's absolute path, and, at `--depth` above 1, each
    /// pair's missing level-note nodes or the notes it composes from, and
    /// every outline item's completeness, as JSON. Read-only unless `--start`
    /// or `--run` is given; fields are only ever added.
    ///
    /// Every number and run flag is a raw string, hand-validated in the
    /// command layer, so an invalid value or combination exits 1 with this
    /// tool's own error text rather than clap's exit 2.
    Outstanding {
        /// The set's slug, set directory, or sub-document path.
        slug: String,
        /// The directory holding one `<name>-profile/SKILL.md` per profile.
        #[arg(long)]
        profiles_dir: PathBuf,
        /// How many levels each pair's tree is researched to. Defaults to 1
        /// and is never read from `research.topic.depth`, so a hand run
        /// must pass the depth `conduct` resolved.
        #[arg(long, default_value = "1", allow_hyphen_values = true)]
        depth: String,
        /// The most spawns to offer; the rest are counted in `remaining`.
        #[arg(long, allow_hyphen_values = true)]
        limit: Option<String>,
        /// Begin a `conduct` run, recording its ledger in the set.
        #[arg(long)]
        start: bool,
        /// Continue the `conduct` run with this id.
        #[arg(long, allow_hyphen_values = true)]
        run: Option<String>,
        /// Acknowledge this batch of the run as spawned.
        #[arg(long, allow_hyphen_values = true)]
        spawned: Option<String>,
    },
    /// Remove the ledger of a `conduct` run that has finished. Succeeds when
    /// the set has no ledger.
    EndRun {
        /// The set's slug, set directory, or sub-document path.
        slug: String,
        /// The run whose ledger is removed.
        #[arg(long, allow_hyphen_values = true)]
        run: String,
    },
}
