//! In-process section diffing for the `work` domain crate.

use similar::TextDiff;
use work::section_diff::SectionDiff;

/// Renders one section's header plus an in-process unified-diff body.
///
/// The `=== name (- LOCAL / + REMOTE) ===` header and blank-line framing is
/// the contract callers depend on; the body is `similar`'s hunk output.
pub fn render(diff: &SectionDiff) -> String {
    let body = TextDiff::from_lines(&diff.local, &diff.remote)
        .unified_diff()
        .context_radius(3)
        .to_string();

    let mut out = format!("=== {} (- LOCAL / + REMOTE) ===\n", diff.name);
    out.push_str(&body);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use work::section_diff::SectionDiff;

    use super::render;

    fn diff(name: &str, local: &str, remote: &str) -> SectionDiff {
        SectionDiff {
            name: name.to_owned(),
            local: local.to_owned(),
            remote: remote.to_owned(),
        }
    }

    #[test]
    fn renders_the_frozen_header_and_a_unified_body() {
        let section = diff("Summary", "local summary\n", "remote summary\n");
        let rendered = render(&section);
        assert!(rendered.starts_with("=== Summary (- LOCAL / + REMOTE) ===\n"));
        assert!(rendered.contains("@@ -1 +1 @@"));
        assert!(rendered.contains("-local summary"));
        assert!(rendered.contains("+remote summary"));
        assert!(!rendered.contains("No newline at end of file"));
        assert!(rendered.ends_with("\n\n"));
    }

    #[test]
    fn identical_sides_render_an_empty_body_under_the_header() {
        let section = diff("Summary", "same text\n", "same text\n");
        let rendered = render(&section);
        assert_eq!(rendered, "=== Summary (- LOCAL / + REMOTE) ===\n\n");
    }

    #[test]
    fn a_section_added_on_the_remote_renders_a_pure_insertion() {
        let section = diff("Summary", "", "remote only\n");
        let rendered = render(&section);
        assert!(rendered.starts_with("=== Summary (- LOCAL / + REMOTE) ===\n"));
        assert!(rendered.contains("@@ -0,0 +1 @@"));
        assert!(rendered.contains("+remote only"));
        assert!(!rendered.contains("No newline at end of file"));
        assert!(rendered.ends_with("\n\n"));
    }

    #[test]
    fn a_section_dropped_on_the_remote_renders_a_pure_deletion() {
        let section = diff("Summary", "local only\n", "");
        let rendered = render(&section);
        assert!(rendered.contains("@@ -1 +0,0 @@"));
        assert!(rendered.contains("-local only"));
        assert!(!rendered.contains("No newline at end of file"));
    }
}
