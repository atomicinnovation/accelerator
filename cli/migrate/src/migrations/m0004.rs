//! Port of `0004-restructure-meta-research-into-subject-subcategories.sh`.
//!
//! Restructures `meta/research/` (plus `meta/design-inventories/`,
//! `meta/design-gaps/`) into `meta/research/{codebase,issues,
//! design-inventories,design-gaps}/`, rewrites the four config key pairs
//! this implies, renames a user-overridden `research.md` template, and
//! rewrites every inbound corpus reference to the moved paths.
//!
//! The bash original's jj-op-id rollback breadcrumb (an informational
//! stderr notice printed only when a real jj-managed working copy is
//! present) is not reproduced — it mutates no state and this port has no
//! other reason to depend on `vcs`/`jj-lib` from a mechanical migration.

use std::path::Path;
use std::path::PathBuf;

use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0004;

impl MigrationMeta for Migration0004 {
    fn id(&self) -> &'static str {
        "0004-restructure-meta-research-into-subject-subcategories"
    }

    fn description(&self) -> &'static str {
        "Restructure meta/research/ into subject subcategories"
    }
}

struct Layout {
    old_research: String,
    old_inv: String,
    old_gaps: String,
    new_research_codebase: String,
    new_research_issues: String,
    new_inv: String,
    new_gaps: String,
    research_had_override: bool,
    inv_had_override: bool,
    gaps_had_override: bool,
}

fn strip_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_owned()
}

fn resolve_layout(ctx: &dyn MigrationContext) -> Layout {
    let research = ctx.configured_path_override("paths.research");
    let inv = ctx.configured_path_override("paths.design_inventories");
    let gaps = ctx.configured_path_override("paths.design_gaps");

    let research_had_override = research.is_some();
    let inv_had_override = inv.is_some();
    let gaps_had_override = gaps.is_some();

    let old_research = strip_trailing_slash(
        &research.unwrap_or_else(|| "meta/research".to_owned()),
    );
    let old_inv = strip_trailing_slash(
        &inv.unwrap_or_else(|| "meta/design-inventories".to_owned()),
    );
    let old_gaps = strip_trailing_slash(
        &gaps.unwrap_or_else(|| "meta/design-gaps".to_owned()),
    );

    let new_research_codebase = format!("{old_research}/codebase");
    let new_research_issues = format!("{old_research}/issues");
    let new_inv = if inv_had_override {
        old_inv.clone()
    } else {
        format!("{old_research}/design-inventories")
    };
    let new_gaps = if gaps_had_override {
        old_gaps.clone()
    } else {
        format!("{old_research}/design-gaps")
    };

    Layout {
        old_research,
        old_inv,
        old_gaps,
        new_research_codebase,
        new_research_issues,
        new_inv,
        new_gaps,
        research_had_override,
        inv_had_override,
        gaps_had_override,
    }
}

const MIXED_STATE_TRIPLES: [(&str, &str, &str); 4] = [
    ("paths", "research", "research_codebase"),
    ("paths", "design_inventories", "research_design_inventories"),
    ("paths", "design_gaps", "research_design_gaps"),
    ("templates", "research", "codebase-research"),
];

fn assert_no_mixed_state(
    ctx: &dyn MigrationContext,
) -> Result<(), MigrationError> {
    for (prefix, old, new) in MIXED_STATE_TRIPLES {
        let old_present = ctx
            .configured_path_override(&format!("{prefix}.{old}"))
            .is_some();
        let new_present = ctx
            .configured_path_override(&format!("{prefix}.{new}"))
            .is_some();
        if old_present && new_present {
            return Err(MigrationError::new(format!(
                "0004: mixed-state config detected — both {prefix}.{old} \
                 and {prefix}.{new} are set. Resolve manually (remove \
                 {prefix}.{old}) and retry."
            )));
        }
    }
    Ok(())
}

impl Migration for Migration0004 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let root = ctx.root().to_path_buf();
        let layout = resolve_layout(ctx);

        assert_no_mixed_state(ctx)?;

        if !ctx.dir_exists(&root.join(&layout.old_research))
            && !ctx.dir_exists(&root.join(&layout.old_inv))
            && !ctx.dir_exists(&root.join(&layout.old_gaps))
        {
            return Ok(ApplyOutcome::Applied);
        }

        let moves = plan_moves(ctx, &root, &layout)?;
        for (source, destination) in &moves {
            let source_path = root.join(source);
            if ctx.dir_exists(&source_path) || ctx.read(&source_path)?.is_some()
            {
                ctx.merge_move(&source_path, &root.join(destination))?;
                println!("0004: moved {source} → {destination}");
            }
        }

        if !layout.inv_had_override {
            cleanup_legacy_parent(ctx, &root, &layout.old_inv)?;
        }
        if !layout.gaps_had_override {
            cleanup_legacy_parent(ctx, &root, &layout.old_gaps)?;
        }

        ensure_gitkeep(ctx, &root, &layout.new_research_codebase)?;
        ensure_gitkeep(ctx, &root, &layout.new_research_issues)?;
        if !layout.inv_had_override {
            ensure_gitkeep(ctx, &root, &layout.new_inv)?;
        }
        if !layout.gaps_had_override {
            ensure_gitkeep(ctx, &root, &layout.new_gaps)?;
        }

        rewrite_config_keys(ctx, &root)?;
        rename_user_template_file(ctx, &root)?;
        if layout.research_had_override {
            inject_research_issues(ctx, &root, &layout)?;
        }

        rewrite_inbound_references(ctx, &layout)?;

        Ok(ApplyOutcome::Applied)
    }
}

fn basename(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn plan_moves(
    ctx: &dyn MigrationContext,
    root: &Path,
    layout: &Layout,
) -> Result<Vec<(String, String)>, MigrationError> {
    let mut moves = Vec::new();

    let research_dir = root.join(&layout.old_research);
    if ctx.dir_exists(&research_dir) {
        for child in top_level_files(ctx, &research_dir)? {
            let Some(base) = basename(&child) else {
                continue;
            };
            if base == ".DS_Store" || base == ".gitkeep" {
                continue;
            }
            moves.push((
                format!("{}/{base}", layout.old_research),
                format!("{}/{base}", layout.new_research_codebase),
            ));
        }
    }

    if !layout.inv_had_override {
        let inv_dir = root.join(&layout.old_inv);
        if ctx.dir_exists(&inv_dir) {
            for child in top_level_dirs(ctx, &inv_dir)? {
                let Some(base) = basename(&child) else {
                    continue;
                };
                moves.push((
                    format!("{}/{base}", layout.old_inv),
                    format!("{}/{base}", layout.new_inv),
                ));
            }
        }
    }

    if !layout.gaps_had_override {
        let gaps_dir = root.join(&layout.old_gaps);
        if ctx.dir_exists(&gaps_dir) {
            for child in top_level_files(ctx, &gaps_dir)? {
                let Some(base) = basename(&child) else {
                    continue;
                };
                if base == ".DS_Store" || base == ".gitkeep" {
                    continue;
                }
                moves.push((
                    format!("{}/{base}", layout.old_gaps),
                    format!("{}/{base}", layout.new_gaps),
                ));
            }
        }
    }

    Ok(moves)
}

fn top_level_files(
    ctx: &dyn MigrationContext,
    dir: &Path,
) -> Result<Vec<PathBuf>, MigrationError> {
    Ok(ctx
        .list_all_under(dir)?
        .into_iter()
        .filter(|path| path.parent() == Some(dir) && !ctx.dir_exists(path))
        .collect())
}

fn top_level_dirs(
    ctx: &dyn MigrationContext,
    dir: &Path,
) -> Result<Vec<PathBuf>, MigrationError> {
    Ok(ctx
        .list_all_under(dir)?
        .into_iter()
        .filter(|path| path.parent() == Some(dir) && ctx.dir_exists(path))
        .collect())
}

fn cleanup_legacy_parent(
    ctx: &dyn MigrationContext,
    root: &Path,
    relative: &str,
) -> Result<(), MigrationError> {
    let full = root.join(relative);
    if !ctx.dir_exists(&full) {
        return Ok(());
    }
    ctx.remove_file(&full.join(".DS_Store"))?;
    ctx.remove_file(&full.join(".gitkeep"))?;

    if ctx.remove_dir_if_empty(&full)? {
        println!("0004: removed empty legacy directory {relative}");
    } else {
        eprintln!(
            "Warning: 0004: legacy directory {relative} not empty — \
             preserved as-is."
        );
        for remaining in ctx.list_all_under(&full)? {
            if let Ok(name) = remaining.strip_prefix(&full) {
                eprintln!(
                    "Warning:   contains: {} (manual cleanup may be \
                     needed)",
                    name.display()
                );
            }
        }
    }
    Ok(())
}

fn ensure_gitkeep(
    ctx: &dyn MigrationContext,
    root: &Path,
    relative: &str,
) -> Result<(), MigrationError> {
    let gitkeep = root.join(relative).join(".gitkeep");
    if ctx.read(&gitkeep)?.is_none() {
        ctx.write(&gitkeep, "")?;
        println!("0004: created {relative}/.gitkeep");
    }
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

fn clean_value(raw: &str) -> String {
    let without_comment = raw.split('#').next().unwrap_or("");
    without_comment.trim().to_owned()
}

const CONFIG_FILES: [&str; 2] = ["config.md", "config.local.md"];

fn rewrite_config_keys(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let pairs: [(&str, &str, &str, bool); 4] = [
        ("paths", "research", "research_codebase", true),
        (
            "paths",
            "design_inventories",
            "research_design_inventories",
            false,
        ),
        ("paths", "design_gaps", "research_design_gaps", false),
        ("templates", "research", "codebase-research", false),
    ];

    for (prefix, old, new, append_codebase) in pairs {
        for file in CONFIG_FILES {
            let path = root.join(".accelerator").join(file);
            let Some(content) = ctx.read(&path)? else {
                continue;
            };
            let Some(old_value) = probe_value_in_content(&content, prefix, old)
            else {
                continue;
            };
            backup_config_once(ctx, &path)?;
            let new_value = if append_codebase {
                format!("{old_value}/codebase")
            } else {
                old_value.clone()
            };
            let rewritten =
                rewrite_one_key(&content, prefix, old, new, &new_value);
            ctx.write(&path, &rewritten)?;
            if old_value == new_value {
                println!(
                    "0004: renamed {prefix}.{old} → {prefix}.{new} \
                     (value: {old_value})"
                );
            } else {
                println!(
                    "0004: renamed {prefix}.{old} → {prefix}.{new} \
                     (value: {old_value} → {new_value})"
                );
            }
        }
    }
    Ok(())
}

fn backup_config_once(
    ctx: &dyn MigrationContext,
    path: &Path,
) -> Result<(), MigrationError> {
    let Some(content) = ctx.read(path)? else {
        return Ok(());
    };
    let backup = path.with_extension("md.0004.bak");
    if ctx.read(&backup)?.is_some() {
        return Ok(());
    }
    ctx.write(&backup, &content)?;
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let backup_name = backup.file_name().unwrap_or_default().to_string_lossy();
    println!(
        "0004: backed up {file_name} → {backup_name} (remove after \
         verifying migration)"
    );
    Ok(())
}

fn rename_user_template_file(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let Some(templates_dir) = ctx.config_value("paths.templates") else {
        return Ok(());
    };
    if templates_dir.is_empty() {
        return Ok(());
    }
    let source = root.join(&templates_dir).join("research.md");
    let destination = root.join(&templates_dir).join("codebase-research.md");
    let Some(content) = ctx.read(&source)? else {
        return Ok(());
    };
    if ctx.read(&destination)?.is_some() {
        return Err(MigrationError::new(format!(
            "0004: destination {} already exists — cannot rename user \
             template.",
            destination.display()
        )));
    }
    ctx.write(&destination, &content)?;
    ctx.remove_file(&source)?;
    println!(
        "0004: renamed {templates_dir}/research.md → \
         {templates_dir}/codebase-research.md"
    );
    Ok(())
}

fn inject_research_issues(
    ctx: &dyn MigrationContext,
    root: &Path,
    layout: &Layout,
) -> Result<(), MigrationError> {
    let value = format!("{}/issues", layout.old_research);
    for file in CONFIG_FILES {
        let path = root.join(".accelerator").join(file);
        let Some(content) = ctx.read(&path)? else {
            continue;
        };
        backup_config_once(ctx, &path)?;
        let rewritten = insert_research_issues(&content, &value);
        if rewritten != content {
            ctx.write(&path, &rewritten)?;
        }
    }
    Ok(())
}

fn insert_research_issues(content: &str, value: &str) -> String {
    match detect_form(content, "paths") {
        Form::Nested => {
            if content
                .lines()
                .any(|line| line.trim_start().starts_with("research_issues:"))
            {
                return content.to_owned();
            }
            let mut out = Vec::new();
            let mut in_block = false;
            let mut last_sibling_index = None;
            let mut sibling_indent = String::new();
            for line in content.lines() {
                if line.trim_end() == "paths:" {
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
            if let Some(index) = last_sibling_index {
                out.insert(
                    index + 1,
                    format!("{sibling_indent}research_issues: {value}"),
                );
            }
            join_preserving_trailing_newline(content, &out)
        }
        Form::Flat => {
            if content
                .lines()
                .any(|line| line.starts_with("paths.research_issues:"))
            {
                return content.to_owned();
            }
            let mut out = Vec::new();
            let mut last_paths_index = None;
            for line in content.lines() {
                if line.starts_with("paths.") {
                    last_paths_index = Some(out.len());
                }
                out.push(line.to_owned());
            }
            if let Some(index) = last_paths_index {
                out.insert(
                    index + 1,
                    format!("paths.research_issues: {value}"),
                );
            }
            join_preserving_trailing_newline(content, &out)
        }
        Form::Absent => content.to_owned(),
    }
}

const SIBLING_SUBCATEGORIES: [&str; 4] =
    ["codebase", "issues", "design-inventories", "design-gaps"];

fn is_boundary(c: Option<char>) -> bool {
    c.is_none_or(|c| {
        matches!(c, '/' | '"' | '\'' | ' ' | '\t' | ')' | ']' | '#')
    })
}

fn rewrite_line_with_pair(line: &str, old: &str, new: &str) -> String {
    if old.is_empty() {
        return line.to_owned();
    }
    let mut out = String::new();
    let mut rest = line;
    while let Some(pos) = rest.find(old) {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + old.len()..];
        let boundary_char = after.chars().next();
        let mut skip = true;
        if is_boundary(boundary_char) {
            skip = false;
            if boundary_char == Some('/') {
                let segment: String = after[1..]
                    .chars()
                    .take_while(|c| !is_boundary(Some(*c)))
                    .collect();
                if SIBLING_SUBCATEGORIES.contains(&segment.as_str()) {
                    skip = true;
                }
            }
        }
        if skip {
            out.push_str(&rest[pos..=pos]);
            rest = &rest[pos + 1..];
        } else {
            out.push_str(new);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn rewrite_inbound_references(
    ctx: &dyn MigrationContext,
    layout: &Layout,
) -> Result<(), MigrationError> {
    let mut pairs = vec![(
        layout.old_research.clone(),
        layout.new_research_codebase.clone(),
    )];
    if !layout.inv_had_override {
        pairs.push((layout.old_inv.clone(), layout.new_inv.clone()));
    }
    if !layout.gaps_had_override {
        pairs.push((layout.old_gaps.clone(), layout.new_gaps.clone()));
    }

    let mut scan_dirs: Vec<PathBuf> = ctx
        .doc_type_dirs()
        .into_iter()
        .map(|entry| entry.dir)
        .filter(|dir| ctx.dir_exists(dir))
        .collect();
    scan_dirs.sort();
    scan_dirs.dedup();

    println!(
        "0004 Step 3: scanning {} configured paths for inbound \
         references…",
        scan_dirs.len()
    );

    for dir in &scan_dirs {
        for file in ctx.list_md_files(dir)? {
            let Some(content) = ctx.read(&file)? else {
                continue;
            };
            let mut rewritten = content.clone();
            for (old, new) in &pairs {
                rewritten = rewritten
                    .lines()
                    .map(|line| rewrite_line_with_pair(line, old, new))
                    .collect::<Vec<_>>()
                    .join("\n");
                if content.ends_with('\n') && !rewritten.ends_with('\n') {
                    rewritten.push('\n');
                }
            }
            if rewritten != content {
                ctx.write(&file, &rewritten)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        detect_form, insert_research_issues, probe_value_in_content,
        rewrite_line_with_pair, rewrite_one_key, Form,
    };

    #[test]
    fn rewrites_a_matching_path_at_a_slash_boundary() {
        assert_eq!(
            rewrite_line_with_pair(
                "see meta/research/a.md here",
                "meta/research",
                "meta/research/codebase"
            ),
            "see meta/research/codebase/a.md here"
        );
    }

    #[test]
    fn leaves_a_non_boundary_match_untouched() {
        assert_eq!(
            rewrite_line_with_pair(
                "meta/research-archive/a.md",
                "meta/research",
                "meta/research/codebase"
            ),
            "meta/research-archive/a.md"
        );
    }

    #[test]
    fn excludes_a_sibling_subcategory_segment_from_rematching() {
        assert_eq!(
            rewrite_line_with_pair(
                "meta/research/codebase/a.md",
                "meta/research",
                "meta/research/codebase"
            ),
            "meta/research/codebase/a.md"
        );
    }

    #[test]
    fn rewrites_a_bare_end_of_line_match() {
        assert_eq!(
            rewrite_line_with_pair(
                "cd meta/design-gaps",
                "meta/design-gaps",
                "meta/research/design-gaps"
            ),
            "cd meta/research/design-gaps"
        );
    }

    #[test]
    fn detect_form_prefers_nested_over_flat() {
        assert!(matches!(
            detect_form("paths:\n  research: x\n", "paths"),
            Form::Nested
        ));
        assert!(matches!(
            detect_form("paths.research: x\n", "paths"),
            Form::Flat
        ));
        assert!(matches!(
            detect_form("nothing here\n", "paths"),
            Form::Absent
        ));
    }

    #[test]
    fn probes_a_nested_value_stripping_comment_and_whitespace() {
        let content = "paths:\n  research: meta/research  # legacy \n";
        assert_eq!(
            probe_value_in_content(content, "paths", "research"),
            Some("meta/research".to_owned())
        );
    }

    #[test]
    fn probes_a_flat_value() {
        let content = "paths.research: custom/research\n";
        assert_eq!(
            probe_value_in_content(content, "paths", "research"),
            Some("custom/research".to_owned())
        );
    }

    #[test]
    fn rewrite_one_key_preserves_indentation_in_nested_form() {
        let content = "paths:\n  research: meta/research\n  work: meta/work\n";
        assert_eq!(
            rewrite_one_key(
                content,
                "paths",
                "research",
                "research_codebase",
                "meta/research/codebase"
            ),
            "paths:\n  research_codebase: meta/research/codebase\n  \
             work: meta/work\n"
        );
    }

    #[test]
    fn rewrite_one_key_rewrites_flat_form() {
        let content = "paths.research: meta/research\n";
        assert_eq!(
            rewrite_one_key(
                content,
                "paths",
                "research",
                "research_codebase",
                "meta/research/codebase"
            ),
            "paths.research_codebase: meta/research/codebase\n"
        );
    }

    #[test]
    fn inserts_research_issues_after_the_blocks_last_sibling_line() {
        // Bash's own scan tracks the *last* sibling seen in the whole
        // `paths:` block (not specifically the `research:` line) and
        // inserts right after it — replicated here, not "improved" into
        // inserting immediately after `research:` specifically.
        let content =
            "paths:\n  research: custom/research\n  work: meta/work\n";
        assert_eq!(
            insert_research_issues(content, "custom/research/issues"),
            "paths:\n  research: custom/research\n  work: meta/work\n  \
             research_issues: custom/research/issues\n"
        );
    }

    #[test]
    fn does_not_duplicate_an_already_present_research_issues_key() {
        let content = "paths:\n  research: x\n  research_issues: x/issues\n";
        assert_eq!(insert_research_issues(content, "x/issues"), content);
    }
}
