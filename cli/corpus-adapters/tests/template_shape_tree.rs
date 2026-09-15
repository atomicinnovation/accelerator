//! The shipped `templates/` skeletons must carry zero template-shape
//! violations against the embedded schema TSV. A repository-integrity guard on
//! this repo's own artefacts, driven directly through the library rather than a
//! command-line surface: template resolution overlays user overrides, and the
//! plugin's default skeletons only ever exist at the repo root.

mod common;

use common::repo_root;
use common::TestError;

use corpus_adapters::frontmatter_validation::validate_templates;
use corpus_adapters::RealFs;

#[test]
fn the_shipped_templates_tree_is_clean() -> Result<(), TestError> {
    let violations = validate_templates(&repo_root()?, &RealFs)?;
    assert!(
        violations.is_empty(),
        "the shipped templates/ tree must carry zero template-shape \
         violations: {}",
        violations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

#[test]
fn the_codebase_research_template_is_born_without_a_research_title_prefix(
) -> Result<(), TestError> {
    let content = std::fs::read_to_string(
        repo_root()?.join("templates/codebase-research.md"),
    )?;

    let title = content
        .lines()
        .find(|line| line.starts_with("title:"))
        .ok_or("codebase-research template has no title: line")?;
    let title_value = title["title:".len()..].trim().trim_matches('"');
    assert!(
        !title_value.starts_with("Research: "),
        "the codebase-research template title must be born without the \
         'Research: ' prefix, got: {title_value}"
    );
    assert!(
        title_value.starts_with('{'),
        "the codebase-research template title must open with the topic \
         placeholder, got: {title_value}"
    );

    let mut fences = 0u8;
    let h1 = content
        .lines()
        .find(|line| {
            if *line == "---" {
                fences += 1;
            }
            fences >= 2 && line.starts_with("# ")
        })
        .ok_or("codebase-research template has no body H1")?;
    let h1_text = h1["# ".len()..].trim();
    assert!(
        !h1_text.starts_with("Research: "),
        "the codebase-research template H1 must be born without the \
         'Research: ' prefix, got: {h1}"
    );
    assert!(
        h1_text.starts_with('['),
        "the codebase-research template H1 must open with the topic \
         placeholder, got: {h1}"
    );
    Ok(())
}
