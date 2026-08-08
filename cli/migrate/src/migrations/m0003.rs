//! Port of `0003-relocate-accelerator-state.sh`.
//!
//! Relocates Accelerator-owned files from `.claude/`/`meta/` onto
//! `.accelerator/`: the scaffold (`.accelerator/.gitignore`,
//! `.accelerator/state/`), the root `.gitignore` rewrite, the six
//! source→destination moves, the legacy `meta/.migrations-*` merge, and the
//! Jira integration's inner `.gitignore`.

use std::path::Path;

use crate::migrations::text::filter_lines;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0003;

impl MigrationMeta for Migration0003 {
    fn id(&self) -> &'static str {
        "0003-relocate-accelerator-state"
    }

    fn description(&self) -> &'static str {
        "Relocate Accelerator-owned files from .claude/ and meta/ to \
         .accelerator/"
    }
}

const JIRA_INNER_GITIGNORE_RULES: [&str; 3] =
    ["site.json", ".refresh-meta.json", ".lock/"];

const LEGACY_ROOT_GITIGNORE_PATTERNS: [&str; 6] = [
    ".claude/accelerator.local.md",
    "/.claude/accelerator.local.md",
    "meta/integrations/jira/.lock",
    "/meta/integrations/jira/.lock",
    "meta/integrations/jira/.refresh-meta.json",
    "/meta/integrations/jira/.refresh-meta.json",
];

impl Migration for Migration0003 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let root = ctx.root().to_path_buf();

        if is_no_op_pending(ctx, &root)? {
            return Ok(ApplyOutcome::NoOpPending);
        }

        init_scaffold(ctx, &root)?;
        rewrite_root_gitignore(ctx, &root)?;
        warn_pinned_overrides(ctx);
        move_sources(ctx, &root)?;
        relocate_state_files(ctx, &root)?;
        inner_jira_gitignore(ctx, &root)?;

        Ok(ApplyOutcome::Applied)
    }
}

const LEGACY_SOURCE_RELATIVE_PATHS: [&str; 8] = [
    ".claude/accelerator.md",
    ".claude/accelerator.local.md",
    ".claude/accelerator/skills",
    ".claude/accelerator/lenses",
    "meta/templates",
    "meta/integrations",
    "meta/.migrations-applied",
    "meta/.migrations-skipped",
];

fn is_no_op_pending(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<bool, MigrationError> {
    let has_source = LEGACY_SOURCE_RELATIVE_PATHS
        .iter()
        .any(|relative| exists(ctx, &root.join(relative)));

    let has_source = has_source
        || (ctx.configured_path_override("paths.tmp").is_none()
            && exists(ctx, &root.join("meta/tmp")));

    if has_source {
        return Ok(false);
    }

    if !ctx.dir_exists(&root.join(".accelerator")) {
        return Ok(true);
    }
    for path in ctx.list_all_under(&root.join(".accelerator"))? {
        let Ok(relative) = path.strip_prefix(root.join(".accelerator")) else {
            continue;
        };
        match relative.to_string_lossy().as_ref() {
            ".gitignore" | "state" | "state/.gitkeep" => {}
            _ => return Ok(false),
        }
    }
    Ok(true)
}

fn exists(ctx: &dyn MigrationContext, path: &Path) -> bool {
    ctx.dir_exists(path)
        || ctx.read(path).is_ok_and(|content| content.is_some())
}

/// `touch`-then-append-each-rule-if-absent: creates `path` (empty) when it
/// does not exist, whether or not `lines` is empty, then appends any of
/// `lines` not already present as a whole line — mirrors
/// `accelerator_ensure_inner_gitignore`/`accelerator_ensure_root_gitignore_rule`'s
/// `touch` + `grep -qFx || append` shape.
fn ensure_lines(
    ctx: &dyn MigrationContext,
    path: &Path,
    lines: &[&str],
) -> Result<(), MigrationError> {
    let existing = ctx.read(path)?;
    let content = existing.clone().unwrap_or_default();
    let mut updated = content.clone();
    for line in lines {
        let already_present =
            content.lines().any(|existing_line| existing_line == *line);
        if !already_present {
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(line);
            updated.push('\n');
        }
    }
    if existing.is_none() || updated != content {
        ctx.write(path, &updated)?;
    }
    Ok(())
}

fn init_scaffold(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    ensure_lines(
        ctx,
        &root.join(".accelerator/.gitignore"),
        &["config.local.md", ".tmp-*"],
    )?;
    ensure_lines(ctx, &root.join(".accelerator/state/.gitkeep"), &[])?;
    Ok(())
}

fn rewrite_root_gitignore(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let gi = root.join(".gitignore");
    if let Some(content) = ctx.read(&gi)? {
        for line in content.lines() {
            let trimmed = line.trim_end();
            for pattern in LEGACY_ROOT_GITIGNORE_PATTERNS {
                if trimmed.starts_with(pattern) && trimmed != pattern {
                    return Err(MigrationError::new(format!(
                        "accelerator migrate: refusing to rewrite '{line}' \
                         in {} —\nline matches a legacy rule but has \
                         trailing content.\nReconcile manually and \
                         re-run.",
                        gi.display()
                    )));
                }
            }
        }
        let rewritten = filter_lines(&content, |line| {
            !LEGACY_ROOT_GITIGNORE_PATTERNS.contains(&line.trim_end())
        });
        if rewritten != content {
            ctx.write(&gi, &rewritten)?;
        }
    }
    ensure_lines(ctx, &gi, &[".accelerator/config.local.md"])?;
    Ok(())
}

/// Bash's `log_warn` only ever prints its first argument — the migration
/// calls it with four, so only the first is observable. Reproduced exactly,
/// not "fixed" into the evidently-intended full sentence.
fn warn_pinned_overrides(ctx: &dyn MigrationContext) {
    if let Some(value) = ctx.configured_path_override("paths.templates") {
        eprintln!(
            "Warning: paths.templates is explicitly set to '{value}'. The \
             migration"
        );
    }
    if let Some(value) = ctx.configured_path_override("paths.integrations") {
        eprintln!(
            "Warning: paths.integrations is explicitly set to '{value}'. \
             The migration"
        );
    }
}

const MOVE_PAIRS: [(&str, &str); 6] = [
    (".claude/accelerator.md", ".accelerator/config.md"),
    (
        ".claude/accelerator.local.md",
        ".accelerator/config.local.md",
    ),
    (".claude/accelerator/skills", ".accelerator/skills"),
    (".claude/accelerator/lenses", ".accelerator/lenses"),
    ("meta/templates", ".accelerator/templates"),
    (
        "meta/integrations/jira",
        ".accelerator/state/integrations/jira",
    ),
];

fn move_sources(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let tmp_configured = ctx.configured_path_override("paths.tmp").is_some();

    for (source, destination) in MOVE_PAIRS {
        ctx.merge_move(&root.join(source), &root.join(destination))?;
    }
    if !tmp_configured {
        ctx.merge_move(&root.join("meta/tmp"), &root.join(".accelerator/tmp"))?;
    }
    Ok(())
}

fn merge_state_file(
    ctx: &dyn MigrationContext,
    root: &Path,
    source_relative: &str,
    destination_name: &str,
) -> Result<(), MigrationError> {
    let source = root.join(source_relative);
    let Some(source_content) = ctx.read(&source)? else {
        return Ok(());
    };
    let destination = root.join(".accelerator/state").join(destination_name);
    let destination_content = ctx.read(&destination)?.unwrap_or_default();

    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for line in destination_content
        .lines()
        .chain(source_content.lines())
        .filter(|line| !line.is_empty())
    {
        if seen.insert(line) {
            unique.push(line);
        }
    }
    let mut merged = unique.join("\n");
    if !merged.is_empty() {
        merged.push('\n');
    }
    ctx.write(&destination, &merged)?;
    ctx.remove_file(&source)?;
    Ok(())
}

fn relocate_state_files(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    merge_state_file(
        ctx,
        root,
        "meta/.migrations-applied",
        "migrations-applied",
    )?;
    merge_state_file(
        ctx,
        root,
        "meta/.migrations-skipped",
        "migrations-skipped",
    )?;
    Ok(())
}

fn inner_jira_gitignore(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let jira_dir = root.join(".accelerator/state/integrations/jira");
    if !ctx.dir_exists(&jira_dir) {
        return Ok(());
    }
    ensure_lines(
        ctx,
        &jira_dir.join(".gitignore"),
        &JIRA_INNER_GITIGNORE_RULES,
    )?;
    ensure_lines(ctx, &jira_dir.join(".gitkeep"), &[])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::{
        ensure_lines, is_no_op_pending, rewrite_root_gitignore,
        warn_pinned_overrides, Migration0003,
    };
    use crate::ports::MigrationContext;
    use crate::ports::MigrationError;
    use crate::registry::{Migration, MigrationMeta};

    type TestResult = Result<(), MigrationError>;

    #[derive(Default)]
    struct FakeContext {
        root: PathBuf,
        files: RefCell<BTreeMap<PathBuf, String>>,
        dirs: RefCell<std::collections::BTreeSet<PathBuf>>,
        configured: BTreeMap<String, String>,
    }

    impl FakeContext {
        fn new() -> Self {
            Self {
                root: PathBuf::from("/repo"),
                ..Self::default()
            }
        }

        fn with_file(self, relative: &str, content: &str) -> Self {
            let path = self.root.join(relative);
            self.files.borrow_mut().insert(path, content.to_owned());
            self
        }

        fn with_dir(self, relative: &str) -> Self {
            self.dirs.borrow_mut().insert(self.root.join(relative));
            self
        }

        fn with_configured(mut self, key: &str, value: &str) -> Self {
            self.configured.insert(key.to_owned(), value.to_owned());
            self
        }
    }

    impl MigrationContext for FakeContext {
        fn doc_type_dirs(&self) -> Vec<crate::ports::DocTypeDir> {
            Vec::new()
        }

        fn revision(&self) -> Option<String> {
            None
        }

        fn corpus_index(&self) -> &dyn crate::ports::CorpusIndex {
            struct NoIndex;
            impl crate::ports::CorpusIndex for NoIndex {
                fn target_exists(&self, _: &str, _: &str) -> bool {
                    false
                }
            }
            &NoIndex
        }

        fn write(
            &self,
            path: &std::path::Path,
            content: &str,
        ) -> Result<(), crate::ports::MigrationError> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), content.to_owned());
            Ok(())
        }

        fn root(&self) -> &std::path::Path {
            &self.root
        }

        fn configured_path_override(&self, key: &str) -> Option<String> {
            self.configured.get(key).cloned()
        }

        fn read(
            &self,
            path: &std::path::Path,
        ) -> Result<Option<String>, crate::ports::MigrationError> {
            Ok(self.files.borrow().get(path).cloned())
        }

        fn dir_exists(&self, path: &std::path::Path) -> bool {
            self.dirs.borrow().contains(path)
        }

        fn list_all_under(
            &self,
            dir: &std::path::Path,
        ) -> Result<Vec<PathBuf>, crate::ports::MigrationError> {
            Ok(self
                .files
                .borrow()
                .keys()
                .filter(|path| path.starts_with(dir))
                .cloned()
                .collect())
        }
    }

    #[test]
    fn ensure_lines_creates_an_absent_file_even_with_no_lines() -> TestResult {
        let ctx = FakeContext::new();
        ensure_lines(&ctx, &ctx.root.join(".accelerator/.gitignore"), &[])?;
        assert_eq!(
            ctx.files
                .borrow()
                .get(&ctx.root.join(".accelerator/.gitignore")),
            Some(&String::new())
        );
        Ok(())
    }

    #[test]
    fn ensure_lines_appends_only_missing_rules() -> TestResult {
        let ctx = FakeContext::new()
            .with_file(".accelerator/.gitignore", "config.local.md\n");
        ensure_lines(
            &ctx,
            &ctx.root.join(".accelerator/.gitignore"),
            &["config.local.md", ".tmp-*"],
        )?;
        assert_eq!(
            ctx.files
                .borrow()
                .get(&ctx.root.join(".accelerator/.gitignore")),
            Some(&"config.local.md\n.tmp-*\n".to_owned())
        );
        Ok(())
    }

    #[test]
    fn preflight_is_no_op_when_nothing_legacy_and_accelerator_is_scaffold_only(
    ) -> TestResult {
        let ctx = FakeContext::new()
            .with_dir(".accelerator")
            .with_file(".accelerator/.gitignore", "config.local.md\n")
            .with_file(".accelerator/state/.gitkeep", "");
        assert!(is_no_op_pending(&ctx, &ctx.root)?);
        Ok(())
    }

    #[test]
    fn preflight_proceeds_when_a_legacy_source_exists() -> TestResult {
        let ctx = FakeContext::new()
            .with_file(".claude/accelerator.md", "---\n---\n");
        assert!(!is_no_op_pending(&ctx, &ctx.root)?);
        Ok(())
    }

    #[test]
    fn preflight_proceeds_when_accelerator_has_extra_content() -> TestResult {
        let ctx = FakeContext::new()
            .with_dir(".accelerator")
            .with_file(".accelerator/.gitignore", "config.local.md\n")
            .with_file(".accelerator/extra.txt", "surprise\n");
        assert!(!is_no_op_pending(&ctx, &ctx.root)?);
        Ok(())
    }

    #[test]
    fn root_gitignore_removes_legacy_rules_and_adds_the_new_one() -> TestResult
    {
        let ctx = FakeContext::new().with_file(
            ".gitignore",
            "node_modules/\n.claude/accelerator.local.md\nfoo.txt\n",
        );
        rewrite_root_gitignore(&ctx, &ctx.root)?;
        assert_eq!(
            ctx.files.borrow().get(&ctx.root.join(".gitignore")),
            Some(
                &"node_modules/\nfoo.txt\n.accelerator/config.local.md\n"
                    .to_owned()
            )
        );
        Ok(())
    }

    #[test]
    fn root_gitignore_refuses_a_legacy_line_with_trailing_content() {
        let ctx = FakeContext::new().with_file(
            ".gitignore",
            ".claude/accelerator.local.md # keep this comment\n",
        );
        assert!(rewrite_root_gitignore(&ctx, &ctx.root).is_err());
    }

    #[test]
    fn migration_metadata_matches_bash() {
        assert_eq!(Migration0003.id(), "0003-relocate-accelerator-state");
        let _: &dyn Migration = &Migration0003;
    }

    #[test]
    fn warn_pinned_overrides_prints_only_bash_own_truncated_message() {
        // No panics/errors — the printed text is asserted by the
        // black-box fixture test, since this fn only writes to stderr.
        let ctx = FakeContext::new()
            .with_configured("paths.templates", "custom/templates");
        warn_pinned_overrides(&ctx);
    }
}
