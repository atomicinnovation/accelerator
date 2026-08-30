//! Locally-declared outbound ports satisfied by `migrate-adapters`.
//!
//! `migrate` itself never imports `config`/`vcs`/`store` directly — every
//! capability a migration or the lifecycle engine needs is reached through
//! one of these traits.

use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crate::interactive::Decision;
use crate::interactive::Transformation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationError(pub String);

impl MigrationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MigrationError {}

impl From<MigrationError> for kernel::Error {
    fn from(error: MigrationError) -> Self {
        Self::Failed(error.0)
    }
}

/// `migrate`'s own shape, mirroring `config::paths::DocTypeDir`'s fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTypeDir {
    pub doc_type: String,
    pub dir: PathBuf,
}

/// The target-existence-check port migration 0007 needs.
///
/// Declared here, alongside `MigrationContext`'s other capabilities, rather
/// than as `corpus::CorpusIndex` — no such type exists in `corpus`.
pub trait CorpusIndex {
    fn target_exists(&self, target_type: &str, target_id: &str) -> bool;
}

/// The capabilities every migration's `apply()`/`apply_decision()`
/// implementation receives.
///
/// `write` is the only *content-mutation* path available to a migration —
/// routing every write through one method is what lets the path manifest be
/// recorded as a side effect of the call itself, rather than as a discipline
/// each migration author must remember. `merge_move` is the one other
/// mutation primitive (whole-file/whole-directory relocation, needed by the
/// directory-rename and state-relocation migrations); it is not
/// content-writing, so it is not manifest-tracked the same way — the
/// migrations that use it own their own idempotency self-check instead.
///
/// `read`/`list_md_files`/`config_value` default to the empty/absent case so
/// the lifecycle-engine test doubles (`migrate/tests/lifecycle.rs`,
/// `engine.rs`), which never exercise a real migration, do not need updating
/// every time a migration's own needs grow this trait.
pub trait MigrationContext {
    fn doc_type_dirs(&self) -> Vec<DocTypeDir>;
    fn revision(&self) -> Option<String>;
    fn corpus_index(&self) -> &dyn CorpusIndex;

    /// # Errors
    /// [`MigrationError`] when the write is refused or fails.
    fn write(&self, path: &Path, content: &str) -> Result<(), MigrationError>;

    /// The project root every migration-relative path is joined against.
    fn root(&self) -> &Path {
        Path::new(".")
    }

    /// A full-stack (personal-over-team, catalogue-default-falling-back)
    /// config lookup, matching `accelerator config get --allow-legacy-layout
    /// <key> ""` — including keys the catalogue no longer recognises, since
    /// migrations read pre-rename legacy key names.
    ///
    /// `Ok(None)` never means "genuinely unreadable" — an unset key still
    /// resolves `Ok` (to its catalogue default, rendered, possibly empty).
    /// `Err` is reserved for a real config-read failure (a corrupted
    /// `.accelerator/config.md`, for instance) — a config-reading migration
    /// must hard-abort on exactly this, not silently treat corruption as
    /// "key absent."
    ///
    /// # Errors
    /// [`MigrationError`] when the config file itself cannot be read or
    /// parsed.
    fn config_value(
        &self,
        _key: &str,
    ) -> Result<Option<String>, MigrationError> {
        Ok(None)
    }

    /// Like [`Self::config_value`], but `Ok(None)` when `key` resolves only
    /// to its catalogue default rather than an explicit team/personal
    /// override — the "is this pinned?" question migration 0003's
    /// pinned-override warnings need, distinct from "what's the effective
    /// value?". Same `Err` reservation as `config_value`.
    ///
    /// # Errors
    /// [`MigrationError`] when the config file itself cannot be read or
    /// parsed.
    fn configured_path_override(
        &self,
        _key: &str,
    ) -> Result<Option<String>, MigrationError> {
        Ok(None)
    }

    /// `Ok(None)` when `path` does not exist.
    ///
    /// # Errors
    /// [`MigrationError`] when a present file cannot be read.
    fn read(&self, _path: &Path) -> Result<Option<String>, MigrationError> {
        Ok(None)
    }

    /// Whether `path` exists and is a directory.
    fn dir_exists(&self, _path: &Path) -> bool {
        false
    }

    /// Removes `path` if present; a no-op (not an error) when it is absent
    /// — mirrors `rm -f`.
    ///
    /// # Errors
    /// [`MigrationError`] when a present file cannot be removed.
    fn remove_file(&self, _path: &Path) -> Result<(), MigrationError> {
        Ok(())
    }

    /// Removes `path` if it is an empty directory; returns whether it was
    /// removed. A non-empty directory is left in place (`Ok(false)`, not an
    /// error) — mirrors `rmdir 2>/dev/null`'s soft-fail contract.
    ///
    /// # Errors
    /// [`MigrationError`] when `path` exists but is not a directory, or the
    /// removal fails for a reason other than non-emptiness.
    fn remove_dir_if_empty(
        &self,
        _path: &Path,
    ) -> Result<bool, MigrationError> {
        Ok(false)
    }

    /// Every `.md` file under `dir`, recursively, sorted.
    ///
    /// # Errors
    /// [`MigrationError`] when the walk itself fails (an absent `dir` is
    /// `Ok(Vec::new())`, not an error).
    fn list_md_files(
        &self,
        _dir: &Path,
    ) -> Result<Vec<PathBuf>, MigrationError> {
        Ok(Vec::new())
    }

    /// Every file and directory under `dir`, recursively, sorted — the
    /// unfiltered counterpart to [`Self::list_md_files`], for a scaffold
    /// presence check that must see non-`.md` entries too.
    ///
    /// # Errors
    /// [`MigrationError`] when the walk itself fails (an absent `dir` is
    /// `Ok(Vec::new())`, not an error).
    fn list_all_under(
        &self,
        _dir: &Path,
    ) -> Result<Vec<PathBuf>, MigrationError> {
        Ok(Vec::new())
    }

    /// Renders `bare_number` (e.g. `"0001"`) as a canonical work-item ID
    /// under the configured `work.id_pattern`/`work.default_project_code`.
    /// Only migration 0002 calls this; other migrations have no need for
    /// it, so it defaults to an error rather than requiring every test
    /// double to implement pattern compilation.
    ///
    /// # Errors
    /// [`MigrationError`] when the configured pattern is malformed or this
    /// context does not implement pattern rendering.
    fn canonicalise_work_item_id(
        &self,
        _bare_number: &str,
    ) -> Result<String, MigrationError> {
        Err(MigrationError::new(
            "canonicalise_work_item_id is not implemented by this context",
        ))
    }

    /// Runs `accelerator corpus frontmatter validate` in-process (no
    /// subprocess) over `files` — an empty slice validates the whole
    /// configured corpus, matching the CLI's own "no `--dir`/`--file`"
    /// convention. Only migration 0007 calls this, so it defaults to an
    /// error rather than requiring every test double to implement
    /// frontmatter validation.
    ///
    /// # Errors
    /// [`MigrationError`] naming every structural or referential violation
    /// found.
    fn validate_frontmatter(
        &self,
        _files: &[PathBuf],
    ) -> Result<(), MigrationError> {
        Err(MigrationError::new(
            "validate_frontmatter is not implemented by this context",
        ))
    }

    /// Recomputes the `/sync-work-items` change-detection baseline for every
    /// tracked item that was `Synced` before this run, so a whole-corpus
    /// re-render does not spuriously reclassify content-identical items as
    /// locally modified. `pre_migration` carries each rewritten `meta/` file's
    /// pre-render bytes, so an entry's pre-migration digest can still be
    /// computed after the files on disk have been re-rendered. A pre-run
    /// diverged entry keeps its baseline, so its pending push survives.
    /// Returns the number of realigned baselines. Only migration 0008 calls
    /// it.
    ///
    /// # Errors
    /// [`MigrationError`] when a baseline file cannot be read or written.
    fn realign_sync_baseline(
        &self,
        _pre_migration: &[(PathBuf, String)],
    ) -> Result<usize, MigrationError> {
        Ok(0)
    }

    /// Moves `src` onto `dst`, merging directories recursively: an absent
    /// destination is a plain move; a type mismatch or same-named leaf
    /// collision is source-wins; two directories merge entry-by-entry, then
    /// the now-empty source is removed. A no-op when `src` does not exist.
    ///
    /// # Errors
    /// [`MigrationError`] when the destination is unsafe (empty, root, or
    /// path-escaping) or the underlying filesystem operation fails.
    fn merge_move(
        &self,
        _src: &Path,
        _dst: &Path,
    ) -> Result<(), MigrationError> {
        Ok(())
    }
}

/// The applied/skipped ledger's file-backed persistence.
///
/// Ledger files are runner-managed bookkeeping — distinct from
/// `MigrationContext::write`'s manifest-tracked mutation path.
pub trait LedgerStore {
    /// # Errors
    /// [`MigrationError`] when the ledger cannot be read.
    fn applied(&self) -> Result<Vec<String>, MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the ledger cannot be written.
    fn write_applied(&self, ids: &[String]) -> Result<(), MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the skip list cannot be read.
    fn skipped(&self) -> Result<Vec<String>, MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the skip list cannot be written.
    fn write_skipped(&self, ids: &[String]) -> Result<(), MigrationError>;
}

/// A held run-level advisory lock, released on `Drop` by whatever concrete
/// guard the adapter wraps — the domain crate never names that type.
pub struct RunLockGuard(#[allow(dead_code)] Box<dyn std::any::Any>);

impl RunLockGuard {
    pub fn new(guard: impl std::any::Any) -> Self {
        Self(Box::new(guard))
    }
}

/// Held for the whole of a default run — every migration's ledger append,
/// not just the first — so two concurrent `accelerator migrate` invocations
/// never interleave writes.
pub trait RunLock {
    /// # Errors
    /// [`MigrationError`] naming the current holder when acquisition times
    /// out.
    fn acquire(&self) -> Result<RunLockGuard, MigrationError>;
}

/// Every repo-relative path with uncommitted changes under the given root
/// prefixes.
pub trait DirtyPathScanner {
    /// # Errors
    /// [`MigrationError`] when the scan itself fails (not: when it finds
    /// dirt).
    fn dirty_paths(
        &self,
        roots: &[&str],
    ) -> Result<Vec<String>, MigrationError>;
}

/// The per-run path manifest and its run-id sidecar.
///
/// The usability gate is modelled directly in the return shape:
/// `Ok(None)` is "absent, unreadable, or (run-id only) empty" — never
/// distinguished further, since every one of those states resolves toward
/// the same fail-closed treatment.
pub trait ManifestStore {
    /// # Errors
    /// [`MigrationError`] when the manifest is present but unreadable.
    fn manifest(&self) -> Result<Option<Vec<String>>, MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the manifest cannot be written.
    fn write_manifest(&self, paths: &[String]) -> Result<(), MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the manifest cannot be appended to.
    fn append_manifest_path(&self, path: &str) -> Result<(), MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the sidecar is present but unreadable.
    fn run_id(&self) -> Result<Option<String>, MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the sidecar cannot be written.
    fn write_run_id(
        &self,
        revision: Option<&str>,
    ) -> Result<(), MigrationError>;

    /// Deletes both the manifest and its run-id sidecar.
    ///
    /// # Errors
    /// [`MigrationError`] when either cannot be removed.
    fn clear(&self) -> Result<(), MigrationError>;
}

/// The bash-session-log-to-canonical-format cutover's one whole-file rewrite.
///
/// Distinct from `corpus::RecordStore` (which the session log itself is —
/// migrations append/remove through it directly): this is the one-time,
/// unconditional re-canonicalisation that must land in the *same* critical
/// section a concurrent `append_record`/`remove_by_key` call would take, so
/// it participates in the same lock rather than racing it. Parsing and
/// re-composing the JSONL bytes is necessarily adapter-side work — `migrate`
/// carries no JSON dependency — so, unlike every other port here, the
/// implementation reads the current file itself rather than being handed
/// bytes to write; the domain engine only decides *when* to call it (once
/// per run, at first access).
pub trait SessionLogRewriter {
    /// A no-op when `path` does not exist yet.
    ///
    /// # Errors
    /// [`MigrationError`] when a record fails validation (the file is left
    /// byte-unchanged) or the write itself fails.
    fn cutover(&self, path: &Path) -> Result<(), MigrationError>;
}

/// One migration's interactive session log, bound to its own path at
/// construction.
///
/// Timestamping a record is the adapter's job — the domain engine supplies
/// only what it actually decided, not a wall-clock reading, keeping
/// `run_interactive` deterministic and independent of `SystemTime`.
pub trait SessionLog {
    /// # Errors
    /// [`MigrationError`] when the log is present but unreadable or invalid.
    fn records(&self) -> Result<Vec<corpus::Record>, MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the write fails.
    fn append(
        &self,
        key: &str,
        outcome: corpus::Outcome,
        proposed_value: &str,
        user_value: Option<&str>,
    ) -> Result<(), MigrationError>;

    /// # Errors
    /// [`MigrationError`] when the removal fails.
    fn remove_by_key(&self, key: &str) -> Result<(), MigrationError>;
}

/// Each interactive migration owns its own session log, at its own path —
/// this is what binds a fresh [`SessionLog`] to the right one for a given
/// migration id.
pub trait SessionLogFactory {
    fn for_migration(&self, id: &str) -> Box<dyn SessionLog>;
}

/// `Eof` is distinct from `Timeout`.
///
/// It's what a TTY source returns if stdin closes mid-session (the reader
/// thread's channel disconnects rather than timing out) — a real, different
/// code path, not a spare variant. The engine treats it identically to
/// `Timeout`'s terminal contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionError {
    NoInputAvailable,
    Timeout,
    Eof,
}

pub trait DecisionSource {
    /// # Errors
    /// [`DecisionError`] when no decision is available within `timeout`, or
    /// at all.
    fn next_decision(
        &self,
        transformation: &Transformation,
        timeout: Duration,
    ) -> Result<Decision, DecisionError>;
}

/// Selected when stdin is not a TTY and no decisions file was supplied.
///
/// Returns `NoInputAvailable` synchronously on the very first call, without
/// reading or otherwise consulting `timeout` at all — no `Instant`, no
/// sleep, no channel wait. This is what gives the structured-stall path its
/// "timeout never armed" guarantee: the engine's call site is genuinely
/// generic (it always calls `next_decision(t, timeout)` the same way
/// regardless of which source is active), so the guarantee has to come from
/// this implementation, not a call-site special case.
pub struct NoInputDecisionSource;

impl DecisionSource for NoInputDecisionSource {
    fn next_decision(
        &self,
        _transformation: &Transformation,
        _timeout: Duration,
    ) -> Result<Decision, DecisionError> {
        Err(DecisionError::NoInputAvailable)
    }
}

pub struct PreviewEntry<'a> {
    pub id: &'a str,
    pub description: &'a str,
}

/// Reports lifecycle events for the caller to render.
///
/// Every user-facing string this engine emits is rendered by the caller
/// (`migrate-cli`'s `render` module owns the exact bash-parity literals) —
/// `migrate` itself only reports *which* event happened and in what order,
/// preserving bash's interleaved stdout/stderr sequencing without the domain
/// crate depending on `migrate-cli`.
pub trait Reporter {
    fn preview(&self, pending: &[PreviewEntry<'_>]);
    fn no_pending_migrations(&self, skipped: &[String]);
    fn unknown_applied_id(&self, id: &str);
    fn unknown_skipped_id(&self, id: &str);
    fn applied_and_skipped(&self, id: &str);
    fn migration_running(&self, id: &str);
    fn migration_applied(&self, id: &str);
    fn migration_no_op(&self, id: &str);
    fn migration_failed(&self, id: &str, error: &MigrationError);
    fn summary(
        &self,
        applied: usize,
        skipped: &[String],
        pending_remaining: usize,
    );

    /// A predicate's `Fail(message)`, relayed verbatim — the message is
    /// NOT re-wrapped.
    fn interactive_fail(&self, id: &str, message: &str);

    /// A `validate_edit` rejection: `"[interactive] {message}"`.
    fn interactive_validation_rejected(&self, message: &str);

    /// The structured stall: no decision input was available for `id`,
    /// and `pending_keys` names every undecided transformation from the
    /// stalled one onward, in emission order.
    fn interactive_stalled(&self, id: &str, pending_keys: &[String]);

    /// A `DecisionSource::Timeout`/`Eof`.
    fn interactive_timeout(&self, id: &str);
}
