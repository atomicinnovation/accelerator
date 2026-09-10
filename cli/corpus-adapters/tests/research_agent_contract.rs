//! Repository-integrity guards on the research infrastructure's plugin
//! markdown: the generic `researcher` agent must carry a bounded, shell-free
//! tool grant, and the finding outputter's authoring scaffold must declare
//! exactly the fields the `(topic-research, finding)` schema row requires — so
//! the writer, the template, and the schema cannot drift silently.

mod common;

use std::collections::BTreeSet;

use common::repo_root;
use common::TestError;
use corpus::frontmatter_validation::parse_entries;
use corpus::frontmatter_validation::schema;
use corpus::frontmatter_validation::template_shape::extract_frontmatter;

fn read(relative: &str) -> Result<String, TestError> {
    Ok(std::fs::read_to_string(repo_root()?.join(relative))?)
}

fn field_names(frontmatter: &str) -> BTreeSet<String> {
    parse_entries(frontmatter)
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

/// The content of each ```` ``` ````-fenced block in `markdown`.
fn fenced_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            match current.take() {
                Some(lines) => blocks.push(lines.join("\n")),
                None => current = Some(Vec::new()),
            }
            continue;
        }
        if let Some(lines) = current.as_mut() {
            lines.push(line);
        }
    }
    blocks
}

#[test]
fn the_researcher_agent_grants_a_bounded_shell_free_tool_set(
) -> Result<(), TestError> {
    let content = read("agents/researcher.md")?;
    let frontmatter = extract_frontmatter(&content);
    let entries = parse_entries(&frontmatter);
    let tools = entries
        .iter()
        .find(|(key, _)| key == "tools")
        .map(|(_, value)| value.as_str())
        .ok_or("agents/researcher.md carries no tools: grant")?;
    let granted: BTreeSet<&str> = tools.split(',').map(str::trim).collect();

    let expected: BTreeSet<&str> = ["WebSearch", "WebFetch", "Write", "Read"]
        .into_iter()
        .collect();
    assert_eq!(
        granted, expected,
        "the researcher's tool grant has drifted from the pinned set"
    );
    assert!(
        !granted.contains("Bash"),
        "the researcher must never be granted Bash — it consumes \
         attacker-controlled web content"
    );
    Ok(())
}

#[test]
fn the_finding_outputters_scaffold_matches_the_finding_schema_row(
) -> Result<(), TestError> {
    let content =
        read("skills/research/outputters/finding-outputter/SKILL.md")?;
    let scaffold = fenced_blocks(&content)
        .into_iter()
        .map(|block| extract_frontmatter(&block))
        .find(|frontmatter| !frontmatter.trim().is_empty())
        .ok_or("the finding outputter carries no example frontmatter block")?;
    let declared = field_names(&scaffold);

    let row = schema::row_for("topic-research", "finding")
        .ok_or("no (topic-research, finding) schema row")?;
    let mut expected: BTreeSet<String> = schema::BASE_FIELDS
        .iter()
        .map(|f| (*f).to_owned())
        .collect();
    expected.extend(["producer", "status", "kind"].map(str::to_owned));
    expected.extend(row.extras.iter().map(|extra| (*extra).to_owned()));

    assert_eq!(
        declared, expected,
        "the finding outputter's scaffold fields must equal the \
         (topic-research, finding) required set"
    );
    Ok(())
}
