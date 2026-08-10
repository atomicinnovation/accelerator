//! The metadata-read path as a black-box entry point: resolves a start
//! directory's repository facts through the real composition —
//! [`VcsBackedRepoFactsProbe`] into `vcs_adapters::facts` — and **prints
//! them**.
//!
//! Printing is what makes the zero-spawn assertion meaningful: the comparison
//! is between a run with the real `git`/`jj` reachable and one without, so a
//! probe degrading to absence has to show up as a changed value rather than as
//! silence.
//!
//! Usage: `corpus-adapters-fixture <start-dir>`.
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::restriction)]

use std::path::Path;
use std::process::ExitCode;

use corpus::RepoFactsProbe as _;
use corpus_adapters::VcsBackedRepoFactsProbe;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [start] = arguments.as_slice() else {
        eprintln!("usage: corpus-adapters-fixture <start-dir>");
        return ExitCode::from(2);
    };

    let facts = VcsBackedRepoFactsProbe.facts(Path::new(start));
    let rendered = facts.map_or_else(
        || "absent".to_owned(),
        |facts| {
            format!(
                "name={} revision={}",
                facts.name,
                facts.revision.unwrap_or_else(|| "none".to_owned())
            )
        },
    );
    println!("facts\t{rendered}");
    ExitCode::SUCCESS
}
