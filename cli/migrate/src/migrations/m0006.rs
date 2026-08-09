//! Canonicalises the work-item-id and author/researcher fields.
//!
//! A single-pass, six-pattern frontmatter/body-label rewrite
//! (`work-item:`/`work_item_id:`/`researcher:`/`author:` in frontmatter,
//! `**Researcher**:`/`**Author**:` in the pre-first-H2 body region) across
//! the `plans`/`research_codebase`/`research_issues` corpora plus any
//! userspace-overridden templates, with REFUSE/DIVERGE/MALFORMED
//! diagnostics preserved as stable substrings.
//!
//! A symlink-escape check — resolving a configured path's parent through
//! the real filesystem and confirming it stays under the project root — is
//! not reproduced; only the surface-form dangerous-pattern check is. A
//! configured path that is itself safe-looking but escapes the project
//! root only via a symlinked parent is the sole case this narrows;
//! ordinary traversal (`../`, absolute paths) is still refused.
//! Reproducing the symlink check would need a new canonicalising port
//! primitive for a corner case with no fixture coverage.
//!
//! Likewise, the corpus-alias and template-alias dedup loops compare
//! `root.join(rel)` path strings rather than `cd && pwd -P` real paths —
//! catching the common case (two keys configured to the literal same
//! relative path) but not a symlink-mediated alias.

use std::path::Path;
use std::path::PathBuf;

use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0006;

impl MigrationMeta for Migration0006 {
    fn id(&self) -> &'static str {
        "0006-canonicalise-work-item-id-and-author"
    }

    fn description(&self) -> &'static str {
        "Canonicalise plan work-item -> work_item_id and research/RCA \
         researcher -> author. Unconditional within frontmatter; \
         body-label rewrite is anchored to the post-frontmatter / \
         pre-first-H2 region."
    }
}

impl Migration for Migration0006 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let root = ctx.root().to_path_buf();
        walk_configured_corpora(ctx, &root)?;
        rewrite_userspace_templates(ctx, &root)?;
        Ok(ApplyOutcome::Applied)
    }
}

fn is_dangerous_path(path: &str) -> bool {
    path.is_empty()
        || path == "."
        || path == ".."
        || path == "/"
        || path.starts_with('/')
        || path.ends_with("/..")
        || path.starts_with("../")
        || path.contains("/../")
        || path.contains("/./")
}

const CORPUS_KEYS: [&str; 3] =
    ["plans", "research_codebase", "research_issues"];

fn walk_configured_corpora(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let mut walked: Vec<(PathBuf, &str)> = Vec::new();
    for key in CORPUS_KEYS {
        let raw_rel = ctx
            .config_value(&format!("paths.{key}"))?
            .unwrap_or_default();
        if raw_rel.is_empty() {
            continue;
        }
        let canon = root.join(&raw_rel);
        if let Some((_, owner)) = walked.iter().find(|(c, _)| *c == canon) {
            eprintln!(
                "Warning: 0006: paths.{key} aliases paths.{owner} \
                 ({raw_rel} -> {}) — skipping duplicate walk",
                canon.display()
            );
            eprintln!("0006: skipping duplicate walk for paths.{key}");
            continue;
        }
        walked.push((canon, key));
        walk_corpus(ctx, root, key)?;
    }
    Ok(())
}

fn walk_corpus(
    ctx: &dyn MigrationContext,
    root: &Path,
    key: &str,
) -> Result<(), MigrationError> {
    let Some(rel) = resolve_corpus_path(ctx, key)? else {
        eprintln!("0006: rewrote 0 file(s) under <unresolved {key}>");
        return Ok(());
    };
    let abs = root.join(&rel);
    if !ctx.dir_exists(&abs) {
        eprintln!("Warning: 0006: {key} directory does not exist: {rel}");
        eprintln!("0006: rewrote 0 file(s) under {rel}");
        return Ok(());
    }
    let mut rewrote = 0usize;
    for file in ctx.list_md_files(&abs)? {
        if rewrite_file(ctx, &file)? {
            rewrote += 1;
        }
    }
    eprintln!("0006: rewrote {rewrote} file(s) under {rel}");
    Ok(())
}

fn resolve_corpus_path(
    ctx: &dyn MigrationContext,
    key: &str,
) -> Result<Option<String>, MigrationError> {
    let rel = ctx
        .config_value(&format!("paths.{key}"))?
        .unwrap_or_default();
    if rel.is_empty() {
        eprintln!(
            "Warning: 0006: config path returned empty for '{key}' — \
             skipping corpus"
        );
        return Ok(None);
    }
    if is_dangerous_path(&rel) {
        eprintln!("Warning: 0006: refusing dangerous paths.{key} value: {rel}");
        eprintln!(
            "Warning: 0006: skipping unsafe paths.{key} — other corpora \
             will still migrate"
        );
        return Ok(None);
    }
    Ok(Some(rel))
}

const TEMPLATE_NAMES: [&str; 3] = ["plan", "codebase-research", "rca"];

fn rewrite_userspace_templates(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let mut resolved: Vec<(PathBuf, &str)> = Vec::new();
    for name in TEMPLATE_NAMES {
        let Some(path) = resolve_user_template_path(ctx, root, name)? else {
            continue;
        };
        if let Some((_, owner)) = resolved.iter().find(|(p, _)| *p == path) {
            eprintln!(
                "Warning: 0006: templates.{name} and templates.{owner} \
                 resolve to the same file ({}) — skipping duplicate \
                 rewrite",
                path.display()
            );
            continue;
        }
        resolved.push((path.clone(), name));
        let touched = rewrite_file(ctx, &path)?;
        eprintln!(
            "0006: template {name} (tier-resolved {}): touched={}",
            path.display(),
            u8::from(touched)
        );
    }
    Ok(())
}

fn resolve_user_template_path(
    ctx: &dyn MigrationContext,
    root: &Path,
    name: &str,
) -> Result<Option<PathBuf>, MigrationError> {
    let tier1 = ctx
        .config_value(&format!("templates.{name}"))?
        .unwrap_or_default();
    if !tier1.is_empty() {
        if is_dangerous_path(&tier1) {
            eprintln!(
                "Warning: 0006: refusing dangerous templates.{name} \
                 value: {tier1}"
            );
            return Ok(None);
        }
        let tier1_abs = root.join(&tier1);
        if ctx.read(&tier1_abs)?.is_some() {
            return Ok(Some(tier1_abs));
        }
        eprintln!(
            "Warning: 0006: templates.{name} points at missing file: \
             {tier1} (skipping; not falling through to tier-2)"
        );
        return Ok(None);
    }

    let tdir_rel = ctx.config_value("paths.templates")?.unwrap_or_default();
    if !tdir_rel.is_empty() {
        let tier2_abs = root.join(&tdir_rel).join(format!("{name}.md"));
        if ctx.read(&tier2_abs)?.is_some() {
            return Ok(Some(tier2_abs));
        }
    }
    Ok(None)
}

fn has_any_legacy_key(content: &str) -> bool {
    content.lines().any(|line| {
        line.starts_with("work-item:")
            || line.starts_with("work_item_id:")
            || line.starts_with("researcher:")
            || line.starts_with("author:")
            || line.starts_with("**Researcher**:")
            || line.starts_with("**Author**:")
    })
}

fn extract_frontmatter(content: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut fence_count = 0u8;
    for line in content.lines() {
        if line == "---" {
            fence_count += 1;
            if fence_count == 1 {
                continue;
            }
            if fence_count == 2 {
                break;
            }
        }
        if fence_count == 1 {
            result.push(line);
        }
    }
    result
}

fn extract_pre_h2(content: &str) -> Vec<&str> {
    let mut result = Vec::new();
    for line in content.lines() {
        if line.starts_with("## ") {
            break;
        }
        result.push(line);
    }
    result
}

fn rewrite_file(
    ctx: &dyn MigrationContext,
    file: &Path,
) -> Result<bool, MigrationError> {
    let Some(content) = ctx.read(file)? else {
        return Ok(false);
    };
    if !has_any_legacy_key(&content) {
        return Ok(false);
    }

    let frontmatter = extract_frontmatter(&content);
    let pre_h2 = extract_pre_h2(&content);
    let flags = Flags {
        has_id: frontmatter.iter().any(|l| l.starts_with("work_item_id:")),
        has_a: frontmatter.iter().any(|l| l.starts_with("author:")),
        has_ab: pre_h2.iter().any(|l| l.starts_with("**Author**:")),
    };

    let file_display = file.display().to_string();
    let (rewritten, diagnostics) = transform(&content, &file_display, &flags);
    for diagnostic in diagnostics {
        eprintln!("Warning: 0006: {diagnostic}");
    }

    if rewritten != content {
        ctx.write(file, &rewritten)?;
        return Ok(true);
    }
    Ok(false)
}

struct Flags {
    has_id: bool,
    has_a: bool,
    has_ab: bool,
}

fn is_double_quoted(value: &str) -> bool {
    value.len() >= 2 && value.starts_with('"') && value.ends_with('"')
}

fn is_single_quoted(value: &str) -> bool {
    value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'')
}

fn normalise_value(value: &str) -> String {
    if is_double_quoted(value) {
        return value.to_owned();
    }
    if is_single_quoted(value) {
        let inner = &value[1..value.len() - 1];
        let escaped = inner.replace('\\', "\\\\").replace('"', "\\\"");
        return format!("\"{escaped}\"");
    }
    format!("\"{value}\"")
}

fn semantic_inner(value: &str) -> &str {
    if is_double_quoted(value) || is_single_quoted(value) {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn refuses(value: &str) -> bool {
    if is_double_quoted(value) || is_single_quoted(value) {
        return false;
    }
    value.contains('#') || value.contains('"')
}

fn extract_value<'a>(line: &'a str, prefix: &str) -> &'a str {
    line[prefix.len()..].trim_matches([' ', '\t'])
}

/// `_rb`/`_ab`/`_r`/`_wi` etc. deliberately mirror the original bash state
/// machine's variable names one-for-one rather than being renamed for
/// `similar_names`, and the function is not split apart for cognitive
/// complexity: this is a direct line-by-line port of a single bash state
/// machine, and splitting it would fragment one coherent transformation
/// across several functions.
#[allow(clippy::too_many_lines)]
#[allow(clippy::similar_names)]
#[allow(clippy::cognitive_complexity)]
fn transform(
    content: &str,
    file_display: &str,
    flags: &Flags,
) -> (String, Vec<String>) {
    let mut out: Vec<String> = Vec::new();
    let mut diagnostics = Vec::new();

    let mut in_frontmatter = false;
    let mut seen_frontmatter_open = false;
    let mut saw_first_h2 = false;
    let mut saw_work_item = false;
    let mut saw_work_item_id = false;
    let mut saw_researcher = false;
    let mut saw_author = false;
    let mut saw_body_researcher = false;
    let mut saw_body_author = false;
    let mut first_wi = String::new();
    let mut first_id = String::new();
    let mut first_r = String::new();
    let mut first_a = String::new();
    let mut first_rb = String::new();
    let mut first_ab = String::new();
    let mut inner_wi = String::new();
    let mut inner_id = String::new();
    let mut saw_wi_anywhere = false;
    let mut saw_r_anywhere = false;
    let mut saw_rb_anywhere = false;
    let mut dropped_first_rb = false;

    for line in content.lines() {
        if line.starts_with("work-item:") {
            saw_wi_anywhere = true;
        }
        if line.starts_with("researcher:") {
            saw_r_anywhere = true;
        }
        if line.starts_with("**Researcher**:") {
            saw_rb_anywhere = true;
        }

        if !seen_frontmatter_open && line == "---" {
            seen_frontmatter_open = true;
            in_frontmatter = true;
            out.push(line.to_owned());
            continue;
        }
        if in_frontmatter && line == "---" {
            in_frontmatter = false;
            out.push(line.to_owned());
            continue;
        }
        if line.starts_with("## ") {
            saw_first_h2 = true;
        }

        if in_frontmatter && line.starts_with("work-item:") {
            let value = extract_value(line, "work-item:");
            if refuses(value) {
                out.push(line.to_owned());
                diagnostics.push(format!(
                    "0006-REFUSE: {file_display} — refused work-item \
                     (unsafe value shape)"
                ));
                continue;
            }
            if !saw_work_item {
                value.clone_into(&mut first_wi);
                semantic_inner(value).clone_into(&mut inner_wi);
                saw_work_item = true;
            }
            if flags.has_id {
                continue;
            }
            if value.is_empty() {
                out.push("work_item_id:".to_owned());
            } else {
                out.push(format!("work_item_id: {}", normalise_value(value)));
            }
            continue;
        }

        if in_frontmatter && line.starts_with("work_item_id:") {
            let value = extract_value(line, "work_item_id:");
            if refuses(value) {
                out.push(line.to_owned());
                diagnostics.push(format!(
                    "0006-REFUSE: {file_display} — refused work_item_id \
                     (unsafe value shape)"
                ));
                continue;
            }
            if saw_work_item_id {
                if semantic_inner(value) != inner_id {
                    diagnostics.push(format!(
                        "0006-DIVERGE: {file_display} — multiple \
                         work_item_id values: kept first \"{inner_id}\", \
                         dropped \"{}\"",
                        semantic_inner(value)
                    ));
                }
                continue;
            }
            value.clone_into(&mut first_id);
            semantic_inner(value).clone_into(&mut inner_id);
            saw_work_item_id = true;
            if value.is_empty() {
                out.push("work_item_id:".to_owned());
            } else {
                out.push(format!("work_item_id: {}", normalise_value(value)));
            }
            continue;
        }

        if in_frontmatter && line.starts_with("researcher:") {
            let value = extract_value(line, "researcher:");
            if !saw_researcher {
                value.clone_into(&mut first_r);
                saw_researcher = true;
            }
            if flags.has_a {
                continue;
            }
            out.push(format!("author:{}", &line["researcher:".len()..]));
            continue;
        }

        if in_frontmatter && line.starts_with("author:") {
            let value = extract_value(line, "author:");
            if !saw_author {
                value.clone_into(&mut first_a);
                saw_author = true;
            }
            out.push(line.to_owned());
            continue;
        }

        if !saw_first_h2 && line.starts_with("**Researcher**:") {
            let value = extract_value(line, "**Researcher**:");
            if !saw_body_researcher {
                value.clone_into(&mut first_rb);
                saw_body_researcher = true;
            }
            if flags.has_ab && !dropped_first_rb {
                dropped_first_rb = true;
                continue;
            }
            out.push(format!(
                "**Author**:{}",
                &line["**Researcher**:".len()..]
            ));
            continue;
        }

        if !saw_first_h2 && line.starts_with("**Author**:") {
            let value = extract_value(line, "**Author**:");
            if !saw_body_author {
                value.clone_into(&mut first_ab);
                saw_body_author = true;
            }
            out.push(line.to_owned());
            continue;
        }

        out.push(line.to_owned());
    }

    if saw_work_item && saw_work_item_id && inner_wi != inner_id {
        diagnostics.push(format!(
            "0006-DIVERGE: {file_display} — work-item={first_wi} vs \
             work_item_id={first_id} (kept work_item_id)"
        ));
    }
    if saw_researcher && saw_author && first_r != first_a {
        diagnostics.push(format!(
            "0006-DIVERGE: {file_display} — researcher={first_r} vs \
             author={first_a} (kept author)"
        ));
    }
    if saw_body_researcher && saw_body_author && first_rb != first_ab {
        diagnostics.push(format!(
            "0006-DIVERGE: {file_display} — **Researcher**={first_rb} vs \
             **Author**={first_ab} (kept **Author**)"
        ));
    }
    if !seen_frontmatter_open
        && (saw_wi_anywhere || saw_r_anywhere || saw_rb_anywhere)
    {
        diagnostics.push(format!(
            "0006-MALFORMED: {file_display} — legacy key seen but no \
             frontmatter fence (---) detected"
        ));
    }

    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    (result, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::{transform, Flags};

    fn no_op_flags() -> Flags {
        Flags {
            has_id: false,
            has_a: false,
            has_ab: false,
        }
    }

    #[test]
    fn renames_work_item_to_work_item_id_when_absent() {
        let content = "---\nwork-item: 0042\ntitle: A\n---\n";
        let (rewritten, diagnostics) =
            transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nwork_item_id: \"0042\"\ntitle: A\n---\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn drops_work_item_when_work_item_id_already_present() {
        let content = "---\nwork-item: 0042\nwork_item_id: \"0099\"\n---\n";
        let flags = Flags {
            has_id: true,
            ..no_op_flags()
        };
        let (rewritten, diagnostics) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, "---\nwork_item_id: \"0099\"\n---\n");
        assert!(diagnostics
            .iter()
            .any(|d| d.contains("work-item=0042 vs work_item_id=\"0099\"")));
    }

    #[test]
    fn refuses_an_unsafe_work_item_value_and_keeps_the_line() {
        let content = "---\nwork-item: \"bad # value\n---\n";
        let (rewritten, diagnostics) =
            transform(content, "f.md", &no_op_flags());
        assert_eq!(content, rewritten);
        assert!(diagnostics
            .iter()
            .any(|d| d.starts_with("0006-REFUSE: f.md — refused work-item")));
    }

    #[test]
    fn keeps_only_the_first_duplicate_work_item_id() {
        let content =
            "---\nwork_item_id: \"0001\"\nwork_item_id: \"0002\"\n---\n";
        let (rewritten, diagnostics) =
            transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nwork_item_id: \"0001\"\n---\n");
        assert!(diagnostics
            .iter()
            .any(|d| d.contains("kept first \"0001\"")));
    }

    #[test]
    fn renames_researcher_to_author_preserving_the_original_line_suffix() {
        let content = "---\nresearcher:   alice  \n---\n";
        let (rewritten, _) = transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nauthor:   alice  \n---\n");
    }

    #[test]
    fn drops_researcher_when_author_already_present() {
        let content = "---\nresearcher: bob\nauthor: carol\n---\n";
        let flags = Flags {
            has_a: true,
            ..no_op_flags()
        };
        let (rewritten, diagnostics) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, "---\nauthor: carol\n---\n");
        assert!(diagnostics
            .iter()
            .any(|d| d.contains("researcher=bob vs author=carol")));
    }

    #[test]
    fn renames_body_researcher_label_before_first_h2_only() {
        let content = "---\n---\n\n**Researcher**: alice\n\n## Findings\n\n\
             **Researcher**: not-renamed\n";
        let (rewritten, _) = transform(content, "f.md", &no_op_flags());
        assert_eq!(
            rewritten,
            "---\n---\n\n**Author**: alice\n\n## Findings\n\n\
             **Researcher**: not-renamed\n"
        );
    }

    #[test]
    fn drops_only_the_first_body_researcher_when_author_present() {
        let content =
            "---\n---\n\n**Researcher**: alice\n**Researcher**: second\n";
        let flags = Flags {
            has_ab: true,
            ..no_op_flags()
        };
        let (rewritten, _) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, "---\n---\n\n**Author**: second\n");
    }

    #[test]
    fn reports_malformed_when_legacy_key_appears_with_no_frontmatter_fence() {
        let content = "work-item: 0001\n\n# No frontmatter\n";
        let (rewritten, diagnostics) =
            transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, content);
        assert!(diagnostics.iter().any(|d| d.starts_with("0006-MALFORMED")));
    }

    #[test]
    fn a_single_quoted_work_item_value_is_normalised_to_double_quotes() {
        let content = "---\nwork-item: '0042'\n---\n";
        let (rewritten, _) = transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nwork_item_id: \"0042\"\n---\n");
    }

    #[test]
    fn a_single_quoted_value_with_an_embedded_quote_is_escaped() {
        let content = "---\nwork-item: 'the \"best\"'\n---\n";
        let (rewritten, _) = transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nwork_item_id: \"the \\\"best\\\"\"\n---\n");
    }

    #[test]
    fn trailing_whitespace_after_an_already_quoted_value_is_trimmed() {
        let content = "---\nwork-item: \"0042\"   \n---\n";
        let (rewritten, diagnostics) =
            transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nwork_item_id: \"0042\"\n---\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn an_empty_work_item_value_normalises_to_a_bare_key() {
        let content = "---\nwork-item:\n---\n";
        let (rewritten, _) = transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nwork_item_id:\n---\n");
    }

    #[test]
    fn an_empty_work_item_value_with_trailing_whitespace_still_normalises_to_a_bare_key(
    ) {
        let content = "---\nwork-item:   \n---\n";
        let (rewritten, _) = transform(content, "f.md", &no_op_flags());
        assert_eq!(rewritten, "---\nwork_item_id:\n---\n");
    }

    #[test]
    fn a_matching_legacy_and_canonical_work_item_value_drops_silently() {
        let content = "---\nwork-item: \"0042\"\nwork_item_id: \"0042\"\n---\n";
        let flags = Flags {
            has_id: true,
            ..no_op_flags()
        };
        let (rewritten, diagnostics) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, "---\nwork_item_id: \"0042\"\n---\n");
        assert!(
            !diagnostics.iter().any(|d| d.contains("work-item")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn a_matching_legacy_and_canonical_value_normalises_the_survivor_even_when_unquoted(
    ) {
        let content = "---\nwork-item: \"0042\"\nwork_item_id: 0042\n---\n";
        let flags = Flags {
            has_id: true,
            ..no_op_flags()
        };
        let (rewritten, diagnostics) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, "---\nwork_item_id: \"0042\"\n---\n");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn a_refused_line_coexisting_with_a_valid_canonical_line_is_not_also_diverged(
    ) {
        let content =
            "---\nwork-item: 0042 # note\nwork_item_id: \"0042\"\n---\n";
        let flags = Flags {
            has_id: true,
            ..no_op_flags()
        };
        let (rewritten, diagnostics) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, content);
        assert!(
            diagnostics.iter().any(|d| d.starts_with("0006-REFUSE")),
            "{diagnostics:?}"
        );
        assert!(
            !diagnostics.iter().any(|d| d.contains("DIVERGE")),
            "a refused line must skip the divergence comparison: \
             {diagnostics:?}"
        );
    }

    #[test]
    fn a_matching_researcher_and_author_value_drops_silently() {
        let content = "---\nresearcher: bob\nauthor: bob\n---\n";
        let flags = Flags {
            has_a: true,
            ..no_op_flags()
        };
        let (rewritten, diagnostics) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, "---\nauthor: bob\n---\n");
        assert!(
            !diagnostics.iter().any(|d| d.contains("researcher")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn a_matching_body_researcher_and_author_label_drops_silently() {
        let content = "---\n---\n\n**Researcher**: bob\n**Author**: bob\n";
        let flags = Flags {
            has_ab: true,
            ..no_op_flags()
        };
        let (rewritten, diagnostics) = transform(content, "f.md", &flags);
        assert_eq!(rewritten, "---\n---\n\n**Author**: bob\n");
        assert!(
            !diagnostics.iter().any(|d| d.contains("Researcher")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn two_pre_h2_researcher_labels_both_convert_when_no_author_exists() {
        let content =
            "---\n---\n\n**Researcher**: alice\n**Researcher**: bob\n";
        let (rewritten, _) = transform(content, "f.md", &no_op_flags());
        assert_eq!(
            rewritten,
            "---\n---\n\n**Author**: alice\n**Author**: bob\n"
        );
    }
}
