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

use vcs::checkout::DualRoots;
use vcs::checkout::JjRepositoryFacts;
use vcs::checkout::JjWorkspaceRole;
use vcs::checkout::WorktreeFacts;
use vcs::RepoRoot;
use vcs::UserIdentityProbe;
use vcs::VcsKind;
use vcs::VcsProbe;
use vcs::VcsReporter;
use vcs_adapters::library::InProcessProbe;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (queries, start): (Vec<&str>, &str) = match arguments.as_slice() {
        [mode, start] if mode == "all" => (ALL.to_vec(), start),
        [mode, start] if mode == "control" => (vec!["control"], start),
        [mode, start] if mode == "status" || mode == "log" => {
            // Brings the real `main`'s one path beyond the library — the
            // ACCELERATOR_LOG subscriber install — inside the zero-spawn
            // envelope. It reads config and installs a stderr subscriber; it
            // never spawns.
            let _ = kernel::logging::init();
            println!("{}", render_report(mode, Path::new(start)));
            return ExitCode::SUCCESS;
        }
        [mode, query, start] if mode == "only" => (vec![query.as_str()], start),
        _ => {
            eprintln!(
                "usage: vcs-adapters-fixture (all|status|log|only <query>) <dir>"
            );
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

/// Mirrors `accelerator-vcs`'s status/log boundary over the port: discover the
/// root, classify it, and fold an adapter `Err` to the same ADR-0066 fallback,
/// so the zero-spawn goldens match the shipped binary's output.
fn render_report(mode: &str, start: &Path) -> String {
    let probe = InProcessProbe;
    let root = probe.discover(start);
    let kind = root
        .as_deref()
        .map_or(VcsKind::Git, |root| probe.kind(root));
    let dir = root.as_deref().unwrap_or(start);
    if mode == "log" {
        probe.log_report(dir, kind).map_or_else(
            |_| "(log unavailable)".to_owned(),
            |report| vcs::log::render(&report),
        )
    } else {
        probe.status_report(dir, kind).map_or_else(
            |_| "(status unavailable)".to_owned(),
            |report| vcs::status::render(&report),
        )
    }
}

// `kind_and_user_name` is deliberately absent: it reads the ambient
// configured identity (`HOME`, `JJ_CONFIG`, `XDG_CONFIG_HOME`, ...), so
// `scrub.rs`'s invariant — that these queries answer identically regardless
// of a poisoned environment — does not hold for it by design. It is reached
// only through `only kind_and_user_name`, exercised by `tests/user_name.rs`.
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
        "kind_and_user_name" => {
            let root = probe.discover(start);
            let rendered = root.map_or_else(
                || "absent".to_owned(),
                |root| {
                    let kind = probe.kind(&root);
                    let name = probe.user_name(&root, kind);
                    format!(
                        "{} {}",
                        kind.as_str(),
                        name.unwrap_or_else(|| "absent".to_owned())
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
    let render = |side: &Result<Option<PathBuf>, kernel::Error>| match side {
        Ok(Some(path)) => path.display().to_string(),
        Ok(None) => "absent".to_owned(),
        Err(error) => format!("error: {error}"),
    };
    format!("git={} jj={}", render(&roots.git), render(&roots.jj))
}

fn show(path: Option<&Path>) -> String {
    path.map_or_else(|| "absent".to_owned(), |path| path.display().to_string())
}
