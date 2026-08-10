//! Every user-facing string this binary emits is reproduced byte-for-byte
//! to match already-captured golden fixtures.

use std::fmt::Write as _;
use std::path::Path;

use migrate::list::ListGroup;
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

/// `--list`: the multi-migration/in-flight-session stderr notes, then
/// every group's tab-delimited lines
/// (`<pos>\t<key>\t<proposed>\t<path>:<anchor>`), segmented by a
/// `# migration <id>` header — and position restarting at 1 — only when
/// more than one interactive migration is pending. `"no pending
/// transformations"` (stdout, lowercase, no trailing punctuation) when
/// nothing was emitted at all.
pub fn render_list(root: &Path, groups: &[ListGroup]) {
    let multi = groups.len() > 1;
    if multi {
        eprintln!(
            "Note: {} interactive migrations pending; resume one at a time \
             with --decisions-file per '# migration <id>' section — a \
             single multi-migration decisions file is not yet supported.",
            groups.len()
        );
    }
    for group in groups {
        if group.had_in_flight_session {
            let path =
                migrate_adapters::session_log::session_log_path(root, group.id);
            eprintln!(
                "Note: migration {} has an in-flight session log; --list \
                 shows only the remaining (undecided) transformations. \
                 Re-run /accelerator:migrate to resume, or rm {} to \
                 discard.",
                group.id,
                path.display()
            );
        }
    }

    let mut emitted = 0usize;
    for group in groups {
        if multi {
            println!("# migration {}", group.id);
        }
        for (index, transformation) in group.transformations.iter().enumerate()
        {
            emitted += 1;
            println!(
                "{}\t{}\t{}\t{}:{}",
                index + 1,
                transformation.short_key(),
                transformation.proposed,
                transformation.path,
                transformation.anchor
            );
        }
    }
    if emitted == 0 {
        println!("no pending transformations");
    }
}

pub struct StdoutReporter {
    pub root: std::path::PathBuf,
}

fn decisions_path(root: &Path, id: &str) -> std::path::PathBuf {
    root.join(format!(".accelerator/state/migrations-{id}-decisions.txt"))
}

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

    fn interactive_fail(&self, id: &str, message: &str) {
        eprintln!("[{id}] {message}");
    }

    fn interactive_validation_rejected(&self, message: &str) {
        eprintln!("[interactive] {message}");
    }

    fn interactive_stalled(&self, id: &str, pending_keys: &[String]) {
        let path = decisions_path(&self.root, id);
        let path = path.display();
        eprintln!("[{id}] MIGRATION STALLED: no decision input available");
        for key in pending_keys {
            eprintln!("[{id}]   pending decision: {key}");
        }
        eprintln!(
            "[{id}]   No decisions file, terminal, or piped input was available to"
        );
        eprintln!(
            "[{id}]   answer this prompt, so the migration cannot proceed."
        );
        eprintln!("[{id}]");
        eprintln!(
            "[{id}]   This migration may have already partially modified the"
        );
        eprintln!("[{id}]   working tree. Re-running /accelerator:migrate resumes this");
        eprintln!(
            "[{id}]   partial run when the base revision is unchanged (decided"
        );
        eprintln!("[{id}]   transformations are replayed, not re-applied).");
        eprintln!("[{id}]");
        eprintln!(
            "[{id}]   To resume: each run answers the current prompt only (you"
        );
        eprintln!(
            "[{id}]   may be stalled again for the next undecided transformation):"
        );
        eprintln!(
            "[{id}]     1. write the decision (accept | skip | edit <value>),"
        );
        eprintln!("[{id}]        one per line, to: {path}");
        eprintln!(
            "[{id}]        (create this file yourself; do not overwrite existing"
        );
        eprintln!("[{id}]        migrations-{id}-* state files)");
        eprintln!("[{id}]     2. then run (copy-pasteable):");
        eprintln!();
        eprintln!("accelerator migrate --decisions-file {path}");
        eprintln!();
        eprintln!("[{id}]   equivalent env-var form:");
        eprintln!();
        eprintln!(
            "ACCELERATOR_MIGRATE_DECISIONS_FILE={path} accelerator migrate"
        );
    }

    fn interactive_timeout(&self, id: &str) {
        eprintln!("[{id}] timed out waiting for a decision");
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
