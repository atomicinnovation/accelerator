//! Port of `0002-rename-work-items-with-project-prefix.sh`.
//!
//! Renames legacy `NNNN-*.md` work items under `meta/work/` to the
//! configured project-prefix pattern, then rewrites every corpus-wide
//! frontmatter reference, Markdown link, and prose reference (heading
//! `#NNNN` and fenced-code-block path literals) that named the old ID.

use std::path::PathBuf;

use crate::migrations::text::map_lines;
use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0002;

impl MigrationMeta for Migration0002 {
    fn id(&self) -> &'static str {
        "0002-rename-work-items-with-project-prefix"
    }

    fn description(&self) -> &'static str {
        "Rename legacy NNNN-*.md work items to the configured project-prefix \
         pattern"
    }
}

struct Rename {
    old_path: PathBuf,
    new_path: PathBuf,
    old_id: String,
    new_id: String,
}

impl Migration for Migration0002 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let root = ctx.root().to_path_buf();
        let pattern = ctx
            .config_value("work.id_pattern")?
            .unwrap_or_else(|| "{number:04d}".to_owned());
        let default_project = ctx
            .config_value("work.default_project_code")?
            .unwrap_or_default();

        if !pattern.contains("{project}") {
            return Ok(ApplyOutcome::NoOpPending);
        }
        if default_project.is_empty() {
            return Err(missing_default_project_error(&pattern));
        }

        let renames = collect_renames(ctx, &root)?;
        if renames.is_empty() {
            return Ok(ApplyOutcome::Applied);
        }

        check_collisions(ctx, &renames)?;
        apply_renames(ctx, &renames)?;
        rewrite_corpus_references(ctx, &root, &renames)?;

        Ok(ApplyOutcome::Applied)
    }
}

fn missing_default_project_error(pattern: &str) -> MigrationError {
    MigrationError::new(format!(
        "error: migration 0002 requires a value for \
         work.default_project_code (your pattern '{pattern}' contains \
         {{project}}). Set work.default_project_code in your config to \
         apply, or run 'accelerator migrate --skip \
         0002-rename-work-items-with-project-prefix' to opt out. See \
         skills/config/configure/SKILL.md > Work Items for details on \
         choosing."
    ))
}

fn collect_renames(
    ctx: &dyn MigrationContext,
    root: &std::path::Path,
) -> Result<Vec<Rename>, MigrationError> {
    let work_dir = root.join("meta/work");
    let mut renames = Vec::new();
    for path in ctx.list_md_files(&work_dir)? {
        if path.parent() != Some(work_dir.as_path()) {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some((old_num, slug)) = split_legacy_id(stem) else {
            continue;
        };
        let old_id = old_num.to_owned();
        let new_id = ctx.canonicalise_work_item_id(old_num)?;
        let new_path = work_dir.join(format!("{new_id}-{slug}.md"));
        renames.push(Rename {
            old_path: path,
            new_path,
            old_id,
            new_id,
        });
    }
    Ok(renames)
}

fn check_collisions(
    ctx: &dyn MigrationContext,
    renames: &[Rename],
) -> Result<(), MigrationError> {
    let mut collisions = Vec::new();
    for rename in renames {
        if rename.new_path != rename.old_path
            && ctx.read(&rename.new_path)?.is_some()
        {
            collisions.push(format!(
                "{} → {}",
                rename.old_path.display(),
                rename.new_path.display()
            ));
        }
    }
    if collisions.is_empty() {
        return Ok(());
    }
    let listed = collisions
        .iter()
        .map(|entry| format!("  {entry}"))
        .collect::<Vec<_>>()
        .join("\n");
    Err(MigrationError::new(format!(
        "error: rename collision detected — target files already \
         exist:\n{listed}"
    )))
}

fn apply_renames(
    ctx: &dyn MigrationContext,
    renames: &[Rename],
) -> Result<(), MigrationError> {
    for rename in renames {
        if let Some(content) = ctx.read(&rename.old_path)? {
            let rewritten = rewrite_own_work_item_id(&content, &rename.new_id);
            ctx.write(&rename.old_path, &rewritten)?;
        }
        if rename.old_path != rename.new_path {
            ctx.merge_move(&rename.old_path, &rename.new_path)?;
        }
    }
    Ok(())
}

fn rewrite_corpus_references(
    ctx: &dyn MigrationContext,
    root: &std::path::Path,
    renames: &[Rename],
) -> Result<(), MigrationError> {
    let all_files = ctx.list_md_files(&root.join("meta"))?;

    rewrite_each(ctx, &all_files, renames, |content, old_id, new_id| {
        rewrite_frontmatter_refs(content, old_id, new_id)
    })?;
    rewrite_each(ctx, &all_files, renames, |content, old_id, new_id| {
        map_lines(content, |line| {
            rewrite_markdown_link_line(line, old_id, new_id)
        })
    })?;
    rewrite_each(ctx, &all_files, renames, |content, old_id, new_id| {
        rewrite_heading_refs(content, old_id, new_id)
    })?;
    rewrite_each(ctx, &all_files, renames, |content, old_id, new_id| {
        rewrite_fenced_code_paths(content, old_id, new_id)
    })?;
    Ok(())
}

fn rewrite_each(
    ctx: &dyn MigrationContext,
    files: &[PathBuf],
    renames: &[Rename],
    pass: impl Fn(&str, &str, &str) -> String,
) -> Result<(), MigrationError> {
    for file in files {
        if let Some(content) = ctx.read(file)? {
            let mut rewritten = content.clone();
            for rename in renames {
                rewritten = pass(&rewritten, &rename.old_id, &rename.new_id);
            }
            if rewritten != content {
                ctx.write(file, &rewritten)?;
            }
        }
    }
    Ok(())
}

/// `^([0-9]{4})-(.+)$` — a legacy bare-numeric work-item filename stem.
fn split_legacy_id(stem: &str) -> Option<(&str, &str)> {
    if stem.len() < 6 {
        return None;
    }
    let (num, rest) = stem.split_at(4);
    if !num.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let slug = rest.strip_prefix('-')?;
    if slug.is_empty() {
        return None;
    }
    Some((num, slug))
}

fn is_fence_line(line: &str) -> bool {
    line.starts_with("---") && line[3..].chars().all(|c| c == ' ' || c == '\t')
}

/// Rewrites the first `work_item_id:` line inside the file's own
/// frontmatter to `new_id`, quoted — the renamed file's own identity field.
fn rewrite_own_work_item_id(content: &str, new_id: &str) -> String {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return content.to_owned();
    };
    if !is_fence_line(first) {
        return content.to_owned();
    }
    let mut out = vec![first.to_owned()];
    let mut in_fm = true;
    for line in lines {
        if in_fm && is_fence_line(line) {
            in_fm = false;
            out.push(line.to_owned());
        } else if in_fm && line.starts_with("work_item_id:") {
            out.push(format!("work_item_id: \"{new_id}\""));
        } else {
            out.push(line.to_owned());
        }
    }
    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

const REFERENCE_FIELDS: [&str; 7] = [
    "work_item_id",
    "parent",
    "related",
    "blocks",
    "blocked_by",
    "supersedes",
    "superseded_by",
];

fn leading_whitespace_len(line: &str) -> usize {
    line.find(|c: char| c != ' ' && c != '\t')
        .unwrap_or(line.len())
}

fn matched_reference_field(line: &str) -> bool {
    let indent_len = leading_whitespace_len(line);
    let rest = &line[indent_len..];
    REFERENCE_FIELDS.iter().any(|field| {
        rest.strip_prefix(field)
            .is_some_and(|after| after.starts_with(':'))
    })
}

fn split_scalar_prefix_and_value(line: &str) -> (String, &str) {
    let indent_len = leading_whitespace_len(line);
    let colon_offset = line[indent_len..]
        .find(':')
        .unwrap_or(line.len() - indent_len);
    let key_end = indent_len + colon_offset + 1;
    let rest = &line[key_end..];
    let ws_len = rest
        .find(|c: char| c != ' ' && c != '\t')
        .unwrap_or(rest.len());
    (line[..key_end + ws_len].to_owned(), &rest[ws_len..])
}

fn is_dash_item_line(line: &str) -> bool {
    let indent_len = leading_whitespace_len(line);
    let rest = &line[indent_len..];
    rest.starts_with('-') && rest[1..].starts_with([' ', '\t'])
}

fn split_dash_prefix_and_value(line: &str) -> (String, &str) {
    let indent_len = leading_whitespace_len(line);
    let dash_end = indent_len + 1;
    let rest = &line[dash_end..];
    let ws_len = rest
        .find(|c: char| c != ' ' && c != '\t')
        .unwrap_or(rest.len());
    (line[..dash_end + ws_len].to_owned(), &rest[ws_len..])
}

fn is_reference_value(value: &str, old_id: &str) -> bool {
    let trimmed = value.trim_end();
    trimmed == format!("\"{old_id}\"")
        || trimmed == format!("'{old_id}'")
        || trimmed == old_id
}

/// Wraps a bare (unquoted) `old_id` occurrence immediately preceded by
/// `before_char` (`[` or `,`) and followed by optional whitespace then `]`
/// or `,`, in double quotes with `new_id` — mirrors the two bracket-aware
/// sed passes over an inline-list line.
fn wrap_bare_id_after(
    line: &str,
    before_char: char,
    old_id: &str,
    new_id: &str,
) -> String {
    let mut result = String::new();
    let mut rest = line;
    while let Some(pos) = rest.find(before_char) {
        let (head, tail) = rest.split_at(pos + before_char.len_utf8());
        let ws1_len = tail
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(tail.len());
        let after_ws1 = &tail[ws1_len..];
        if let Some(after_id) = after_ws1.strip_prefix(old_id) {
            let ws2_len = after_id
                .find(|c: char| !c.is_whitespace())
                .unwrap_or(after_id.len());
            let after_ws2 = &after_id[ws2_len..];
            if after_ws2.starts_with(']') || after_ws2.starts_with(',') {
                result.push_str(head);
                result.push_str(&tail[..ws1_len]);
                result.push('"');
                result.push_str(new_id);
                result.push('"');
                result.push_str(&after_id[..ws2_len]);
                rest = after_ws2;
                continue;
            }
        }
        result.push_str(head);
        rest = tail;
    }
    result.push_str(rest);
    result
}

fn rewrite_inline_list_line(line: &str, old_id: &str, new_id: &str) -> String {
    let step1 =
        line.replace(&format!("\"{old_id}\""), &format!("\"{new_id}\""));
    let step2 = step1.replace(&format!("'{old_id}'"), &format!("\"{new_id}\""));
    let step3 = wrap_bare_id_after(&step2, '[', old_id, new_id);
    wrap_bare_id_after(&step3, ',', old_id, new_id)
}

/// One `old_id` → `new_id` pass over every reference-field frontmatter line
/// in `content` — inline lists, single-line scalars, and (matching bash's
/// own structure, which never actually reaches this branch for any of the
/// seven known field names) multi-line dash-item continuations.
fn rewrite_frontmatter_refs(
    content: &str,
    old_id: &str,
    new_id: &str,
) -> String {
    let mut out = Vec::new();
    let mut in_fm = false;
    let mut in_list = false;
    for line in content.lines() {
        if !in_fm && out.is_empty() && is_fence_line(line) {
            in_fm = true;
            out.push(line.to_owned());
            continue;
        }
        if in_fm && is_fence_line(line) {
            in_fm = false;
            in_list = false;
            out.push(line.to_owned());
            continue;
        }
        if !in_fm {
            out.push(line.to_owned());
            continue;
        }
        if matched_reference_field(line) {
            in_list = false;
            if line.contains('[') {
                out.push(rewrite_inline_list_line(line, old_id, new_id));
                if !line.contains(']') {
                    in_list = true;
                }
            } else {
                let (prefix, value) = split_scalar_prefix_and_value(line);
                if is_reference_value(value, old_id) {
                    out.push(format!("{prefix}\"{new_id}\""));
                } else {
                    out.push(line.to_owned());
                }
            }
        } else if in_list && is_dash_item_line(line) {
            let (prefix, value) = split_dash_prefix_and_value(line);
            if is_reference_value(value, old_id) {
                out.push(format!("{prefix}\"{new_id}\""));
            } else {
                out.push(line.to_owned());
            }
        } else {
            in_list = false;
            out.push(line.to_owned());
        }
    }
    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// `s|(\[[^]]*\]\([^)]*/)OLDID-([^)]+\.md)(#[^)]*)?\)|\1NEWID-\2\3)|g` — a
/// Markdown link whose target path's basename starts with `OLDID-`.
fn rewrite_markdown_link_line(
    line: &str,
    old_id: &str,
    new_id: &str,
) -> String {
    let mut result = String::new();
    let mut rest = line;
    let mut consumed = 0usize;
    while let Some(open) = rest.find('[') {
        if let Some((matched_len, replacement)) =
            try_match_link(rest, open, old_id, new_id)
        {
            result.push_str(&rest[..open]);
            result.push_str(&replacement);
            rest = &rest[open + matched_len..];
            consumed += 1;
            let _ = consumed;
            continue;
        }
        result.push_str(&rest[..=open]);
        rest = &rest[open + 1..];
    }
    result.push_str(rest);
    result
}

fn try_match_link(
    line: &str,
    start: usize,
    old_id: &str,
    new_id: &str,
) -> Option<(usize, String)> {
    let after_open = &line[start + 1..];
    let close_bracket_rel = after_open.find(']')?;
    if after_open[..close_bracket_rel].contains('[') {
        return None;
    }
    let link_text_end = start + 1 + close_bracket_rel;
    if line.as_bytes()[link_text_end + 1..].first() != Some(&b'(') {
        return None;
    }
    let paren_start = link_text_end + 2;
    let after_paren = &line[paren_start..];
    let close_paren_rel = after_paren.find(')')?;
    let scoped = &after_paren[..close_paren_rel];

    let needle = format!("/{old_id}-");
    let needle_pos = scoped.find(&needle)?;
    let path_prefix = &scoped[..=needle_pos];
    let after_needle = &scoped[needle_pos + needle.len()..];

    let md_pos = after_needle.find(".md")?;
    if md_pos == 0 {
        return None;
    }
    let slug = &after_needle[..md_pos];
    let anchor = &after_needle[md_pos + 3..];
    if !anchor.is_empty() && !anchor.starts_with('#') {
        return None;
    }

    let link_text = &line[start..=link_text_end];
    let replacement =
        format!("{link_text}({path_prefix}{new_id}-{slug}.md{anchor})");
    let total_len = paren_start + close_paren_rel + 1 - start;
    Some((total_len, replacement))
}

fn is_heading_line(line: &str) -> bool {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    hashes > 0
        && line
            .as_bytes()
            .get(hashes)
            .is_some_and(|byte| *byte == b' ' || *byte == b'\t')
}

const fn is_ref_boundary_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn rewrite_heading_line(line: &str, old_id: &str, new_id: &str) -> String {
    let needle = format!("#{old_id}");
    let mut result = String::new();
    let mut rest = line;
    while let Some(pos) = rest.find(&needle) {
        let before = &rest[..pos];
        let after = &rest[pos + needle.len()..];
        let pre_ok = before
            .chars()
            .next_back()
            .is_none_or(|c| !is_ref_boundary_char(c));
        let post_ok = after
            .chars()
            .next()
            .is_none_or(|c| !(is_ref_boundary_char(c) || c == '-'));
        result.push_str(before);
        result.push('#');
        result.push_str(if pre_ok && post_ok { new_id } else { old_id });
        rest = after;
    }
    result.push_str(rest);
    result
}

fn rewrite_heading_refs(content: &str, old_id: &str, new_id: &str) -> String {
    map_lines(content, |line| {
        if is_heading_line(line) {
            rewrite_heading_line(line, old_id, new_id)
        } else {
            line.to_owned()
        }
    })
}

const FENCED_TAGS: [&str; 5] = ["bash", "sh", "yaml", "json", "text"];

fn is_tagged_fence_open(line: &str) -> bool {
    line.strip_prefix("```")
        .is_some_and(|rest| FENCED_TAGS.iter().any(|tag| rest.starts_with(tag)))
}

fn rewrite_fenced_code_paths(
    content: &str,
    old_id: &str,
    new_id: &str,
) -> String {
    let old_needle = format!("meta/work/{old_id}-");
    let new_replacement = format!("meta/work/{new_id}-");
    let mut out = Vec::new();
    let mut in_tagged = false;
    for line in content.lines() {
        if is_tagged_fence_open(line) {
            in_tagged = true;
            out.push(line.to_owned());
        } else if line.starts_with("```") {
            in_tagged = false;
            out.push(line.to_owned());
        } else if in_tagged {
            out.push(line.replace(&old_needle, &new_replacement));
        } else {
            out.push(line.to_owned());
        }
    }
    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        rewrite_fenced_code_paths, rewrite_frontmatter_refs,
        rewrite_heading_refs, rewrite_markdown_link_line,
        rewrite_own_work_item_id, split_legacy_id,
    };

    #[test]
    fn splits_legacy_ids() {
        assert_eq!(split_legacy_id("0001-foo"), Some(("0001", "foo")));
        assert_eq!(split_legacy_id("0001-"), None);
        assert_eq!(split_legacy_id("PROJ-0001-foo"), None);
        assert_eq!(split_legacy_id("001-foo"), None);
    }

    #[test]
    fn rewrites_the_renamed_files_own_id() {
        let content = "---\nwork_item_id: \"0001\"\n---\n\n# Foo\n";
        assert_eq!(
            rewrite_own_work_item_id(content, "PROJ-0001"),
            "---\nwork_item_id: \"PROJ-0001\"\n---\n\n# Foo\n"
        );
    }

    #[test]
    fn rewrites_scalar_frontmatter_references_quoted_and_bare() {
        let content = "---\nparent: \"0001\"\nrelated: 0001\n---\n";
        assert_eq!(
            rewrite_frontmatter_refs(content, "0001", "PROJ-0001"),
            "---\nparent: \"PROJ-0001\"\nrelated: \"PROJ-0001\"\n---\n"
        );
    }

    #[test]
    fn rewrites_inline_list_references() {
        let content = "---\nblocks: [0001, \"0002\"]\n---\n";
        let rewritten = rewrite_frontmatter_refs(content, "0001", "PROJ-0001");
        assert_eq!(rewritten, "---\nblocks: [\"PROJ-0001\", \"0002\"]\n---\n");
    }

    #[test]
    fn leaves_unrelated_frontmatter_untouched() {
        let content = "---\ntitle: \"0001 is not a reference field\"\n---\n";
        assert_eq!(
            rewrite_frontmatter_refs(content, "0001", "PROJ-0001"),
            content
        );
    }

    #[test]
    fn rewrites_a_markdown_link_target() {
        let line = "See [the item](../work/0001-foo.md#section).";
        assert_eq!(
            rewrite_markdown_link_line(line, "0001", "PROJ-0001"),
            "See [the item](../work/PROJ-0001-foo.md#section)."
        );
    }

    #[test]
    fn leaves_a_link_to_a_different_id_untouched() {
        let line = "See [the item](../work/0002-foo.md).";
        assert_eq!(rewrite_markdown_link_line(line, "0001", "PROJ-0001"), line);
    }

    #[test]
    fn rewrites_bounded_heading_references_only() {
        let content = "# Depends on #0001 and X#0001 and #00012\n";
        assert_eq!(
            rewrite_heading_refs(content, "0001", "PROJ-0001"),
            "# Depends on #PROJ-0001 and X#0001 and #00012\n"
        );
    }

    #[test]
    fn does_not_rewrite_non_heading_lines() {
        let content = "Body text mentioning #0001 stays put.\n";
        assert_eq!(rewrite_heading_refs(content, "0001", "PROJ-0001"), content);
    }

    #[test]
    fn rewrites_tagged_fenced_code_block_paths_only() {
        let content =
            "```bash\naccelerator work show meta/work/0001-foo.md\n```\n\
             ```\nmeta/work/0001-foo.md\n```\n";
        let rewritten = rewrite_fenced_code_paths(content, "0001", "PROJ-0001");
        assert!(rewritten.contains("meta/work/PROJ-0001-foo.md"));
        assert!(rewritten.contains("\nmeta/work/0001-foo.md\n```\n"));
    }
}
