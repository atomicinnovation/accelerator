//! Strips the `Research: ` title prefix from codebase-research documents.
//!
//! A content-gated, quote-aware rewrite over the frontmatter `title:` and the
//! pre-first-`## ` body H1 of every file under `paths.research_codebase`. The
//! rewrite is idempotent: a stripped title no longer carries the prefix, so a
//! second application is byte-stable.
//!
//! `m0006`'s quote helpers (`is_double_quoted`, `semantic_inner`,
//! `normalise_value`, `refuses`) and its positional body anchoring are
//! duplicated inline rather than shared through `migrations/text.rs`: an
//! applied migration is immutable history, and sharing helpers that later
//! evolve would risk silently changing this migration's byte-stable behaviour
//! or shifting the public-API snapshot.

use std::path::Path;

use crate::ports::MigrationContext;
use crate::ports::MigrationError;
use crate::registry::ApplyOutcome;
use crate::registry::Migration;
use crate::registry::MigrationMeta;

pub struct Migration0010;

const RESEARCH_PREFIX: &str = "Research: ";

impl MigrationMeta for Migration0010 {
    fn id(&self) -> &'static str {
        "0010-strip-research-title-prefix"
    }

    fn description(&self) -> &'static str {
        "Strip the 'Research: ' prefix from the frontmatter title and body \
         H1 of every codebase-research document. Idempotent; only files whose \
         title or H1 literally begins with the prefix are rewritten."
    }
}

impl Migration for Migration0010 {
    fn apply(
        &self,
        ctx: &dyn MigrationContext,
    ) -> Result<ApplyOutcome, MigrationError> {
        let root = ctx.root().to_path_buf();
        strip_prefix_across_corpus(ctx, &root)?;
        Ok(ApplyOutcome::Applied)
    }
}

fn strip_prefix_across_corpus(
    ctx: &dyn MigrationContext,
    root: &Path,
) -> Result<(), MigrationError> {
    let Some(rel) = resolve_corpus_path(ctx)? else {
        return Ok(());
    };
    let abs = root.join(&rel);
    if !ctx.dir_exists(&abs) {
        eprintln!(
            "Warning: 0010: research_codebase directory does not exist: {rel}"
        );
        eprintln!("0010: rewrote 0 file(s) under {rel}");
        return Ok(());
    }
    let mut rewrote = 0usize;
    for file in ctx.list_md_files(&abs)? {
        if rewrite_file(ctx, &file)? {
            rewrote += 1;
        }
    }
    eprintln!("0010: rewrote {rewrote} file(s) under {rel}");
    Ok(())
}

fn resolve_corpus_path(
    ctx: &dyn MigrationContext,
) -> Result<Option<String>, MigrationError> {
    let rel = ctx
        .config_value("paths.research_codebase")?
        .unwrap_or_default();
    if rel.is_empty() {
        eprintln!(
            "Warning: 0010: config path returned empty for \
             'research_codebase' — skipping corpus"
        );
        return Ok(None);
    }
    if is_dangerous_path(&rel) {
        eprintln!(
            "Warning: 0010: refusing dangerous paths.research_codebase \
             value: {rel}"
        );
        return Ok(None);
    }
    Ok(Some(rel))
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

fn rewrite_file(
    ctx: &dyn MigrationContext,
    file: &Path,
) -> Result<bool, MigrationError> {
    let Some(content) = ctx.read(file)? else {
        return Ok(false);
    };
    let rewritten = strip_research_title_prefix(&content);
    if rewritten != content {
        ctx.write(file, &rewritten)?;
        return Ok(true);
    }
    Ok(false)
}

/// The pure transform: strip the `Research: ` prefix from the frontmatter
/// title and the pre-first-`## ` body H1, or return the input unchanged when
/// neither carries the prefix.
fn strip_research_title_prefix(content: &str) -> String {
    if !carries_research_title_prefix(content) {
        return content.to_owned();
    }

    let mut out: Vec<String> = Vec::new();
    let mut in_frontmatter = false;
    let mut seen_frontmatter_open = false;
    let mut saw_first_h2 = false;

    for line in content.lines() {
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

        if in_frontmatter && line.starts_with("title:") {
            if let Some(rewritten) = strip_frontmatter_title(line) {
                out.push(rewritten);
                continue;
            }
        } else if !in_frontmatter && !saw_first_h2 {
            if let Some(rest) = line.strip_prefix("# Research: ") {
                out.push(format!("# {rest}"));
                continue;
            }
        }
        out.push(line.to_owned());
    }

    let mut result = out.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// The content pre-gate. Computed from the same in-frontmatter `title:` and
/// positional `!in_frontmatter` scan the rewrite uses, so the "touch this
/// file" and "which lines to strip" decisions cannot drift.
fn carries_research_title_prefix(content: &str) -> bool {
    let mut in_frontmatter = false;
    let mut seen_frontmatter_open = false;
    let mut saw_first_h2 = false;

    for line in content.lines() {
        if !seen_frontmatter_open && line == "---" {
            seen_frontmatter_open = true;
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && line == "---" {
            in_frontmatter = false;
            continue;
        }
        if line.starts_with("## ") {
            saw_first_h2 = true;
        }
        if in_frontmatter
            && line.starts_with("title:")
            && frontmatter_title_is_prefixed(line)
        {
            return true;
        }
        if !in_frontmatter && !saw_first_h2 && line.starts_with("# Research: ")
        {
            return true;
        }
    }
    false
}

fn frontmatter_title_is_prefixed(line: &str) -> bool {
    let value = title_value(line);
    !refuses(value) && semantic_inner(value).starts_with(RESEARCH_PREFIX)
}

fn strip_frontmatter_title(line: &str) -> Option<String> {
    let value = title_value(line);
    if refuses(value) {
        return None;
    }
    let stripped = semantic_inner(value).strip_prefix(RESEARCH_PREFIX)?;
    let reconstructed = if is_double_quoted(value) {
        format!("\"{stripped}\"")
    } else if is_single_quoted(value) {
        format!("'{stripped}'")
    } else {
        stripped.to_owned()
    };
    Some(format!("title: {}", normalise_value(&reconstructed)))
}

fn title_value(line: &str) -> &str {
    line["title:".len()..].trim_matches([' ', '\t'])
}

fn is_double_quoted(value: &str) -> bool {
    value.len() >= 2 && value.starts_with('"') && value.ends_with('"')
}

fn is_single_quoted(value: &str) -> bool {
    value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'')
}

fn semantic_inner(value: &str) -> &str {
    if is_double_quoted(value) || is_single_quoted(value) {
        &value[1..value.len() - 1]
    } else {
        value
    }
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

fn refuses(value: &str) -> bool {
    if is_double_quoted(value) || is_single_quoted(value) {
        return false;
    }
    value.contains('#') || value.contains('"')
}

#[cfg(test)]
mod tests {
    use super::strip_research_title_prefix;

    #[test]
    fn a_double_quoted_title_is_stripped() {
        let content = "---\ntitle: \"Research: Foo\"\n---\n";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"Foo\"\n---\n"
        );
    }

    #[test]
    fn a_single_quoted_title_strips_and_normalises_to_double_quotes() {
        let content = "---\ntitle: 'Research: Foo'\n---\n";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"Foo\"\n---\n"
        );
    }

    #[test]
    fn an_unquoted_title_strips_and_is_wrapped_in_double_quotes() {
        let content = "---\ntitle: Research: Foo\n---\n";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"Foo\"\n---\n"
        );
    }

    #[test]
    fn an_unquoted_title_with_a_hash_is_left_byte_identical() {
        let content = "---\ntitle: Research: Foo # note\n---\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn an_unquoted_title_with_a_quote_is_left_byte_identical() {
        let content = "---\ntitle: Research: a\"b\n---\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn a_double_quoted_embedded_quote_survives_stripping() {
        let content = "---\ntitle: \"Research: the \\\"best\\\"\"\n---\n";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"the \\\"best\\\"\"\n---\n"
        );
    }

    #[test]
    fn a_block_scalar_title_is_left_byte_identical() {
        let content = "---\ntitle: |\n  Research: Foo\n---\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn a_near_miss_researchers_title_never_matches() {
        let content = "---\ntitle: \"Researchers: Foo\"\n---\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn a_body_h1_before_the_first_h2_is_stripped() {
        let content =
            "---\ntitle: \"Foo\"\n---\n\n# Research: Foo\n\n## Section\n";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"Foo\"\n---\n\n# Foo\n\n## Section\n"
        );
    }

    #[test]
    fn a_body_h1_after_the_first_h2_is_left_alone() {
        let content = "---\ntitle: \"Foo\"\n---\n\n## Section\n\n\
                       # Research: Later\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn a_hash_comment_inside_the_frontmatter_never_matches_as_an_h1() {
        let content = "---\n# Research: not an h1\ntitle: \"Foo\"\n---\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn an_h2_research_heading_never_matches() {
        let content = "---\ntitle: \"Foo\"\n---\n\n## Research: Section\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn both_the_title_and_the_body_h1_strip_together() {
        let content = "---\ntitle: \"Research: Foo\"\n---\n\n# Research: Foo\n";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"Foo\"\n---\n\n# Foo\n"
        );
    }

    #[test]
    fn a_prefix_free_document_is_byte_identical() {
        let content = "---\ntitle: \"Foo\"\n---\n\n# Foo\n\nBody\n";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn a_prefix_free_document_without_a_trailing_newline_is_byte_identical() {
        let content = "---\ntitle: \"Foo\"\n---\n\n# Foo";
        assert_eq!(strip_research_title_prefix(content), content);
    }

    #[test]
    fn a_prefixed_title_without_a_trailing_newline_keeps_its_ending() {
        let content = "---\ntitle: \"Research: Foo\"\n---";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"Foo\"\n---"
        );
    }

    #[test]
    fn a_pre_h2_fenced_lookalike_is_rewritten_by_the_positional_guard() {
        // The pre-`## ` region is assumed fence-free by the
        // codebase-research template convention; the positional guard does
        // not detect fences, so a column-0 `# Research:` in that region is
        // rewritten. This pins that accepted behaviour.
        let content = "---\ntitle: \"Foo\"\n---\n\n```\n# Research: fenced\n\
                       ```\n\n## Section\n";
        assert_eq!(
            strip_research_title_prefix(content),
            "---\ntitle: \"Foo\"\n---\n\n```\n# fenced\n```\n\n## Section\n"
        );
    }

    #[test]
    fn the_transform_is_idempotent() {
        let content = "---\ntitle: \"Research: Foo\"\n---\n\n# Research: Foo\n";
        let once = strip_research_title_prefix(content);
        let twice = strip_research_title_prefix(&once);
        assert_eq!(once, twice);
    }
}
