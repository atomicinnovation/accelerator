use std::error::Error;
use std::path::Path;

use research::confinement::command_decision;
use research::confinement::confined_researcher;
use research::confinement::decide;
use research::confinement::Action;
use research::confinement::Block;
use research::confinement::Construct;
use research::confinement::Decision;
use research::confinement::FindingsScope;
use research::confinement::InternalFailure;
use research::confinement::PathRejection;
use research::confinement::Refusal;
use research::confinement::Researcher;
use research::confinement::Researchers;
use research::confinement::ToolCall;
use research::confinement::TopicsRelativePath;
use research::confinement::WriteRefusal;
use research::confinement::PERMITTED_PREFIX;
use research::confinement::STRICTER_THAN_MEASURED;

const MEASURED_PREFIX: &str = "./probe";

struct Fixture {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn fixture(name: &str) -> Result<Fixture, Box<dyn Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines().filter(|line| !line.starts_with('#'));
    let columns = lines
        .next()
        .map(|header| header.split('\t').map(str::to_owned).collect())
        .unwrap_or_default();
    let rows = lines
        .map(|line| line.split('\t').map(unescape).collect())
        .collect();
    Ok(Fixture { columns, rows })
}

fn unescape(field: &str) -> String {
    field
        .replace('⏎', "\n")
        .replace('⇥', "\t")
        .replace('␍', "\r")
        .replace('⍽', "\u{a0}")
        .replace('␋', "\u{b}")
        .replace('␌', "\u{c}")
}

fn release(column: &str) -> Vec<u32> {
    column
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect()
}

fn command(text: &str) -> Decision {
    decide(&Action::Command(text.to_owned()), &NoFindings)
}

struct NoFindings;

impl FindingsScope for NoFindings {
    fn contains(&self, _target: &TopicsRelativePath) -> bool {
        false
    }
}

struct EveryPath;

impl FindingsScope for EveryPath {
    fn contains(&self, _target: &TopicsRelativePath) -> bool {
        true
    }
}

#[test]
fn the_guard_agrees_with_the_latest_measured_claude_code_release(
) -> Result<(), Box<dyn Error>> {
    let baseline = fixture("claude-bash-baseline.tsv")?;
    let latest = (1..baseline.columns.len())
        .max_by_key(|&index| release(&baseline.columns[index]))
        .ok_or("the baseline measures no release")?;
    assert!(baseline.rows.len() > 50, "the baseline lost its rows");
    let disagreements: Vec<String> = baseline
        .rows
        .iter()
        .filter_map(|row| {
            let measured = &row[0];
            let judged = measured.replacen(
                MEASURED_PREFIX,
                PERMITTED_PREFIX.trim_end(),
                1,
            );
            let verdict = command(&judged);
            let agrees = match (row[latest].as_str(), &verdict) {
                ("allowed", Decision::Pass)
                | ("denied", Decision::Block(_)) => true,
                ("allowed", Decision::Block(Block::Syntax(construct))) => {
                    STRICTER_THAN_MEASURED.contains(construct)
                }
                _ => false,
            };
            (!agrees).then(|| {
                format!(
                    "{measured:?}: measured {}, guard {verdict:?}",
                    row[latest]
                )
            })
        })
        .collect();
    assert!(
        disagreements.is_empty(),
        "the guard disagrees with Claude Code {}:\n{}",
        baseline.columns[latest],
        disagreements.join("\n")
    );
    Ok(())
}

#[test]
fn every_lexer_boundary_row_gets_its_verdict() -> Result<(), Box<dyn Error>> {
    let boundary = fixture("guard-boundary.tsv")?;
    assert!(boundary.rows.len() > 50, "the boundary rows went missing");
    for row in &boundary.rows {
        let verdict = command(&row[0]);
        let passed = matches!(verdict, Decision::Pass);
        assert_eq!(
            passed,
            row[1] == "pass",
            "{:?} expected {}, got {verdict:?}",
            row[0],
            row[1]
        );
    }
    Ok(())
}

#[test]
fn the_stricter_constructs_are_a_dollar_expansion_and_input_redirection() {
    assert_eq!(
        STRICTER_THAN_MEASURED,
        [Construct::Expansion, Construct::InputRedirection]
    );
    assert_eq!(
        command("accelerator research fetch $HOME"),
        Decision::Block(Block::Syntax(Construct::Expansion))
    );
    assert_eq!(
        command("accelerator research fetch </dev/tcp/example.org/80"),
        Decision::Block(Block::Syntax(Construct::InputRedirection))
    );
}

#[test]
fn a_command_without_the_prefix_is_the_wrong_command() {
    for text in [
        "ls",
        "accelerator research fetch",
        "accelerator research fetchx a",
        "FOO=1 accelerator research fetch a",
        "accelerator research guard",
    ] {
        assert_eq!(command(text), Decision::Block(Block::WrongCommand));
    }
}

#[test]
fn surrounding_whitespace_is_ignored() {
    assert_eq!(
        command("  accelerator research fetch arxiv search 'x'\n"),
        Decision::Pass
    );
}

#[test]
fn each_construct_is_named_by_what_blocked_it() {
    let cases = [
        ("a; b", Construct::Separator),
        ("a | b", Construct::Pipe),
        ("a & b", Construct::Ampersand),
        ("a\nb", Construct::Newline),
        ("a\\\nb", Construct::LineContinuation),
        ("$HOME", Construct::Expansion),
        ("`ls`", Construct::Backtick),
        ("=(ls)", Construct::ProcessSubstitution),
        ("(a)", Construct::Parenthesis),
        ("{a,b}", Construct::Brace),
        ("< a", Construct::InputRedirection),
        ("> a", Construct::OutputRedirection),
        ("&> a", Construct::OutputRedirection),
    ];
    for (rest, construct) in cases {
        assert_eq!(
            command_decision(&format!("{PERMITTED_PREFIX}{rest}")),
            Decision::Block(Block::Syntax(construct)),
            "{rest:?}"
        );
    }
}

#[test]
fn a_write_inside_the_findings_scope_passes() {
    let target = TopicsRelativePath::new("s/findings/01-x-web.md".to_owned());
    assert_eq!(
        decide(&Action::Write(Ok(target)), &EveryPath),
        Decision::Pass
    );
}

#[test]
fn a_write_outside_the_findings_scope_is_blocked() {
    let target = TopicsRelativePath::new("s/brief.md".to_owned());
    assert_eq!(
        decide(&Action::Write(Ok(target.clone())), &NoFindings),
        Decision::Block(Block::Write(WriteRefusal::OutsideFindings(target)))
    );
}

#[test]
fn a_rejected_write_path_is_blocked_whatever_the_scope() {
    assert_eq!(
        decide(&Action::Write(Err(PathRejection::DotComponent)), &EveryPath),
        Decision::Block(Block::Write(WriteRefusal::Rejected(
            PathRejection::DotComponent
        )))
    );
}

#[test]
fn an_unreadable_call_is_blocked() {
    assert_eq!(
        decide(&Action::Unreadable, &EveryPath),
        Decision::Block(Block::Unreadable)
    );
}

fn subagent(agent_type: &str) -> ToolCall {
    ToolCall::new(Some("a1".to_owned()), Some(agent_type.to_owned()))
}

#[test]
fn only_a_researcher_subagent_is_confined() {
    let researchers = Researchers::with_configured("custom:researcher");
    assert_eq!(
        confined_researcher(&subagent("accelerator:researcher"), &researchers),
        Some(Researcher::Default)
    );
    assert_eq!(
        confined_researcher(&subagent("custom:researcher"), &researchers),
        Some(Researcher::Configured("custom:researcher".to_owned()))
    );
    assert_eq!(
        confined_researcher(&subagent("accelerator:reviewer"), &researchers),
        None
    );
    assert_eq!(confined_researcher(&subagent(""), &researchers), None);
}

#[test]
fn a_call_without_an_agent_id_is_the_main_thread() {
    let researchers = Researchers::default_only();
    for agent_id in [None, Some(String::new())] {
        let call =
            ToolCall::new(agent_id, Some("accelerator:researcher".to_owned()));
        assert!(!call.is_subagent());
        assert_eq!(confined_researcher(&call, &researchers), None);
    }
}

#[test]
fn a_configured_default_name_matches_as_the_default() {
    let researchers = Researchers::with_configured("accelerator:researcher");
    assert_eq!(
        confined_researcher(&subagent("accelerator:researcher"), &researchers),
        Some(Researcher::Default)
    );
}

fn refusal(researcher: &Researcher, block: &Block) -> String {
    Refusal {
        researcher,
        topics: "/p/meta/research/topics",
        block,
    }
    .to_string()
}

#[test]
fn refusals_carry_a_coded_cause_specific_reason() {
    let default = Researcher::Default;
    assert_eq!(
        refusal(&default, &Block::WrongCommand),
        "E_RESEARCH_GUARD_COMMAND: accelerator:researcher may run only \
         'accelerator research fetch …'"
    );
    assert_eq!(
        refusal(&default, &Block::Syntax(Construct::Separator)),
        "E_RESEARCH_GUARD_SYNTAX: command contains a ';' separator — pass \
         the query as one single-quoted argument"
    );
    assert_eq!(
        refusal(&default, &Block::Unreadable),
        "E_RESEARCH_GUARD_UNREADABLE: accelerator:researcher tool call has \
         no readable command or path"
    );
}

#[test]
fn a_configured_researcher_is_named_with_its_key() {
    let configured = Researcher::Configured("custom:researcher".to_owned());
    assert_eq!(
        refusal(&configured, &Block::WrongCommand),
        "E_RESEARCH_GUARD_COMMAND: custom:researcher (agents.researcher) may \
         run only 'accelerator research fetch …'"
    );
}

#[test]
fn every_write_refusal_names_its_cause() {
    let prefix = "E_RESEARCH_GUARD_WRITE: accelerator:researcher may write \
                  only /p/meta/research/topics/<set>/findings/<name>.md — ";
    let cases = [
        (
            WriteRefusal::OutsideFindings(TopicsRelativePath::new(
                "s/brief.md".to_owned(),
            )),
            "'s/brief.md' is not a finding",
        ),
        (
            WriteRefusal::Rejected(PathRejection::NotUnderTopics),
            "not under /p/meta/research/topics",
        ),
        (
            WriteRefusal::Rejected(PathRejection::DotComponent),
            "a '.' or '..' component",
        ),
        (
            WriteRefusal::Rejected(PathRejection::Symlink(
                "findings".to_owned(),
            )),
            "a symlink at 'findings'",
        ),
        (
            WriteRefusal::Rejected(PathRejection::Uninspectable),
            "a path component that could not be inspected",
        ),
    ];
    for (write, cause) in cases {
        assert_eq!(
            refusal(&Researcher::Default, &Block::Write(write)),
            format!("{prefix}{cause}")
        );
    }
}

#[test]
fn every_construct_describes_itself() {
    let cases = [
        (Construct::Separator, "a ';' separator"),
        (Construct::Pipe, "a '|' pipe"),
        (Construct::Ampersand, "a '&' operator"),
        (Construct::Newline, "a newline"),
        (Construct::LineContinuation, "a backslash-newline"),
        (Construct::Expansion, "a '$' expansion"),
        (Construct::Backtick, "a '`' substitution"),
        (Construct::ProcessSubstitution, "a '=(…)' substitution"),
        (Construct::Parenthesis, "a parenthesis"),
        (Construct::Brace, "a brace"),
        (Construct::InputRedirection, "an input redirection"),
        (
            Construct::OutputRedirection,
            "an output redirection other than to /dev/null",
        ),
    ];
    for (construct, description) in cases {
        assert_eq!(construct.to_string(), description);
    }
}

#[test]
fn an_internal_failure_blocks_with_its_own_code() {
    assert_eq!(
        InternalFailure(&Researcher::Default).to_string(),
        "E_RESEARCH_GUARD_INTERNAL: accelerator:researcher tool call \
         blocked — the guard failed while judging it"
    );
}
