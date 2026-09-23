//! Path-manifest ownership classification.
//!
//! Three classes, exactly as documented — runner-managed bookkeeping,
//! current-run interactive session artefacts (the files the run's session
//! writes, or reads back on resume), and everything else, which must appear
//! in the manifest verbatim.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    RunnerManaged,
    SessionArtefact,
    Manifested,
    Unowned,
}

pub struct RunnerPaths<'a> {
    pub applied: &'a str,
    pub skipped: &'a str,
    pub run_paths: &'a str,
    pub recorded_run_base: &'a str,
    /// The run-lock directory, matched by prefix: its sentinel carries a
    /// per-acquisition nonce, and the lock is held across every scan.
    pub lock_dir: &'a str,
}

/// Classifies one repo-relative dirty path.
///
/// `run_base_matches` gates class (b) only — a stale run's session
/// artefacts are never owned by pattern, and fall through to the manifest
/// check (which a stale run's artefacts also fail, since they were never
/// entered into a fresh run's manifest).
#[must_use]
pub fn classify(
    path: &str,
    runner: &RunnerPaths<'_>,
    manifest: &[String],
    run_base_matches: bool,
) -> Ownership {
    if is_runner_managed(path, runner) {
        return Ownership::RunnerManaged;
    }
    if run_base_matches && is_session_artefact(path) {
        return Ownership::SessionArtefact;
    }
    if manifest.iter().any(|manifested| manifested == path) {
        return Ownership::Manifested;
    }
    Ownership::Unowned
}

/// The runner's own append-only bookkeeping, owned by pattern rather than by
/// manifest.
///
/// Unlike a session artefact, ownership here needs no matching run base:
/// these four paths hold nothing the user authored, so a run that finds them
/// dirty has found its own writing, whatever run base recorded them.
#[must_use]
pub fn is_runner_managed(path: &str, runner: &RunnerPaths<'_>) -> bool {
    path == runner.applied
        || path == runner.skipped
        || path == runner.run_paths
        || path == runner.recorded_run_base
        || path.starts_with(runner.lock_dir)
}

const SESSION_PREFIX: &str = ".accelerator/state/migrations-";
const DECISIONS_SUFFIX: &str = "-decisions.txt";
const SESSION_SUFFIXES: [&str; 4] = [
    "-session.jsonl",
    "-stderr.log",
    "-resume-state.tmp",
    DECISIONS_SUFFIX,
];

/// The id's *first* character must be a lowercase letter or digit — the
/// rest is unconstrained (real ids like
/// `0007-unify-meta-corpus-frontmatter` carry hyphens throughout).
fn migration_id<'a>(rest: &'a str, suffix: &str) -> Option<&'a str> {
    let id = rest.strip_suffix(suffix)?;
    id.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
        .then_some(id)
}

/// The file a stalled interactive migration asks the user to write its
/// decisions to, repo-relative.
#[must_use]
pub fn decisions_file(migration_id: &str) -> String {
    format!("{SESSION_PREFIX}{migration_id}{DECISIONS_SUFFIX}")
}

#[must_use]
pub fn is_session_artefact(path: &str) -> bool {
    let Some(rest) = path.strip_prefix(SESSION_PREFIX) else {
        return false;
    };
    SESSION_SUFFIXES
        .iter()
        .any(|suffix| migration_id(rest, suffix).is_some())
}

/// True only for the canonical interactive session log.
///
/// Never the stderr capture or the resume-state tmp —
/// `is_session_artefact`'s stricter sibling, used wherever the caller
/// specifically wants the decided-transformation count (`wc -l`-equivalent)
/// a log, and only a log, gives.
#[must_use]
pub fn is_session_log(path: &str) -> bool {
    path.strip_prefix(SESSION_PREFIX)
        .and_then(|rest| migration_id(rest, "-session.jsonl"))
        .is_some()
}
