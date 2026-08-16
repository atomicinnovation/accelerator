//! Section extraction and comparison for conflict-resolution diffing.
//!
//! Compares normalised `String`s directly rather than hashing them: the
//! two are behaviourally identical here, and hashing only ever existed to
//! sidestep `diff`'s exit-status portability trap.

use crate::normalise::trim_lines;

/// A named section of a work-item-shaped document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionName {
    Frontmatter,
    Preamble,
    Heading(String),
}

impl SectionName {
    fn as_heading_text(&self) -> Option<&str> {
        match self {
            Self::Heading(name) => Some(name),
            Self::Frontmatter | Self::Preamble => None,
        }
    }
}

/// Ordered, de-duplicated `## ` heading list across both files, in
/// first-appearance order: local first, then remote.
#[must_use]
pub fn heading_union(local: &[String], remote: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for heading in local.iter().chain(remote.iter()) {
        if seen.insert(heading.clone()) {
            out.push(heading.clone());
        }
    }
    out
}

/// Empty when line 1 is not `---` (no frontmatter) or the frontmatter never
/// closes — both are failure arms, and neither is distinguished here.
fn extract_frontmatter(content: &str) -> String {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return String::new();
    };
    if first.trim_end() != "---" {
        return String::new();
    }
    let mut out = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    if closed {
        out
    } else {
        String::new()
    }
}

/// The whole file verbatim when line 1 is not `---` (no frontmatter), empty
/// when the frontmatter never closes, otherwise everything after the
/// closing `---` line.
fn body_after_frontmatter(content: &str) -> String {
    let mut lines = content.lines();
    let Some(first) = lines.next() else {
        return String::new();
    };
    if first.trim_end() != "---" {
        let mut out = String::new();
        out.push_str(first);
        out.push('\n');
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
        return out;
    }
    let mut closed = false;
    let mut out = String::new();
    for line in lines {
        if !closed {
            if line.trim_end() == "---" {
                closed = true;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if closed {
        out
    } else {
        String::new()
    }
}

/// Extracts one named section's raw content.
#[must_use]
pub fn extract_section(content: &str, name: &SectionName) -> String {
    match name {
        SectionName::Frontmatter => extract_frontmatter(content),
        SectionName::Preamble => {
            let body = body_after_frontmatter(content);
            let mut out = String::new();
            for line in body.lines() {
                if line.starts_with("## ") {
                    break;
                }
                out.push_str(line);
                out.push('\n');
            }
            out
        }
        SectionName::Heading(heading) => {
            let body = body_after_frontmatter(content);
            let marker = format!("## {heading}");
            let mut out = String::new();
            let mut grabbing = false;
            for line in body.lines() {
                if line == marker {
                    grabbing = true;
                    continue;
                }
                if grabbing && line.starts_with("## ") {
                    break;
                }
                if grabbing {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out
        }
    }
}

fn headings_in(content: &str) -> Vec<String> {
    body_after_frontmatter(content)
        .lines()
        .filter_map(|line| line.strip_prefix("## ").map(str::to_owned))
        .collect()
}

/// A section whose trimmed content differs between local and remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDiff {
    pub name: String,
    pub local: String,
    pub remote: String,
}

fn section_label(name: &SectionName) -> String {
    match name {
        SectionName::Frontmatter => "frontmatter".to_owned(),
        SectionName::Preamble => "(preamble)".to_owned(),
        SectionName::Heading(heading) => heading.clone(),
    }
}

/// The sections that differ (after `--stdin`-equivalent trim-only
/// normalisation) between `local_content` and `remote_content`.
///
/// Sections are compared in frontmatter, preamble, then heading-union
/// order. Reproduces the un-stripped-`IGNORE_KEYS` behaviour on the
/// frontmatter section exactly (see the module-level docs on
/// [`crate::normalise`]): a change limited to `last_updated`/`revision`
/// still shows as differing.
#[must_use]
pub fn differing_sections(
    local_content: &str,
    remote_content: &str,
) -> Vec<SectionDiff> {
    let local_headings = headings_in(local_content);
    let remote_headings = headings_in(remote_content);
    let headings = heading_union(&local_headings, &remote_headings);

    let mut names = vec![SectionName::Frontmatter, SectionName::Preamble];
    names.extend(headings.into_iter().map(SectionName::Heading));

    let mut out = Vec::new();
    for name in &names {
        let local_raw = extract_section(local_content, name);
        let remote_raw = extract_section(remote_content, name);
        let local_trimmed = trim_lines(&local_raw);
        let remote_trimmed = trim_lines(&remote_raw);
        if local_trimmed == remote_trimmed {
            continue;
        }
        out.push(SectionDiff {
            name: name
                .as_heading_text()
                .map_or_else(|| section_label(name), str::to_owned),
            local: local_trimmed,
            remote: remote_trimmed,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{differing_sections, SectionName};

    fn names(diffs: &[super::SectionDiff]) -> Vec<&str> {
        diffs.iter().map(|d| d.name.as_str()).collect()
    }

    #[test]
    fn frontmatter_only_change_is_reported() {
        let local = "---\nstatus: draft\ntitle: Foo\n---\n## Summary\ntext\n";
        let remote = "---\nstatus: ready\ntitle: Foo\n---\n## Summary\ntext\n";
        assert_eq!(
            names(&differing_sections(local, remote)),
            vec!["frontmatter"]
        );
    }

    #[test]
    fn preamble_change_is_reported() {
        let local =
            "---\nstatus: draft\n---\npreamble local\n## Summary\ntext\n";
        let remote =
            "---\nstatus: draft\n---\npreamble remote\n## Summary\ntext\n";
        assert_eq!(
            names(&differing_sections(local, remote)),
            vec!["(preamble)"]
        );
    }

    #[test]
    fn named_heading_diff_one_sided_heading_and_identical_section_omitted() {
        let local = "---\nstatus: draft\n---\n## Summary\nlocal summary\n## Requirements\nsame text\n";
        let remote = "---\nstatus: draft\n---\n## Summary\nremote summary\n## Requirements\nsame text\n## Open Questions\nnew section\n";
        assert_eq!(
            names(&differing_sections(local, remote)),
            vec!["Summary", "Open Questions"]
        );
    }

    #[test]
    fn byte_identical_after_normalisation_yields_no_differences() {
        let local = "---\nstatus: draft   \n---\n## Summary\ntext   \n";
        let remote = "---\nstatus: draft\n---\n## Summary\ntext\n";
        assert!(differing_sections(local, remote).is_empty());
    }

    #[test]
    fn frontmatter_only_last_updated_change_is_still_reported() {
        let local = "---\nstatus: draft\nlast_updated: \"2026-01-01T00:00:00Z\"\n---\n## Summary\ntext\n";
        let remote = "---\nstatus: draft\nlast_updated: \"2026-01-02T00:00:00Z\"\n---\n## Summary\ntext\n";
        assert_eq!(
            names(&differing_sections(local, remote)),
            vec!["frontmatter"]
        );
    }

    #[test]
    fn extract_section_reads_a_named_heading() {
        let content = "---\nstatus: draft\n---\n## Summary\nhello\n## Requirements\nworld\n";
        assert_eq!(
            super::extract_section(
                content,
                &SectionName::Heading("Summary".to_owned())
            ),
            "hello\n"
        );
    }
}
