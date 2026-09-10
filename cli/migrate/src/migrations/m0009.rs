//! Materialises the split configuration key model on disk.
//!
//! Renames the deprecated `work.default_project_code` into its two successors —
//! the work-owned local ID prefix `work.key` (only when `id_pattern` references
//! the prefix token) and the integration-owned scope key (`jira.project_key` /
//! `linear.team_key`, per the active `work.integration`) — and rewrites a
//! `{project}` `id_pattern` to `{key}`. Rewrites in place at the config level
//! each key is found, so a team value stays team and a personal value stays
//! personal.
//!
//! The read-time alias (shipped alongside `work.key`) keeps every legacy config
//! working; this migration is the durability step that lets the alias be deleted
//! in the following release.

// DSL token markers such as "{project}" and "{key}" are literals, not format
// args.
#![allow(clippy::literal_string_with_formatting_args)]

use std::path::Path;

use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0009;

impl MigrationMeta for Migration0009 {
    fn id(&self) -> &'static str {
        "0009-split-work-key-from-tracker-scope-key"
    }

    fn description(&self) -> &'static str {
        "Materialise work.key and the tracker scope key from the deprecated \
         work.default_project_code, and rewrite a {project} id_pattern to \
         {key}."
    }
}

const CONFIG_FILES: [&str; 2] = ["config.md", "config.local.md"];
const PERSONAL_FILE: &str = "config.local.md";

/// The integration-owned scope key a `work.integration` value materialises into.
fn scope_key_for(integration: &str) -> Option<(&'static str, &'static str)> {
    match integration {
        "jira" => Some(("jira", "project_key")),
        "linear" => Some(("linear", "team_key")),
        _ => None,
    }
}

impl Migration for Migration0009 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let integration =
            ctx.config_value("work.integration")?.unwrap_or_default();
        let id_pattern =
            ctx.config_value("work.id_pattern")?.unwrap_or_default();
        let pattern_uses_prefix = corpus::references_key(&id_pattern);

        let legacy_pinned =
            ctx.configured_path_override("work.default_project_code")?;
        let work_key_pinned = ctx.configured_path_override("work.key")?;
        if let (Some(legacy), Some(work_key)) =
            (&legacy_pinned, &work_key_pinned)
        {
            if legacy != work_key {
                return Err(MigrationError::new(
                    "0009: mixed-state config — work.default_project_code \
                     and work.key are both set with divergent values. \
                     Resolve manually (remove work.default_project_code) and \
                     retry.",
                ));
            }
        }

        let scope_key = scope_key_for(&integration);
        let scope_key_pinned = match scope_key {
            Some((prefix, key)) => ctx
                .configured_path_override(&format!("{prefix}.{key}"))?
                .is_some(),
            None => false,
        };

        let root = ctx.root().to_path_buf();
        for file in CONFIG_FILES {
            let path = root.join(".accelerator").join(file);
            let is_personal = file == PERSONAL_FILE;
            let Some(original) = ctx.read(&path)? else {
                continue;
            };
            let mut content = original.clone();

            if let Some(pattern) =
                probe_value_in_content(&content, "work", "id_pattern")
            {
                if pattern.contains("{project}") {
                    backup_once(ctx, &path, is_personal)?;
                    let rewritten = pattern.replace("{project}", "{key}");
                    content = rewrite_one_key(
                        &content,
                        "work",
                        "id_pattern",
                        "id_pattern",
                        &rewritten,
                    );
                }
            }

            if let Some(legacy) =
                probe_value_in_content(&content, "work", "default_project_code")
            {
                backup_once(ctx, &path, is_personal)?;
                if pattern_uses_prefix {
                    content = ensure_key(&content, "work", "key", &legacy);
                }
                if let Some((prefix, key)) = scope_key {
                    if !scope_key_pinned {
                        content = ensure_key(&content, prefix, key, &legacy);
                    }
                }
                content = remove_key(&content, "work", "default_project_code");
            }

            if content != original {
                write_file(ctx, &path, &content, is_personal)?;
            }
        }

        Ok(ApplyOutcome::Applied)
    }
}

fn write_file(
    ctx: &dyn MigrationContext,
    path: &Path,
    content: &str,
    is_personal: bool,
) -> Result<(), MigrationError> {
    if is_personal {
        ctx.write_private(path, content)
    } else {
        ctx.write(path, content)
    }
}

fn backup_once(
    ctx: &dyn MigrationContext,
    path: &Path,
    is_personal: bool,
) -> Result<(), MigrationError> {
    let Some(content) = ctx.read(path)? else {
        return Ok(());
    };
    let backup = path.with_extension("md.0009.bak");
    if ctx.read(&backup)?.is_some() {
        return Ok(());
    }
    write_file(ctx, &backup, &content, is_personal)?;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let backup_name = backup.file_name().unwrap_or_default().to_string_lossy();
    eprintln!(
        "0009: backed up {file_name} → {backup_name} (remove after \
         verifying migration)"
    );
    Ok(())
}

enum Form {
    Nested,
    Flat,
    Absent,
}

fn detect_form(content: &str, prefix: &str) -> Form {
    let block_line = format!("{prefix}:");
    if content.lines().any(|line| line.trim_end() == block_line) {
        return Form::Nested;
    }
    let flat_prefix = format!("{prefix}.");
    if content.lines().any(|line| line.starts_with(&flat_prefix)) {
        return Form::Flat;
    }
    Form::Absent
}

fn clean_value(raw: &str) -> String {
    let without_comment = raw.split('#').next().unwrap_or("");
    without_comment.trim().to_owned()
}

fn join_preserving_trailing_newline(
    original: &str,
    lines: &[String],
) -> String {
    let mut result = lines.join("\n");
    if original.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn probe_value_in_content(
    content: &str,
    prefix: &str,
    key: &str,
) -> Option<String> {
    match detect_form(content, prefix) {
        Form::Nested => {
            let block_line = format!("{prefix}:");
            let key_prefix = format!("{key}:");
            let mut in_block = false;
            for line in content.lines() {
                if line.trim_end() == block_line {
                    in_block = true;
                    continue;
                }
                if in_block && !line.starts_with([' ', '\t']) {
                    in_block = false;
                }
                if in_block {
                    let indent_len = line
                        .find(|c: char| c != ' ' && c != '\t')
                        .unwrap_or(line.len());
                    let rest = &line[indent_len..];
                    if indent_len > 0 && rest.starts_with(&key_prefix) {
                        return Some(clean_value(&rest[key_prefix.len()..]));
                    }
                }
            }
            None
        }
        Form::Flat => {
            let key_prefix = format!("{prefix}.{key}:");
            content.lines().find_map(|line| {
                line.strip_prefix(&key_prefix).map(clean_value)
            })
        }
        Form::Absent => None,
    }
}

fn rewrite_one_key(
    content: &str,
    prefix: &str,
    old_key: &str,
    new_key: &str,
    new_value: &str,
) -> String {
    match detect_form(content, prefix) {
        Form::Nested => {
            let block_line = format!("{prefix}:");
            let old_prefix = format!("{old_key}:");
            let mut in_block = false;
            let mut out = Vec::new();
            for line in content.lines() {
                if line.trim_end() == block_line {
                    in_block = true;
                    out.push(line.to_owned());
                    continue;
                }
                if in_block && !line.starts_with([' ', '\t']) {
                    in_block = false;
                }
                if in_block {
                    let indent_len = line
                        .find(|c: char| c != ' ' && c != '\t')
                        .unwrap_or(line.len());
                    let (indent, rest) = line.split_at(indent_len);
                    if indent_len > 0 && rest.starts_with(&old_prefix) {
                        out.push(format!("{indent}{new_key}: {new_value}"));
                        continue;
                    }
                }
                out.push(line.to_owned());
            }
            join_preserving_trailing_newline(content, &out)
        }
        Form::Flat => {
            let old_prefix = format!("{prefix}.{old_key}:");
            let mut out = Vec::new();
            for line in content.lines() {
                if line.starts_with(&old_prefix) {
                    out.push(format!("{prefix}.{new_key}: {new_value}"));
                } else {
                    out.push(line.to_owned());
                }
            }
            join_preserving_trailing_newline(content, &out)
        }
        Form::Absent => content.to_owned(),
    }
}

fn remove_key(content: &str, prefix: &str, key: &str) -> String {
    match detect_form(content, prefix) {
        Form::Nested => {
            let block_line = format!("{prefix}:");
            let key_prefix = format!("{key}:");
            let mut in_block = false;
            let mut out = Vec::new();
            for line in content.lines() {
                if line.trim_end() == block_line {
                    in_block = true;
                    out.push(line.to_owned());
                    continue;
                }
                if in_block && !line.starts_with([' ', '\t']) {
                    in_block = false;
                }
                if in_block {
                    let indent_len = line
                        .find(|c: char| c != ' ' && c != '\t')
                        .unwrap_or(line.len());
                    let rest = &line[indent_len..];
                    if indent_len > 0 && rest.starts_with(&key_prefix) {
                        continue;
                    }
                }
                out.push(line.to_owned());
            }
            join_preserving_trailing_newline(content, &out)
        }
        Form::Flat => {
            let key_prefix = format!("{prefix}.{key}:");
            let out: Vec<String> = content
                .lines()
                .filter(|line| !line.starts_with(&key_prefix))
                .map(str::to_owned)
                .collect();
            join_preserving_trailing_newline(content, &out)
        }
        Form::Absent => content.to_owned(),
    }
}

fn ensure_key(content: &str, prefix: &str, key: &str, value: &str) -> String {
    if probe_value_in_content(content, prefix, key).is_some() {
        return content.to_owned();
    }
    match detect_form(content, prefix) {
        Form::Nested => insert_into_nested_block(content, prefix, key, value),
        Form::Flat => insert_into_flat_form(content, prefix, key, value),
        Form::Absent => create_nested_block(content, prefix, key, value),
    }
}

fn insert_into_nested_block(
    content: &str,
    prefix: &str,
    key: &str,
    value: &str,
) -> String {
    let block_line = format!("{prefix}:");
    let mut out = Vec::new();
    let mut in_block = false;
    let mut last_sibling_index = None;
    let mut sibling_indent = String::from("  ");
    for line in content.lines() {
        if line.trim_end() == block_line {
            in_block = true;
            out.push(line.to_owned());
            continue;
        }
        if in_block && !line.starts_with([' ', '\t']) {
            in_block = false;
        }
        if in_block {
            let indent_len = line
                .find(|c: char| c != ' ' && c != '\t')
                .unwrap_or(line.len());
            if indent_len > 0 {
                last_sibling_index = Some(out.len());
                line[..indent_len].clone_into(&mut sibling_indent);
            }
        }
        out.push(line.to_owned());
    }
    let entry = format!("{sibling_indent}{key}: {value}");
    if let Some(index) = last_sibling_index {
        out.insert(index + 1, entry);
    } else if let Some(index) =
        out.iter().position(|line| line.trim_end() == block_line)
    {
        out.insert(index + 1, entry);
    }
    join_preserving_trailing_newline(content, &out)
}

fn insert_into_flat_form(
    content: &str,
    prefix: &str,
    key: &str,
    value: &str,
) -> String {
    let flat_prefix = format!("{prefix}.");
    let mut out = Vec::new();
    let mut last_index = None;
    for line in content.lines() {
        if line.starts_with(&flat_prefix) {
            last_index = Some(out.len());
        }
        out.push(line.to_owned());
    }
    let entry = format!("{prefix}.{key}: {value}");
    if let Some(index) = last_index {
        out.insert(index + 1, entry);
    }
    join_preserving_trailing_newline(content, &out)
}

fn create_nested_block(
    content: &str,
    prefix: &str,
    key: &str,
    value: &str,
) -> String {
    let header = format!("{prefix}:");
    let entry = format!("  {key}: {value}");
    let lines: Vec<&str> = content.lines().collect();
    let mut fence_count = 0;
    let mut insert_at = None;
    for (index, line) in lines.iter().enumerate() {
        if line.trim_end() == "---" {
            fence_count += 1;
            if fence_count == 2 {
                insert_at = Some(index);
                break;
            }
        }
    }
    let mut out: Vec<String> =
        lines.iter().map(|line| (*line).to_owned()).collect();
    if let Some(index) = insert_at {
        out.insert(index, entry);
        out.insert(index, header);
    } else {
        out.push(header);
        out.push(entry);
    }
    join_preserving_trailing_newline(content, &out)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::literal_string_with_formatting_args
)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use super::{
        ensure_key, probe_value_in_content, remove_key, rewrite_one_key,
        Migration, Migration0009,
    };
    use crate::ports::{
        CorpusIndex, DocTypeDir, MigrationContext, MigrationError,
    };
    use crate::registry::ApplyOutcome;

    struct NoIndex;
    impl CorpusIndex for NoIndex {
        fn target_exists(&self, _target_type: &str, _target_id: &str) -> bool {
            false
        }
    }

    struct TestCtx {
        root: PathBuf,
        files: RefCell<HashMap<PathBuf, String>>,
        config: HashMap<String, String>,
        pinned: HashMap<String, String>,
        index: NoIndex,
    }

    impl TestCtx {
        fn new() -> Self {
            Self {
                root: PathBuf::from("/repo"),
                files: RefCell::new(HashMap::new()),
                config: HashMap::new(),
                pinned: HashMap::new(),
                index: NoIndex,
            }
        }

        fn with_file(self, relative: &str, content: &str) -> Self {
            self.files.borrow_mut().insert(
                PathBuf::from("/repo").join(relative),
                content.to_owned(),
            );
            self
        }

        fn with_config(mut self, key: &str, value: &str) -> Self {
            self.config.insert(key.to_owned(), value.to_owned());
            self
        }

        fn with_pinned(mut self, key: &str, value: &str) -> Self {
            self.pinned.insert(key.to_owned(), value.to_owned());
            self.config.insert(key.to_owned(), value.to_owned());
            self
        }

        fn content(&self, relative: &str) -> Option<String> {
            self.files
                .borrow()
                .get(&PathBuf::from("/repo").join(relative))
                .cloned()
        }
    }

    impl MigrationContext for TestCtx {
        fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
            Vec::new()
        }
        fn revision(&self) -> Option<String> {
            None
        }
        fn corpus_index(&self) -> &dyn CorpusIndex {
            &self.index
        }
        fn root(&self) -> &Path {
            &self.root
        }
        fn write(
            &self,
            path: &Path,
            content: &str,
        ) -> Result<(), MigrationError> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), content.to_owned());
            Ok(())
        }
        fn read(&self, path: &Path) -> Result<Option<String>, MigrationError> {
            Ok(self.files.borrow().get(path).cloned())
        }
        fn config_value(
            &self,
            key: &str,
        ) -> Result<Option<String>, MigrationError> {
            Ok(Some(self.config.get(key).cloned().unwrap_or_default()))
        }
        fn configured_path_override(
            &self,
            key: &str,
        ) -> Result<Option<String>, MigrationError> {
            Ok(self.pinned.get(key).cloned())
        }
    }

    fn apply(ctx: &TestCtx) -> Result<ApplyOutcome, MigrationError> {
        Migration0009.apply(ctx)
    }

    #[test]
    fn probe_and_rewrite_round_trip_a_quoted_value() {
        let content = "---\nwork:\n  id_pattern: \"{project}-{number:04d}\"\n\
                       ---\n";
        let value =
            probe_value_in_content(content, "work", "id_pattern").unwrap();
        assert_eq!(value, "\"{project}-{number:04d}\"");
        let rewritten = rewrite_one_key(
            content,
            "work",
            "id_pattern",
            "id_pattern",
            &value.replace("{project}", "{key}"),
        );
        assert!(rewritten.contains("id_pattern: \"{key}-{number:04d}\""));
    }

    #[test]
    fn ensure_key_inserts_when_absent_and_is_idempotent() {
        let content = "---\nwork:\n  id_pattern: \"{key}-{number:04d}\"\n---\n";
        let inserted = ensure_key(content, "work", "key", "\"PP\"");
        assert!(inserted.contains("key: \"PP\""));
        assert_eq!(ensure_key(&inserted, "work", "key", "\"PP\""), inserted);
    }

    #[test]
    fn ensure_key_creates_an_absent_block() {
        let content = "---\nwork:\n  integration: jira\n---\n";
        let inserted = ensure_key(content, "jira", "project_key", "\"PP\"");
        assert!(
            inserted.contains("jira:\n  project_key: \"PP\""),
            "{inserted}"
        );
    }

    #[test]
    fn remove_key_drops_the_nested_line() {
        let content = "---\nwork:\n  key: \"PP\"\n  \
                       default_project_code: \"PP\"\n---\n";
        let removed = remove_key(content, "work", "default_project_code");
        assert!(!removed.contains("default_project_code"));
        assert!(removed.contains("key: \"PP\""));
    }

    #[test]
    fn a_tracker_backed_project_pattern_materialises_both_keys() {
        let config = "---\nwork:\n  integration: jira\n  \
                      id_pattern: \"{project}-{number:04d}\"\n  \
                      default_project_code: \"PP\"\njira:\n  site: acme\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_config("work.integration", "jira")
            .with_config("work.id_pattern", "{project}-{number:04d}")
            .with_pinned("work.default_project_code", "PP");
        assert!(matches!(apply(&ctx).unwrap(), ApplyOutcome::Applied));
        let out = ctx.content(".accelerator/config.md").unwrap();
        assert!(out.contains("id_pattern: \"{key}-{number:04d}\""), "{out}");
        assert!(out.contains("key: \"PP\""), "{out}");
        assert!(out.contains("project_key: \"PP\""), "{out}");
        assert!(!out.contains("default_project_code"), "{out}");
    }

    #[test]
    fn a_tracker_less_project_pattern_materialises_only_work_key() {
        let config = "---\nwork:\n  \
                      id_pattern: \"{project}-{number:04d}\"\n  \
                      default_project_code: \"PP\"\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_config("work.id_pattern", "{project}-{number:04d}")
            .with_pinned("work.default_project_code", "PP");
        apply(&ctx).unwrap();
        let out = ctx.content(".accelerator/config.md").unwrap();
        assert!(out.contains("key: \"PP\""), "{out}");
        assert!(!out.contains("project_key"), "{out}");
        assert!(!out.contains("team_key"), "{out}");
        assert!(!out.contains("default_project_code"), "{out}");
    }

    #[test]
    fn a_bare_numeric_tracker_config_materialises_no_work_key() {
        let config = "---\nwork:\n  integration: jira\n  \
                      id_pattern: \"{number:04d}\"\n  \
                      default_project_code: \"PP\"\njira:\n  site: acme\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_config("work.integration", "jira")
            .with_config("work.id_pattern", "{number:04d}")
            .with_pinned("work.default_project_code", "PP");
        apply(&ctx).unwrap();
        let out = ctx.content(".accelerator/config.md").unwrap();
        assert!(out.contains("project_key: \"PP\""), "{out}");
        assert!(
            !out.lines()
                .any(|line| line.trim_start().starts_with("key:")),
            "no work.key materialised: {out}"
        );
        assert!(!out.contains("default_project_code"), "{out}");
    }

    #[test]
    fn a_divergent_pinned_pair_aborts_without_mutating() {
        let config = "---\nwork:\n  key: \"AA\"\n  \
                      default_project_code: \"BB\"\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_pinned("work.key", "AA")
            .with_pinned("work.default_project_code", "BB");
        assert!(apply(&ctx).is_err());
        assert_eq!(
            ctx.content(".accelerator/config.md").as_deref(),
            Some(config)
        );
    }

    #[test]
    fn a_legacy_beside_an_equal_work_key_is_not_mixed_state() {
        let config = "---\nwork:\n  \
                      id_pattern: \"{key}-{number:04d}\"\n  key: \"PP\"\n  \
                      default_project_code: \"PP\"\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_config("work.id_pattern", "{key}-{number:04d}")
            .with_pinned("work.key", "PP")
            .with_pinned("work.default_project_code", "PP");
        apply(&ctx).unwrap();
        let out = ctx.content(".accelerator/config.md").unwrap();
        assert!(!out.contains("default_project_code"), "{out}");
        assert!(out.contains("key: \"PP\""), "{out}");
    }

    #[test]
    fn an_already_pinned_scope_key_is_left_untouched() {
        let config = "---\nwork:\n  integration: jira\n  \
                      id_pattern: \"{number:04d}\"\n  \
                      default_project_code: \"PP\"\njira:\n  \
                      project_key: \"OPS\"\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_config("work.integration", "jira")
            .with_config("work.id_pattern", "{number:04d}")
            .with_pinned("work.default_project_code", "PP")
            .with_pinned("jira.project_key", "OPS");
        apply(&ctx).unwrap();
        let out = ctx.content(".accelerator/config.md").unwrap();
        assert!(out.contains("project_key: \"OPS\""), "{out}");
        assert!(!out.contains("project_key: \"PP\""), "{out}");
        assert!(!out.contains("default_project_code"), "{out}");
    }

    #[test]
    fn a_clean_repo_with_nothing_to_migrate_is_applied_and_unchanged() {
        let config = "---\nwork:\n  id_pattern: \"{key}-{number:04d}\"\n  \
                      key: \"PP\"\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_config("work.id_pattern", "{key}-{number:04d}")
            .with_pinned("work.key", "PP");
        assert!(matches!(apply(&ctx).unwrap(), ApplyOutcome::Applied));
        assert_eq!(
            ctx.content(".accelerator/config.md").as_deref(),
            Some(config)
        );
    }

    #[test]
    fn a_second_run_is_idempotent() {
        let config = "---\nwork:\n  integration: jira\n  \
                      id_pattern: \"{project}-{number:04d}\"\n  \
                      default_project_code: \"PP\"\njira:\n  site: acme\n---\n";
        let ctx = TestCtx::new()
            .with_file(".accelerator/config.md", config)
            .with_config("work.integration", "jira")
            .with_config("work.id_pattern", "{project}-{number:04d}")
            .with_pinned("work.default_project_code", "PP");
        apply(&ctx).unwrap();
        let after_first = ctx.content(".accelerator/config.md").unwrap();
        // On a second run the legacy key is gone, so nothing changes.
        let ctx2 = TestCtx::new()
            .with_file(".accelerator/config.md", &after_first)
            .with_config("work.integration", "jira")
            .with_config("work.id_pattern", "{key}-{number:04d}")
            .with_pinned("work.key", "PP")
            .with_pinned("jira.project_key", "PP");
        apply(&ctx2).unwrap();
        assert_eq!(
            ctx2.content(".accelerator/config.md").as_deref(),
            Some(after_first.as_str())
        );
    }
}
