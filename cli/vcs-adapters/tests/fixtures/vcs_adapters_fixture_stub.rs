//! The stubbed twin of `vcs_adapters_fixture`: the same command surface and the
//! same printing, with every query replaced by a constant.
//!
//! It exists to give the size floor a denominator. Built alongside the linked
//! artefact from a crate that declares `gix` and `jj-lib` either way, the delta
//! between the two is what dead-code elimination removed — so a linked build
//! that stopped calling the queries would collapse toward this size and the
//! floor would catch it.
//!
//! A second `[[bin]]` rather than a `stub` feature on `vcs-adapters`: the crate
//! must gain no `[features]` entry beyond `bash-parity`, and CI's
//! `--all-features` would otherwise turn a fixture feature on workspace-wide.
#![allow(
    clippy::exit,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::restriction
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let queries: Vec<&str> = match arguments.as_slice() {
        [mode, _start] if mode == "all" => ALL.to_vec(),
        [mode, query, _start] if mode == "only" => vec![query.as_str()],
        _ => {
            eprintln!("usage: vcs-adapters-fixture-stub (all|only <q>) <dir>");
            return ExitCode::from(2);
        }
    };

    for query in queries {
        println!("{query}\tabsent");
    }
    ExitCode::SUCCESS
}

const ALL: [&str; 8] = [
    "is_bare",
    "worktree",
    "superproject",
    "jj_workspace_root",
    "jj_repository",
    "dual_roots",
    "discover",
    "kind_and_revision",
];
