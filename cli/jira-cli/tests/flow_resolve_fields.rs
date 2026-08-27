//! The `resolve-fields` contract: the tab-separated four-field line, the kind →
//! issue-type map, the flag/config/id project precedence, and the already-synced
//! guard. Config-only — no client, no mock, no seam.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use std::path::Path;

use support::Token;

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    support::run_with(dir, args, None, &Token::Present)
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf8 stdout")
}

#[test]
fn a_flag_project_wins_and_a_known_kind_maps() {
    let dir = support::scratch(support::CONFIG);
    let output = run(
        dir.path(),
        &["resolve-fields", "--kind", "bug", "--project", "FOO"],
    );
    assert!(output.status.success(), "exited {:?}", output.status.code());
    assert_eq!(stdout_of(&output), "Bug\tmapped\tFOO\tflag\n");
}

#[test]
fn the_configured_default_project_is_the_config_source() {
    let dir = support::scratch(support::CONFIG);
    let output = run(dir.path(), &["resolve-fields", "--kind", "task"]);
    assert!(output.status.success(), "exited {:?}", output.status.code());
    assert_eq!(stdout_of(&output), "Task\tmapped\tENG\tconfig\n");
}

#[test]
fn the_project_derives_from_the_shared_config_source() {
    // `jira resolve-fields` and `work create --push --dry-run` do
    // NOT agree field-for-field — the work form emits the raw kind with a
    // network-preview `configured`/`unresolvable` source vocabulary, while this
    // config-only form emits the mapped Jira issue-type with a `mapped`/`config`
    // vocabulary. The one field they genuinely share is the project value, both
    // read from `work.default_project_code`; this pins that shared derivation.
    let dir = support::scratch(support::CONFIG);
    let output = run(dir.path(), &["resolve-fields", "--kind", "bug"]);
    assert!(output.status.success(), "exited {:?}", output.status.code());
    let fields: Vec<String> = stdout_of(&output)
        .trim_end()
        .split('\t')
        .map(str::to_owned)
        .collect();
    // project value == work.default_project_code, from the config source.
    assert_eq!(fields[2], "ENG", "the project is the configured default");
    assert_eq!(fields[3], "config", "read from the shared config source");
    // The type is the mapped Jira issue-type name, not the raw kind the work
    // form emits — the deliberate divergence.
    assert_eq!(fields[0], "Bug", "the mapped issue-type, not the raw kind");
    assert_eq!(fields[1], "mapped");
}

#[test]
fn an_unknown_kind_defaults_to_task() {
    let dir = support::scratch(support::CONFIG);
    let output = run(
        dir.path(),
        &["resolve-fields", "--kind", "chore", "--project", "P"],
    );
    assert_eq!(stdout_of(&output), "Task\tdefault\tP\tflag\n");
}

#[test]
fn a_project_coded_id_is_the_id_source_when_no_config() {
    // A config with no default project, so the id supplies the project.
    let config = "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
        email: toby@example.com\n---\n";
    let dir = support::scratch(config);
    let output = run(
        dir.path(),
        &["resolve-fields", "--kind", "story", "--id", "PROJ-42"],
    );
    assert_eq!(stdout_of(&output), "Story\tmapped\tPROJ\tid\n");
}

#[test]
fn an_unresolvable_project_exits_108() {
    let config = "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
        email: toby@example.com\n---\n";
    let dir = support::scratch(config);
    let output = run(dir.path(), &["resolve-fields", "--kind", "bug"]);
    assert_eq!(output.status.code(), Some(108));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("E_RESOLVE_NO_PROJECT"));
}

#[test]
fn an_already_synced_file_exits_109() {
    let dir = support::scratch(support::CONFIG);
    let item = dir.path().join("wi.md");
    std::fs::write(
        &item,
        "---\nkind: bug\nexternal_id: \"PROJ-7\"\n---\nbody\n",
    )
    .unwrap();
    let output = run(
        dir.path(),
        &["resolve-fields", "--file", item.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(109));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("E_RESOLVE_ALREADY_SYNCED"));
}

#[test]
fn a_file_reads_its_kind_from_frontmatter() {
    let dir = support::scratch(support::CONFIG);
    let item = dir.path().join("wi.md");
    std::fs::write(&item, "---\nkind: epic\nid: WI-1\n---\nbody\n").unwrap();
    let output = run(
        dir.path(),
        &["resolve-fields", "--file", item.to_str().unwrap()],
    );
    assert!(output.status.success(), "exited {:?}", output.status.code());
    assert_eq!(stdout_of(&output), "Epic\tmapped\tENG\tconfig\n");
}
