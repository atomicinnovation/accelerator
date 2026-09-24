//! The clap inbound adapter: the `accelerator-research` command-line surface.
//!
//! Families, verbs and the limit are taken as raw text so the domain's
//! `FetchRequest::parse` owns every usage message.

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
}
