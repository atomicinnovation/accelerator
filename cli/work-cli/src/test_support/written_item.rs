//! Reading back a work item a writer produced, shared by this binary's unit
//! tests and, through `#[path]`, its subprocess suites.

use corpus::frontmatter_validation::pipeline::validate_text;
use corpus::frontmatter_validation::schema;
use corpus::frontmatter_validation::Violation;
use corpus_adapters::YamlFrontmatter;

/// The keys a writer leaves out when they have no value: the ones the
/// corpus schema declares optional for a work item.
///
/// # Panics
///
/// When the schema has no work-item row.
#[must_use]
pub fn omitted_when_empty() -> Vec<&'static str> {
    let work_item =
        schema::row_for("work-item", "").expect("a work-item schema row");
    work_item
        .typed_linkage_keys
        .iter()
        .chain(work_item.optional_extras)
        .copied()
        .collect()
}

/// # Panics
///
/// When `content` has no fenced frontmatter.
#[must_use]
pub fn frontmatter_of(content: &str) -> String {
    let (frontmatter, _) =
        work_adapters::sync::digest::split_frontmatter_and_body(content)
            .expect("a fenced frontmatter");
    frontmatter
}

#[must_use]
pub fn omit_when_empty_lines(frontmatter: &str) -> Vec<&str> {
    let omitted = omitted_when_empty();
    frontmatter
        .lines()
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(key, _)| omitted.contains(&key))
        })
        .collect()
}

#[must_use]
pub fn violations(content: &str) -> Vec<Violation> {
    validate_text(content, &YamlFrontmatter)
}
