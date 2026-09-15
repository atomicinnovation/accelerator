//! Repository-integrity guards on the research infrastructure's plugin
//! markdown: the generic `researcher` agent must carry a bounded, shell-free
//! tool grant, and the `topic-research-finding` template the finding outputter
//! delegates to must declare exactly the fields the `(topic-research, finding)`
//! schema row requires, plus the omit-when-empty linkage slots — so the writer,
//! the template, and the schema cannot drift silently.

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
fn the_finding_template_matches_the_finding_schema_row() -> Result<(), TestError>
{
    let content = read("templates/topic-research-finding.md")?;
    let frontmatter = extract_frontmatter(&content);
    let declared = field_names(&frontmatter);

    let row = schema::row_for("topic-research", "finding")
        .ok_or("no (topic-research, finding) schema row")?;
    let mut expected: BTreeSet<String> = schema::BASE_FIELDS
        .iter()
        .map(|f| (*f).to_owned())
        .collect();
    expected.extend(["producer", "status", "kind"].map(str::to_owned));
    expected.extend(row.extras.iter().map(|extra| (*extra).to_owned()));
    // The template also carries the omit-when-empty typed-linkage slots, which a
    // written finding drops when they have no value.
    expected.extend(["parent", "relates_to"].map(str::to_owned));

    assert_eq!(
        declared, expected,
        "the finding template's fields must equal the (topic-research, \
         finding) required set plus the omit-when-empty linkage slots"
    );
    Ok(())
}
