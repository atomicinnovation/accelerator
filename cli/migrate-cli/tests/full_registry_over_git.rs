//! The whole real registry chained over a legacy layout committed to a git
//! repository, pinned byte for byte: the per-migration progress log, the
//! migrated work item, the relocated scaffold, and — when the chain halts —
//! the run's own bookkeeping.
#![cfg(feature = "bash-parity")]

mod common;

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use common::run;
use common::tempdir;
use common::write;
use common::Outcome;
use common::TestError;
use common::MANIFEST_FILE;
use common::RUN_BASE_FILE;
use tempfile::TempDir;
use vcs_test_support::hermetic::Hermetic;

const LEGACY_CONFIG: &str = "---\npaths:\n  tickets: meta/tickets\n---\n";

const UNTYPED_TICKET: &str = "---\nticket_id: 0001\n---\n\n# 0001: Foo\n";

const TYPED_TICKET: &str = "---\nticket_id: 0001\ntitle: Foo\ntype: task\n\
     status: draft\npriority: medium\ndate: 2026-01-01\nauthor: a\n---\n\n\
     # 0001: Foo\n";

const TICKET_REVIEW: &str =
    "---\ntype: work-item-review\n---\n\n# foo-review-1\n";

const RELOCATED_GITIGNORE: &str = "config.local.md\n.tmp-*\n";

const NO_PROJECT_PATTERN_LOG: [&str; 4] = [
    "[0001-rename-tickets-to-work] running",
    "[0001-rename-tickets-to-work] applied",
    "[0002-rename-work-items-with-project-prefix] running",
    "[0002-rename-work-items-with-project-prefix] no-op (stays pending)",
];

const RELOCATION_AND_RESEARCH_LOG: [&str; 4] = [
    "[0003-relocate-accelerator-state] running",
    "[0003-relocate-accelerator-state] applied",
    "[0004-restructure-meta-research-into-subject-subcategories] running",
    "[0004-restructure-meta-research-into-subject-subcategories] applied",
];

const MISSING_PLANS_AND_RESEARCH_LOG: [&str; 8] = [
    "[0006-canonicalise-work-item-id-and-author] running",
    "Warning: 0006: plans directory does not exist: meta/plans",
    "0006: rewrote 0 file(s) under meta/plans",
    "Warning: 0006: research_codebase directory does not exist: \
     meta/research/codebase",
    "0006: rewrote 0 file(s) under meta/research/codebase",
    "Warning: 0006: research_issues directory does not exist: \
     meta/research/issues",
    "0006: rewrote 0 file(s) under meta/research/issues",
    "[0006-canonicalise-work-item-id-and-author] applied",
];

const PRECONDITION_REFUSAL: &str = "[0007-unify-meta-corpus-frontmatter] \
     precondition pre-pass refused — zero files mutated — resolve the \
     refusals above (or revert meta/ via your VCS), then re-run";

fn legacy_git_repository(
    tag: &str,
    ticket: &str,
    review: Option<&str>,
) -> Result<(TempDir, Hermetic, PathBuf), TestError> {
    vcs_test_support::hermetic::assert_git_is_recent_enough()?;
    let work = tempdir(tag)?;
    let env = Hermetic::rooted_at(work.path())?;
    let root = work.path().join("repo");
    fs::create_dir_all(&root)?;
    env.git(&["init", "--quiet"], &root)?;
    write(&root, "meta/tickets/0001-foo.md", ticket)?;
    if let Some(review) = review {
        write(&root, "meta/reviews/tickets/foo-review-1.md", review)?;
    }
    write(&root, ".claude/accelerator.md", LEGACY_CONFIG)?;
    env.git(&["add", "--all"], &root)?;
    env.git(&["commit", "--quiet", "-m", "legacy layout"], &root)?;
    Ok((work, env, root))
}

fn migrate_quietly(env: &Hermetic, root: &Path) -> Result<Outcome, TestError> {
    run(env, root, &[], &[("ACCELERATOR_LOG", "off")])
}

fn lines(log: &[&str]) -> String {
    log.iter().flat_map(|line| [*line, "\n"]).collect()
}

fn read(root: &Path, relative: &str) -> Result<String, TestError> {
    Ok(fs::read_to_string(root.join(relative))?)
}

#[test]
fn a_typed_ticket_reaches_the_unified_schema_byte_for_byte(
) -> Result<(), TestError> {
    let (_work, env, root) =
        legacy_git_repository("typed-ticket", TYPED_TICKET, None)?;

    let outcome = migrate_quietly(&env, &root)?;

    assert_eq!(outcome.code, 0, "{}", outcome.stderr);
    assert!(
        outcome.stdout.ends_with(
            "\nMigration complete. applied: 9; pending (no-op): 1.\n"
        ),
        "{}",
        outcome.stdout
    );
    let expected_log = [
        lines(&NO_PROJECT_PATTERN_LOG),
        lines(&RELOCATION_AND_RESEARCH_LOG),
        lines(&[
            "[0005-rename-work-item-type-to-kind] running",
            "0005: rewrote 1 file(s) under meta/work",
            "[0005-rename-work-item-type-to-kind] applied",
        ]),
        lines(&MISSING_PLANS_AND_RESEARCH_LOG),
        lines(&[
            "[0007-unify-meta-corpus-frontmatter] running",
            "[0007-unify-meta-corpus-frontmatter] applied",
            "[0008-canonical-frontmatter-quoting] running",
            "0008: 2 file(s) re-rendered, 0 with dropped comments/CRLF, 0 \
             sync baseline(s) realigned — revert this migration commit to \
             recover",
            "[0008-canonical-frontmatter-quoting] applied",
            "[0009-split-work-key-from-tracker-scope-key] running",
            "[0009-split-work-key-from-tracker-scope-key] applied",
            "[0010-strip-research-title-prefix] running",
            "Warning: 0010: research_codebase directory does not exist: \
             meta/research/codebase",
            "0010: rewrote 0 file(s) under meta/research/codebase",
            "[0010-strip-research-title-prefix] applied",
        ]),
    ]
    .concat();
    assert_eq!(outcome.stderr, expected_log);

    assert_eq!(
        read(&root, "meta/work/0001-foo.md")?,
        "---\nid: \"0001\"\ntitle: \"Foo\"\nkind: \"task\"\n\
         status: \"draft\"\npriority: \"medium\"\n\
         date: \"2026-01-01T00:00:00+00:00\"\nauthor: \"a\"\n\
         type: \"work-item\"\ntags: []\nschema_version: 1\n\
         last_updated: \"2026-01-01T00:00:00+00:00\"\n\
         last_updated_by: \"a\"\n---\n\n# 0001: Foo\n"
    );
    assert_eq!(
        read(&root, ".accelerator/config.md")?,
        "---\npaths:\n  work: \"meta/work\"\n---\n"
    );
    assert_eq!(read(&root, ".accelerator/.gitignore")?, RELOCATED_GITIGNORE);
    assert_eq!(
        common::applied_ledger(&root)?,
        "0001-rename-tickets-to-work\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n\
         0007-unify-meta-corpus-frontmatter\n\
         0008-canonical-frontmatter-quoting\n\
         0009-split-work-key-from-tracker-scope-key\n\
         0010-strip-research-title-prefix\n"
    );
    assert!(!root.join(MANIFEST_FILE).exists());
    assert!(!root.join(RUN_BASE_FILE).exists());
    Ok(())
}

#[test]
fn an_untyped_ticket_halts_the_chain_at_the_unified_schema(
) -> Result<(), TestError> {
    let (_work, env, root) = legacy_git_repository(
        "untyped-ticket",
        UNTYPED_TICKET,
        Some(TICKET_REVIEW),
    )?;
    let head = env.git(&["rev-parse", "HEAD"], &root)?;

    let outcome = migrate_quietly(&env, &root)?;

    assert_eq!(outcome.code, 1, "{}", outcome.stderr);
    assert!(
        !outcome.stdout.contains("Migration complete."),
        "{}",
        outcome.stdout
    );
    let expected_log = [
        lines(&NO_PROJECT_PATTERN_LOG),
        lines(&RELOCATION_AND_RESEARCH_LOG),
        lines(&[
            "[0005-rename-work-item-type-to-kind] running",
            "0005: rewrote 0 file(s) under meta/work",
            "[0005-rename-work-item-type-to-kind] applied",
        ]),
        lines(&MISSING_PLANS_AND_RESEARCH_LOG),
        lines(&[
            "[0007-unify-meta-corpus-frontmatter] running",
            "0007-REFUSE: meta/work/0001-foo.md — work-item missing kind: \
             (run migration 0005 first)",
            PRECONDITION_REFUSAL,
            PRECONDITION_REFUSAL,
            "[0007-unify-meta-corpus-frontmatter] failed",
            PRECONDITION_REFUSAL,
        ]),
    ]
    .concat();
    assert_eq!(outcome.stderr, expected_log);

    assert_eq!(
        read(&root, "meta/work/0001-foo.md")?,
        "---\nwork_item_id: 0001\n---\n\n# 0001: Foo\n"
    );
    assert_eq!(
        read(&root, "meta/reviews/work/foo-review-1.md")?,
        TICKET_REVIEW
    );
    assert_eq!(
        read(&root, ".accelerator/config.md")?,
        "---\npaths:\n  work: meta/work\n---\n"
    );
    assert_eq!(read(&root, ".accelerator/.gitignore")?, RELOCATED_GITIGNORE);
    assert_eq!(
        common::applied_ledger(&root)?,
        "0001-rename-tickets-to-work\n\
         0003-relocate-accelerator-state\n\
         0004-restructure-meta-research-into-subject-subcategories\n\
         0005-rename-work-item-type-to-kind\n\
         0006-canonicalise-work-item-id-and-author\n"
    );
    assert_eq!(
        read(&root, MANIFEST_FILE)?,
        "meta/tickets/0001-foo.md\n\
         .claude/accelerator.md\n\
         .accelerator/.gitignore\n\
         .accelerator/state/.gitkeep\n\
         .gitignore\n"
    );
    assert_eq!(read(&root, RUN_BASE_FILE)?.trim_end(), head);
    Ok(())
}
