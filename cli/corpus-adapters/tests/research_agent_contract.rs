//! Repository-integrity guards on the research infrastructure's plugin
//! markdown: the generic `researcher` agent must carry a bounded tool grant
//! whose `Bash` only the registered research guard confines; each academic
//! profile must reach its source only through a fetch the guard passes;
//! and the `topic-research-finding` template the finding outputter
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
use research::confinement::command_decision;
use research::confinement::Decision;
use serde_json::Value;

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
fn the_researcher_agent_grants_a_bounded_tool_set() -> Result<(), TestError> {
    let content = read("agents/researcher.md")?;
    let frontmatter = extract_frontmatter(&content);
    let entries = parse_entries(&frontmatter);
    let tools = entries
        .iter()
        .find(|(key, _)| key == "tools")
        .map(|(_, value)| value.as_str())
        .ok_or("agents/researcher.md carries no tools: grant")?;
    let granted: BTreeSet<&str> = tools.split(',').map(str::trim).collect();

    let expected: BTreeSet<&str> =
        ["WebSearch", "WebFetch", "Write", "Read", "Bash"]
            .into_iter()
            .collect();
    assert_eq!(
        granted, expected,
        "the researcher's tool grant has drifted from the pinned set"
    );
    Ok(())
}

const RESEARCH_GUARD: &str =
    "/bin/accelerator research guard --fail-safe --non-blocking";

fn pre_tool_use_commands(matcher: &str) -> Result<Vec<String>, TestError> {
    let hooks: Value = serde_json::from_str(&read("hooks/hooks.json")?)?;
    let groups = hooks["hooks"]["PreToolUse"]
        .as_array()
        .ok_or("hooks/hooks.json registers no PreToolUse hooks")?;
    Ok(groups
        .iter()
        .filter(|group| group["matcher"] == matcher)
        .filter_map(|group| group["hooks"].as_array())
        .flatten()
        .filter_map(|hook| hook["command"].as_str().map(str::to_owned))
        .collect())
}

#[test]
fn bash_is_granted_only_beside_the_registered_research_guard(
) -> Result<(), TestError> {
    for matcher in ["Bash", "Write|Edit|MultiEdit|NotebookEdit"] {
        let commands = pre_tool_use_commands(matcher)?;
        assert!(
            commands
                .iter()
                .any(|command| command.ends_with(RESEARCH_GUARD)),
            "the {matcher} PreToolUse group does not run the research \
             guard: {commands:?}"
        );
    }
    Ok(())
}

#[test]
fn the_researcher_body_names_no_source_family() -> Result<(), TestError> {
    let content = read("agents/researcher.md")?;
    let body = content
        .split_once("\n---\n")
        .map_or(content.as_str(), |(_, body)| body)
        .to_lowercase();
    for family in ["openalex", "arxiv", "web-profile"] {
        assert!(
            !body.contains(family),
            "the researcher's body names {family}; profiles are injected"
        );
    }
    Ok(())
}

#[test]
fn every_profile_invocation_passes_the_guard() -> Result<(), TestError> {
    for family in ["openalex", "arxiv"] {
        let content = read(&format!(
            "skills/research/profiles/{family}-profile/SKILL.md"
        ))?;
        let invocations = fenced_lines(&content)
            .into_iter()
            .chain(quoted_queries(&content).into_iter().map(|query| {
                format!("accelerator research fetch {family} search {query}")
            }))
            .collect::<Vec<_>>();
        assert!(
            invocations.len() > 2,
            "the {family} profile shows too few invocations: {invocations:?}"
        );
        for invocation in invocations {
            assert_eq!(
                command_decision(&invocation),
                Decision::Pass,
                "the {family} profile shows a call the guard blocks: \
                 {invocation}"
            );
        }
    }
    Ok(())
}

fn fenced_lines(content: &str) -> Vec<String> {
    let mut in_fence = false;
    let mut lines = Vec::new();
    for line in content.lines().map(str::trim) {
        if line.starts_with("```") {
            in_fence = !in_fence;
        } else if in_fence && !line.is_empty() {
            lines.push(line.to_owned());
        }
    }
    lines
}

/// The worked examples of a query written as one single-quoted argument.
fn quoted_queries(content: &str) -> Vec<String> {
    content
        .split("`'")
        .skip(1)
        .filter_map(|rest| rest.split_once("'`"))
        .map(|(query, _)| format!("'{query}'"))
        .collect()
}

#[test]
fn the_openalex_profile_never_instructs_web_fetching() -> Result<(), TestError>
{
    assert_profile_only_fetches("openalex", "OpenAlex")
}

#[test]
fn the_arxiv_profile_never_instructs_web_fetching() -> Result<(), TestError> {
    assert_profile_only_fetches("arxiv", "arXiv")
}

/// An academic profile grants the fetch and no web tool, mentions a web
/// tool only to forbid it, and runs nothing but its own family's fetch.
fn assert_profile_only_fetches(
    family: &str,
    name: &str,
) -> Result<(), TestError> {
    let content = read(&format!(
        "skills/research/profiles/{family}-profile/SKILL.md"
    ))?;
    let frontmatter = extract_frontmatter(&content);
    assert!(
        frontmatter.contains("Bash(accelerator research fetch *)"),
        "the {name} profile must grant the fetch"
    );
    assert!(
        !frontmatter.contains("Web"),
        "the {name} profile must grant no web tool: {frontmatter}"
    );

    let body = content
        .split_once("\n---\n")
        .map_or(content.as_str(), |(_, body)| body);
    for line in body.lines().filter(|line| line.contains("Web")) {
        let text = line.trim_start().trim_start_matches("- ");
        assert!(
            text.starts_with("No ") || text.starts_with("Never "),
            "the {name} profile mentions a web tool other than to forbid \
             it: {line}"
        );
    }

    let fetch = format!("accelerator research fetch {family} ");
    let mut in_fence = false;
    let mut invocations = 0;
    for line in body.lines().map(str::trim) {
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence && !line.is_empty() {
            assert!(
                line.starts_with(&fetch),
                "the {name} profile runs something other than the fetch: \
                 {line}"
            );
            invocations += 1;
        }
    }
    assert!(invocations > 0, "the {name} profile shows no fetch to run");
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
