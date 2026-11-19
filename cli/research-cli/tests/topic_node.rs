//! `accelerator-research topic node` black-box CLI coverage: one offered
//! node's context, read from its run's ledger.

use std::fs;
use std::process::Output;

use serde_json::json;
use serde_json::Value;

mod topic_project;

use topic_project::assert_exits_1;
use topic_project::budgeting;
use topic_project::continued;
use topic_project::node_query_set;
use topic_project::started;
use topic_project::tree;
use topic_project::Project;
use topic_project::TestError;
use topic_project::OPENALEX_STEM;
use topic_project::TOPICS;
use topic_project::WEB_STEM;

const DEPTH: [&str; 2] = ["--depth", "3"];

fn answered(output: &Output) -> Result<Value, TestError> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn queried(project: &Project, run: &str, at: &str) -> Result<Value, TestError> {
    answered(&project.node(run, at)?)
}

fn note_at(project: &Project, stem: &str, lineage: &str) -> String {
    project
        .set("s")
        .join(format!("findings/{stem}.levels/{lineage}.md"))
        .to_string_lossy()
        .into_owned()
}

fn budgeted_run(project: &Project) -> Result<String, TestError> {
    let (_, run) = started(project, &DEPTH)?;
    budgeting(project, &[8, 5, 3])?;
    Ok(run)
}

fn not_offered(at: &str, run: &str) -> String {
    format!(
        "E_TOPIC_RESEARCH_NODE_NOT_OFFERED: node {at} is not in run {run}'s \
         offered batch; re-run conduct"
    )
}

fn malformed_node(text: &str) -> String {
    format!(
        "E_TOPIC_RESEARCH_NODE: --node must be a node's spawn ref, \
         <stem>:<lineage>, got '{text}'"
    )
}

fn no_usable_ledger(run: &str) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN_LEDGER: no usable ledger for run {run}; re-run \
         conduct to start a fresh run"
    )
}

fn superseded(run: &str, owner: &str) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN_SUPERSEDED: run {run} was superseded by run \
         {owner}; another conduct run now owns this set, so let it finish"
    )
}

const WEB_KNOWN: [&str; 4] = ["A?", "A1?", "A2?", "A3?"];

#[test]
fn a_web_node_answers_with_its_eight_keys() -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;

    let context = queried(&project, &run, "03-a-web:2-1")?;

    assert_eq!(
        context,
        json!({
            "question": "A1?",
            "lineage": "2-1",
            "level": 2,
            "cap": 2,
            "known_questions": WEB_KNOWN,
            "id": "s.03-a-web.2-1",
            "path": note_at(&project, WEB_STEM, "2-1"),
            "search_budget": 5,
        })
    );
    Ok(())
}

#[test]
fn an_openalex_node_answers_with_seven_keys() -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;

    let context = queried(&project, &run, "04-b-openalex:2-1")?;

    assert_eq!(
        context,
        json!({
            "question": "B1?",
            "lineage": "2-1",
            "level": 2,
            "cap": 2,
            "known_questions": ["B?", "B1?", "B2?", "B3?"],
            "id": "s.04-b-openalex.2-1",
            "path": note_at(&project, OPENALEX_STEM, "2-1"),
        })
    );
    Ok(())
}

fn plan_node<'a>(plan: &'a Value, spawn: &str) -> Result<&'a Value, TestError> {
    plan["pairs"]
        .as_array()
        .ok_or("no pairs")?
        .iter()
        .filter_map(|pair| pair["nodes"].as_array())
        .flatten()
        .find(|node| node["spawn"] == json!(spawn))
        .ok_or_else(|| format!("no node {spawn}").into())
}

#[test]
fn a_web_nodes_budget_equals_its_plan_entry() -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;

    let plan = continued(&project, &run, &DEPTH)?;

    assert_eq!(plan_node(&plan, "03-a-web:2-1")?["search_budget"], json!(5));
    assert_eq!(
        queried(&project, &run, "03-a-web:2-1")?["search_budget"],
        json!(5)
    );
    Ok(())
}

#[test]
fn a_set_named_by_its_directory_answers_with_the_same_id_and_path(
) -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;
    let by_slug = queried(&project, &run, "03-a-web:2-1")?;
    let set_dir = project.set("s");
    let outline = set_dir.join("outline.md");

    for named in [set_dir, outline] {
        let named = named.to_str().ok_or("non-utf8")?;
        let output = project
            .run("node", &[named, "--run", &run, "--node", "03-a-web:2-1"])?;
        let context = answered(&output)?;
        assert_eq!(context["id"], json!("s.03-a-web.2-1"), "{named}");
        assert_eq!(context["path"], by_slug["path"], "{named}");
    }
    Ok(())
}

#[test]
fn a_sibling_note_written_mid_batch_never_changes_known_questions(
) -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;

    project.write_note(WEB_STEM, "2-1", "A1?", &["A4?"])?;

    assert_eq!(
        queried(&project, &run, "03-a-web:2-2")?["known_questions"],
        json!(WEB_KNOWN)
    );
    Ok(())
}

#[test]
fn a_repeat_plan_keeps_pending_nodes_first_context() -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;
    project.write_note(WEB_STEM, "2-1", "A1?", &["A4?"])?;

    continued(&project, &run, &DEPTH)?;

    for at in ["03-a-web:2-2", "03-a-web:2-3"] {
        assert_eq!(
            queried(&project, &run, at)?["known_questions"],
            json!(WEB_KNOWN),
            "{at}"
        );
    }
    assert_exits_1(
        &project.node(&run, "03-a-web:2-1")?,
        &not_offered("03-a-web:2-1", &run),
    );
    Ok(())
}

fn every_level_two_note_written(project: &Project) -> Result<(), TestError> {
    project.write_note(WEB_STEM, "2-1", "A1?", &["A4?"])?;
    project.write_note(WEB_STEM, "2-2", "A2?", &[])?;
    project.write_note(WEB_STEM, "2-3", "A3?", &[])?;
    for (lineage, question) in [("2-1", "B1?"), ("2-2", "B2?"), ("2-3", "B3?")]
    {
        project.write_profiled_note(
            "openalex",
            OPENALEX_STEM,
            lineage,
            question,
            &[],
        )?;
    }
    Ok(())
}

#[test]
fn a_node_of_the_next_level_knows_its_parents_follow_ups(
) -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;
    every_level_two_note_written(&project)?;

    continued(&project, &run, &[&DEPTH[..], &["--spawned", "0"]].concat())?;

    let context = queried(&project, &run, "03-a-web:3-1-1")?;
    assert_eq!(context["question"], json!("A4?"));
    assert_eq!(
        context["known_questions"],
        json!(["A?", "A1?", "A2?", "A3?", "A4?"])
    );
    assert_eq!(context["search_budget"], json!(3));
    Ok(())
}

#[test]
fn a_node_offered_only_in_an_earlier_batch_is_refused() -> Result<(), TestError>
{
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;
    every_level_two_note_written(&project)?;

    let plan =
        continued(&project, &run, &[&DEPTH[..], &["--spawned", "0"]].concat())?;

    assert_exits_1(
        &project.node(&run, "03-a-web:2-1")?,
        &not_offered("03-a-web:2-1", &run),
    );
    let offered: Vec<&Value> = plan["pairs"]
        .as_array()
        .ok_or("no pairs")?
        .iter()
        .filter_map(|pair| pair["nodes"].as_array())
        .flatten()
        .collect();
    assert_eq!(offered.len(), 1);
    for node in offered {
        let spawn = node["spawn"].as_str().ok_or("no spawn")?;
        queried(&project, &run, spawn)?;
    }
    Ok(())
}

#[test]
fn a_re_issued_spawn_answers_as_its_first_issue() -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;

    let first = project.node(&run, "03-a-web:2-2")?;
    let second = project.node(&run, "03-a-web:2-2")?;

    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    Ok(())
}

#[test]
fn a_web_node_withheld_after_a_halt_is_refused() -> Result<(), TestError> {
    let project = node_query_set()?;
    let one_at_a_time = ["--depth", "3", "--concurrency", "1"];
    let (first, run) = started(&project, &one_at_a_time)?;
    assert_eq!(
        plan_node(&first, "03-a-web:2-1")?["spawn"],
        json!("03-a-web:2-1")
    );
    queried(&project, &run, "03-a-web:2-1")?;

    continued(
        &project,
        &run,
        &[
            &one_at_a_time[..],
            &["--spawned", "0", "--withhold-profile", "web"],
        ]
        .concat(),
    )?;

    for at in ["03-a-web:2-2", "03-a-web:2-1"] {
        assert_exits_1(&project.node(&run, at)?, &not_offered(at, &run));
    }
    Ok(())
}

#[test]
fn a_pair_spawn_ref_is_refused() -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;

    assert_exits_1(
        &project.node(&run, "03-a-web")?,
        &malformed_node("03-a-web"),
    );
    Ok(())
}

fn malformed_run(text: &str) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN: --run must be 1 to 64 letters, digits, '_' \
         or '-', got '{text}'"
    )
}

#[test]
fn a_malformed_run_is_refused() -> Result<(), TestError> {
    let project = node_query_set()?;
    budgeted_run(&project)?;

    assert_exits_1(
        &project.node("bad run!", "03-a-web:2-1")?,
        &malformed_run("bad run!"),
    );
    Ok(())
}

#[test]
fn a_malformed_run_is_refused_before_a_malformed_node() -> Result<(), TestError>
{
    let project = node_query_set()?;
    budgeted_run(&project)?;

    assert_exits_1(
        &project.node("bad run!", "03-a-web")?,
        &malformed_run("bad run!"),
    );
    Ok(())
}

#[test]
fn the_query_writes_nothing() -> Result<(), TestError> {
    let project = node_query_set()?;
    let run = budgeted_run(&project)?;
    let before = tree(&project.root.join(TOPICS))?;

    queried(&project, &run, "03-a-web:2-1")?;
    let _ = project.node(&run, "03-a-web:2-9")?;

    assert_eq!(tree(&project.root.join(TOPICS))?, before);
    Ok(())
}

fn assert_refused_with_prefix(output: &Output, prefix: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.starts_with(prefix), "{stderr}");
}

#[test]
fn the_query_refuses_what_it_cannot_answer() -> Result<(), TestError> {
    use std::os::unix::fs::PermissionsExt as _;

    let project = node_query_set()?;
    let at = "03-a-web:2-1";

    assert_exits_1(&project.node("r1", at)?, &no_usable_ledger("r1"));

    for slug in ["nope", "../..", "/tmp"] {
        assert_exits_1(
            &project.run("node", &[slug, "--run", "r1", "--node", at])?,
            &format!(
                "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set '{slug}'"
            ),
        );
    }

    let (_, first) = started(&project, &DEPTH)?;
    assert_exits_1(&project.node("r1", at)?, &superseded("r1", &first));
    assert_exits_1(
        &project.node(&first, "03-a-web:2-9")?,
        &not_offered("03-a-web:2-9", &first),
    );
    for malformed in ["2-1", "03-a-web:x", ""] {
        assert_exits_1(
            &project.node(&first, malformed)?,
            &malformed_node(malformed),
        );
    }

    let (_, second) = started(&project, &DEPTH)?;
    assert_exits_1(&project.node(&first, at)?, &superseded(&first, &second));

    fs::write(project.ledger(), "{")?;
    assert_exits_1(&project.node(&second, at)?, &no_usable_ledger(&second));

    fs::set_permissions(project.ledger(), fs::Permissions::from_mode(0o000))?;
    assert_refused_with_prefix(
        &project.node(&second, at)?,
        &format!(
            "E_TOPIC_RESEARCH_RUN_LEDGER: could not read {}: ",
            project.ledger().display()
        ),
    );
    fs::set_permissions(project.ledger(), fs::Permissions::from_mode(0o644))?;

    let (_, third) = started(&project, &DEPTH)?;
    project.run("end-run", &["s", "--run", &third])?;
    assert_exits_1(&project.node(&third, at)?, &no_usable_ledger(&third));
    Ok(())
}
