//! Composition root for the library-backed probe: calls every query and both
//! port methods against a start directory and **prints each result**.
//!
//! Printing is load-bearing twice over. Without a caller that consumes them,
//! dead-code elimination would let the musl and size checks pass while linking
//! almost none of `gix`/`jj-lib`. And the scrub-invariant runs compare a
//! poisoned child against a clean one on captured stdout — both arms through
//! this same binary, so a difference is a behavioural difference rather than
//! two serialisation routes disagreeing.
//!
//! Usage: `vcs-adapters-fixture <mode> <start-dir>`, where mode is `all` or
//! `only <query>`.
#![allow(
    clippy::exit,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::restriction
)]

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use vcs::RepoRoot;
use vcs::VcsProbe;
use vcs_adapters::library::DualRoots;
use vcs_adapters::library::InProcessProbe;
use vcs_adapters::library::JjRepositoryFacts;
use vcs_adapters::library::JjWorkspaceRole;
use vcs_adapters::library::WorktreeFacts;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (queries, start): (Vec<&str>, &str) = match arguments.as_slice() {
        [mode, start] if mode == "all" => (ALL.to_vec(), start),
        [mode, start] if mode == "control" => (vec!["control"], start),
        [mode, query, start] if mode == "only" => (vec![query.as_str()], start),
        _ => {
            eprintln!("usage: vcs-adapters-fixture (all|only <query>) <dir>");
            return ExitCode::from(2);
        }
    };

    let start = Path::new(start);
    for query in queries {
        if let Err(message) = report(query, start) {
            eprintln!("{message}");
            return ExitCode::from(1);
        }
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

fn report(query: &str, start: &Path) -> Result<(), String> {
    let probe = InProcessProbe;
    match query {
        "is_bare" => print(query, &optional(probe.is_bare(start))),
        "worktree" => {
            let facts = probe
                .worktree(start)
                .map(|found| found.map(|facts| worktree(&facts)));
            print(query, &optional(facts));
        }
        "superproject" => {
            let root = probe
                .superproject(start)
                .map(|found| found.map(|path| render(&path)));
            print(query, &optional(root));
        }
        "jj_workspace_root" => {
            let root = probe
                .jj_workspace_root(start)
                .map(|found| found.map(|path| render(&path)));
            print(query, &optional(root));
        }
        "jj_repository" => {
            let facts = probe
                .jj_repository(start)
                .map(|found| found.map(|facts| repository(&facts)));
            print(query, &optional(facts));
        }
        "dual_roots" => print(query, &dual(&probe.dual_roots(start))),
        // The non-vacuity control for the scrub invariant, run *inside* the
        // poisoned child so the poison it reports is the one the queries above
        // ran under. Unlike plain `discover`, this entry point reads the
        // environment, so under a live poison it resolves the poison target.
        "control" => {
            let found = gix::discover_with_environment_overrides(start)
                .ok()
                .and_then(|repository| {
                    repository.git_dir().canonicalize().ok()
                });
            print(query, &show(found.as_deref()));
        }
        "discover" => print(query, &show(probe.discover(start).as_deref())),
        "kind_and_revision" => {
            let root = probe.discover(start);
            let rendered = root.map_or_else(
                || "absent".to_owned(),
                |root| {
                    let kind = probe.kind(&root);
                    let revision = probe.revision(&root, kind);
                    format!(
                        "{} {}",
                        kind.as_str(),
                        revision.unwrap_or_else(|| "none".to_owned())
                    )
                },
            );
            print(query, &rendered);
        }
        other => return Err(format!("unknown query: {other}")),
    }
    Ok(())
}

fn print(query: &str, rendered: &str) {
    println!("{query}\t{rendered}");
}

/// Renders `Ok(None)` and `Err` differently, because collapsing them is exactly
/// the conflation the query signatures exist to prevent.
fn optional<T: std::fmt::Display>(
    value: Result<Option<T>, vcs_adapters::library::Error>,
) -> String {
    match value {
        Ok(Some(inner)) => inner.to_string(),
        Ok(None) => "absent".to_owned(),
        Err(error) => format!("error: {error}"),
    }
}

fn render(path: &Path) -> String {
    path.display().to_string()
}

fn worktree(facts: &WorktreeFacts) -> String {
    format!(
        "linked={} git_dir={} common_dir={} main={}",
        facts.linked,
        facts.git_dir.display(),
        facts.common_dir.display(),
        show(facts.main_worktree_root.as_deref())
    )
}

fn repository(facts: &JjRepositoryFacts) -> String {
    let role = match facts.role {
        JjWorkspaceRole::Main => "main",
        JjWorkspaceRole::Secondary => "secondary",
    };
    format!("role={role} main_root={}", facts.main_root.display())
}

fn dual(roots: &DualRoots) -> String {
    let render =
        |side: &Result<Option<PathBuf>, vcs_adapters::library::Error>| {
            match side {
                Ok(Some(path)) => path.display().to_string(),
                Ok(None) => "absent".to_owned(),
                Err(error) => format!("error: {error}"),
            }
        };
    format!("git={} jj={}", render(&roots.git), render(&roots.jj))
}

fn show(path: Option<&Path>) -> String {
    path.map_or_else(|| "absent".to_owned(), |path| path.display().to_string())
}
