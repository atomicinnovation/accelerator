//! The fence-less backfill: synthesises a canonical frontmatter block for a
//! file with no strict leading fence (including a *loose*-fenced file, e.g.
//! a legacy note whose `---` carries trailing whitespace), preserving the
//! original body. A line-oriented port of `backfill_file`.

use std::path::PathBuf;

use corpus::doc_type::DocTypeKey;
use corpus::frontmatter_validation::schema as fm_schema;

use crate::migrations::m0007::rewrite::derive_id;
use crate::migrations::m0007::rewrite::derive_stem;

fn is_loose_fence(line: &str) -> bool {
    line.trim_end() == "---"
}

fn first_h1(content: &str) -> Option<String> {
    let line = content.lines().find(|line| line.starts_with("# "))?;
    Some(line[2..].trim_end().to_owned())
}

fn strip_existing_pseudo_frontmatter(content: &str) -> &str {
    if !content.lines().next().is_some_and(is_loose_fence) {
        return content;
    }
    let mut fence_count = 0u32;
    let mut offset = 0usize;
    for line in content.lines() {
        offset += line.len() + 1;
        if is_loose_fence(line) {
            fence_count += 1;
            if fence_count == 2 {
                return content.get(offset..).unwrap_or_default();
            }
        }
    }
    content
}

fn filename_date_prefix(stem: &str) -> Option<String> {
    let bytes = stem.as_bytes();
    let date_shaped = bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit);
    date_shaped.then(|| format!("{}T00:00:00+00:00", &stem[..10]))
}

/// Synthesises canonical frontmatter for a fence-less `content`, returning
/// the new document and any diagnostic (an anchored type with no derivable
/// revision).
#[must_use]
pub fn backfill(
    content: &str,
    relpath: &str,
    table: &[(DocTypeKey, PathBuf)],
    repo_name: &str,
) -> (String, Vec<String>) {
    let linkage_type =
        corpus::linkage::type_from_path(relpath, table).unwrap_or_default();
    let stem = derive_stem(relpath, linkage_type);
    let title = first_h1(content)
        .unwrap_or_else(|| stem.clone())
        .replace('"', "");
    let iso = filename_date_prefix(&stem);
    let author = "Unknown";
    let anchored = fm_schema::row_for(linkage_type)
        .is_some_and(|row| row.code_state_anchored);

    let mut diagnostics = Vec::new();
    if anchored {
        diagnostics.push(format!(
            "0007-DIVERGE[author-lookup-failed]: {relpath} — no VCS revision for anchored backfill"
        ));
    }

    let id = derive_id(linkage_type, &stem);
    let mut lines = vec![
        "---".to_owned(),
        format!("type: {linkage_type}"),
        format!("id: \"{id}\""),
        format!("title: \"{title}\""),
    ];
    if let Some(iso) = &iso {
        lines.push(format!("date: \"{iso}\""));
    }
    lines.push(format!("author: {author}"));
    if linkage_type == "note" {
        lines.push("producer: create-note".to_owned());
        lines.push("status: captured".to_owned());
    }
    if matches!(
        linkage_type,
        "note" | "codebase-research" | "issue-research"
    ) {
        lines.push(format!("topic: \"{title}\""));
    }
    lines.push("tags: []".to_owned());
    if anchored {
        lines.push(format!("repository: \"{repo_name}\""));
    }
    if let Some(iso) = &iso {
        lines.push(format!("last_updated: \"{iso}\""));
    }
    lines.push(format!("last_updated_by: {author}"));
    lines.push("schema_version: 1".to_owned());
    lines.push("---".to_owned());
    let frontmatter = lines.join("\n") + "\n\n";

    let body = strip_existing_pseudo_frontmatter(content);
    (format!("{frontmatter}{body}"), diagnostics)
}

#[cfg(test)]
mod tests {
    use super::backfill;
    use corpus::doc_type::DocTypeKey;
    use std::path::PathBuf;

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![(DocTypeKey::Notes, PathBuf::from("meta/notes"))]
    }

    #[test]
    fn backfills_a_bare_note_with_no_frontmatter_at_all() {
        let content = "# My Note\n\nSome body text.\n";
        let (result, diagnostics) = backfill(
            content,
            "meta/notes/2026-01-01-my-note.md",
            &table(),
            "repo",
        );
        assert!(result.starts_with("---\ntype: note\n"));
        assert!(result.contains("id: \"2026-01-01-my-note\"\n"));
        assert!(result.contains("title: \"My Note\"\n"));
        assert!(result.contains("date: \"2026-01-01T00:00:00+00:00\"\n"));
        assert!(result.contains("producer: create-note\n"));
        assert!(result.contains("status: captured\n"));
        assert!(result.contains("topic: \"My Note\"\n"));
        assert!(result.contains("tags: []\n"));
        assert!(result.contains("schema_version: 1\n"));
        assert!(result.ends_with("---\n\n# My Note\n\nSome body text.\n"));
        // `note` is a code-state-anchored type; with no VCS per-file history
        // query available (see the module's own author/revision-resolution
        // narrowing), a missing revision always breadcrumbs.
        assert!(diagnostics
            .iter()
            .any(|d| d.contains("no VCS revision for anchored backfill")));
    }

    #[test]
    fn strips_an_existing_loose_fenced_pseudo_frontmatter() {
        let content = "--- \nold: stuff\n--- \n\nBody kept.\n";
        let (result, _) =
            backfill(content, "meta/notes/note.md", &table(), "repo");
        // The retired bash source this mirrors reproduces every line after
        // the second fence verbatim, including the original blank
        // separator line — the synthesised frontmatter's own trailing blank
        // line (`---\n\n`) is therefore followed by that preserved blank
        // line, not collapsed into it.
        assert!(result.ends_with("---\n\n\nBody kept.\n"));
        assert!(!result.contains("old: stuff"));
    }

    #[test]
    fn falls_back_to_the_stem_when_no_h1_is_present() {
        let content = "no heading here\n";
        let (result, _) =
            backfill(content, "meta/notes/plain-stem.md", &table(), "repo");
        assert!(result.contains("title: \"plain-stem\"\n"));
    }
}
