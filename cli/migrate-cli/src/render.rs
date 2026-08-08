//! Every user-facing string this binary emits, reproduced byte-for-byte from
//! `run-migrations.sh` (the `accelerator migrate` invocation form replacing
//! `bash $0`/`run-migrations.sh` per the port's fixed normalisation rule).

use std::fmt::Write as _;
use std::path::Path;

use migrate::ports::MigrationError;
use migrate::ports::PreviewEntry;
use migrate::ports::Reporter;
use migrate::preflight::AffordanceEntry;

const PREAMBLE: &str = "\nMigrations rewrite files and may make repo-wide changes; commit\nyour working tree before running so VCS revert is available as\nrollback. The pre-flight will refuse to run on a dirty tree\nunless ACCELERATOR_MIGRATE_FORCE=1 is set.\n\n";

pub const DIRTY_TREE_REFUSAL: &str = "Error: dirty working tree — uncommitted changes detected in meta/, .claude/accelerator*.md, or .accelerator/.\nCommit or discard those changes first, or set ACCELERATOR_MIGRATE_FORCE=1 to skip this check.";

pub fn resume_affordance(root: &Path, affordance: &[AffordanceEntry]) {
    eprintln!("Resuming over this run's own partial migration output:");
    for entry in affordance {
        eprintln!("  {}", entry.path);
        if let Some(count) = entry.session_log_decision_count {
            let abs = root.join(&entry.path);
            eprintln!(
                "    interactive migration — resuming: replays {count} \
                 decided transformation(s) and re-prompts only undecided ones"
            );
            eprintln!(
                "    (with no decisions channel it re-stalls — resume \
                 non-interactively via --decisions-file)."
            );
            eprintln!(
                "    To discard instead: rm {}  (loses {count} decisions)",
                abs.display()
            );
        }
    }
}

pub struct StdoutReporter;

impl Reporter for StdoutReporter {
    fn preview(&self, pending: &[PreviewEntry<'_>]) {
        println!("About to apply {} migration(s):", pending.len());
        for entry in pending {
            println!("  {} — {}", entry.id, entry.description);
            println!("    To skip: accelerator migrate --skip {}", entry.id);
        }
        print!("{PREAMBLE}");
    }

    fn no_pending_migrations(&self, skipped: &[String]) {
        println!("No pending migrations.");
        if !skipped.is_empty() {
            let joined = skipped.iter().fold(String::new(), |mut acc, id| {
                let _ = write!(acc, "{id} ");
                acc
            });
            println!("Skipped: {joined}");
        }
    }

    fn unknown_applied_id(&self, id: &str) {
        eprintln!(
            "[warning] migrations-applied references unknown migration \
             {id} — preserved on rewrite"
        );
    }

    fn unknown_skipped_id(&self, id: &str) {
        eprintln!(
            "[warning] migrations-skipped references unknown migration \
             {id} — preserved on rewrite"
        );
    }

    fn applied_and_skipped(&self, id: &str) {
        eprintln!(
            "[warning] migration {id} appears in BOTH .migrations-applied \
             and .migrations-skipped — applied takes precedence"
        );
    }

    fn migration_running(&self, id: &str) {
        eprintln!("[{id}] running");
    }

    fn migration_applied(&self, id: &str) {
        eprintln!("[{id}] applied");
    }

    fn migration_no_op(&self, id: &str) {
        eprintln!("[{id}] no-op (stays pending)");
    }

    fn migration_failed(&self, id: &str, error: &MigrationError) {
        let message = error.to_string();
        if !message.is_empty() {
            eprintln!("{message}");
        }
        eprintln!("[{id}] failed");
    }

    fn summary(
        &self,
        applied: usize,
        skipped: &[String],
        pending_remaining: usize,
    ) {
        let mut summary = format!("applied: {applied}");
        if !skipped.is_empty() {
            let joined = skipped
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            let _ = write!(summary, "; skipped: {joined}");
        }
        if pending_remaining > 0 {
            let _ = write!(summary, "; pending (no-op): {pending_remaining}");
        }
        println!();
        println!("Migration complete. {summary}.");
    }
}
