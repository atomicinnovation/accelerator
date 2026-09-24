#![allow(clippy::expect_used, clippy::panic)]

use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use serde_json::json;
use serde_json::Value;

const BINARY: &str = env!("CARGO_BIN_EXE_accelerator-research");
const RESEARCHER: &str = "accelerator:researcher";
const FETCH: &str = "accelerator research fetch arxiv search 'attention heads'";

struct Project {
    work: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let work = tempfile::Builder::new()
            .prefix("research-guard-")
            .tempdir()
            .expect("tempdir");
        let root = work.path().join("repo");
        std::fs::create_dir_all(root.join(".accelerator"))
            .expect("mkdir .accelerator");
        std::fs::create_dir_all(root.join("meta/research/topics"))
            .expect("mkdir topics");
        Self { work, root }
    }

    fn config(self, body: &str) -> Self {
        std::fs::write(self.root.join(".accelerator/config.md"), body)
            .expect("write config.md");
        self
    }

    fn unparseable_config(self) -> Self {
        self.config("---\npaths: [unclosed\n---\n")
    }

    fn mkdir(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(&path).expect("mkdir");
        path
    }

    fn at(&self, relative: &str) -> String {
        self.root.join(relative).display().to_string()
    }
}

struct Verdict {
    code: Option<i32>,
    stderr: String,
}

impl Verdict {
    fn passed(&self) -> bool {
        self.code == Some(0)
    }

    fn assert_passes(&self) {
        assert_eq!(self.code, Some(0), "expected a pass: {}", self.stderr);
    }

    fn assert_passes_silently(&self) {
        self.assert_passes();
        assert_eq!(self.stderr, "", "expected no diagnostic");
    }

    fn assert_blocks(&self, reason: &str) {
        assert_eq!(self.code, Some(2), "expected a block: {}", self.stderr);
        assert!(
            self.stderr.contains(reason),
            "expected {reason:?} in {}",
            self.stderr
        );
    }
}

fn guard_with(cwd: &Path, args: &[&str], stdin: &str) -> Verdict {
    let mut child = Command::new(BINARY)
        .arg("guard")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the guard");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait for the guard");
    Verdict {
        code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn guard(cwd: &Path, payload: &Value) -> Verdict {
    guard_with(
        cwd,
        &["--fail-safe", "--non-blocking"],
        &payload.to_string(),
    )
}

fn call(
    cwd: &Path,
    agent_type: Option<&str>,
    tool_name: &str,
    tool_input: &Value,
) -> Value {
    let mut payload = json!({
        "session_id": "s1",
        "hook_event_name": "PreToolUse",
        "cwd": cwd,
        "tool_name": tool_name,
        "tool_input": tool_input,
    });
    if let Some(agent_type) = agent_type {
        payload["agent_id"] = json!("agent-1");
        payload["agent_type"] = json!(agent_type);
    }
    payload
}

fn bash(project: &Project, agent_type: Option<&str>, command: &str) -> Verdict {
    guard(
        &project.root,
        &call(
            &project.root,
            agent_type,
            "Bash",
            &json!({ "command": command }),
        ),
    )
}

fn write(project: &Project, agent_type: Option<&str>, path: &str) -> Verdict {
    guard(
        &project.root,
        &call(
            &project.root,
            agent_type,
            "Write",
            &json!({ "file_path": path, "content": "x" }),
        ),
    )
}

const FINDING: &str = "meta/research/topics/s/findings/01-x-web.md";

#[test]
fn the_researcher_may_run_the_fetch() {
    bash(&Project::new(), Some(RESEARCHER), FETCH).assert_passes_silently();
}

#[test]
fn the_researcher_may_run_nothing_else() {
    bash(&Project::new(), Some(RESEARCHER), "ls").assert_blocks(
        "E_RESEARCH_GUARD_COMMAND: accelerator:researcher may run only \
         'accelerator research fetch …'",
    );
}

#[test]
fn the_researcher_may_not_chain_onto_the_fetch() {
    bash(&Project::new(), Some(RESEARCHER), &format!("{FETCH}; ls"))
        .assert_blocks("E_RESEARCH_GUARD_SYNTAX: command contains a ';'");
}

#[test]
fn the_researcher_may_write_a_finding() {
    let project = Project::new();
    project.mkdir("meta/research/topics/s/findings");
    write(&project, Some(RESEARCHER), &project.at(FINDING))
        .assert_passes_silently();
}

#[test]
fn the_researcher_may_not_write_config() {
    let project = Project::new();
    write(
        &project,
        Some(RESEARCHER),
        &project.at(".accelerator/config.md"),
    )
    .assert_blocks("E_RESEARCH_GUARD_WRITE: accelerator:researcher");
}

#[test]
fn a_dot_dot_escape_from_findings_is_blocked() {
    let project = Project::new();
    project.mkdir("meta/research/topics/s/findings/new");
    write(
        &project,
        Some(RESEARCHER),
        &project.at("meta/research/topics/s/findings/new/../../../../\
             .accelerator/config.md"),
    )
    .assert_blocks("a '.' or '..' component");
}

#[test]
fn a_relative_path_resolves_against_the_hook_cwd() {
    let project = Project::new();
    project.mkdir("meta/research/topics/s/findings");
    write(&project, Some(RESEARCHER), FINDING).assert_passes_silently();
    write(
        &project,
        Some(RESEARCHER),
        "meta/research/topics/s/brief.md",
    )
    .assert_blocks("'s/brief.md' is not a finding");
}

#[test]
fn the_first_write_before_findings_exists_is_allowed() {
    let project = Project::new();
    write(&project, Some(RESEARCHER), &project.at(FINDING))
        .assert_passes_silently();
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_at_the_target_is_blocked() {
    let project = Project::new();
    let findings = project.mkdir("meta/research/topics/s/findings");
    std::os::unix::fs::symlink(
        project.root.join(".accelerator/config.md"),
        findings.join("01-x-web.md"),
    )
    .expect("symlink");
    write(&project, Some(RESEARCHER), &project.at(FINDING))
        .assert_blocks("a symlink at '01-x-web.md'");
}

#[cfg(unix)]
#[test]
fn a_symlinked_findings_directory_is_blocked() {
    let project = Project::new();
    project.mkdir("meta/research/topics/s");
    std::os::unix::fs::symlink(
        project.root.join(".accelerator"),
        project.root.join("meta/research/topics/s/findings"),
    )
    .expect("symlink");
    write(&project, Some(RESEARCHER), &project.at(FINDING))
        .assert_blocks("a symlink at 'findings'");
}

#[cfg(unix)]
#[test]
fn a_symlinked_ancestor_of_the_project_is_followed() {
    let project = Project::new();
    project.mkdir("meta/research/topics/s/findings");
    let linked = project.work.path().join("linked");
    std::os::unix::fs::symlink(&project.root, &linked).expect("symlink");
    let finding = linked.join(FINDING).display().to_string();
    guard(
        &linked,
        &call(
            &linked,
            Some(RESEARCHER),
            "Write",
            &json!({ "file_path": finding, "content": "x" }),
        ),
    )
    .assert_passes_silently();
}

#[test]
fn a_configured_topics_directory_replaces_the_default() {
    let project = Project::new()
        .config("---\npaths:\n  research_topics: docs/topics\n---\n");
    project.mkdir("docs/topics/s/findings");
    write(&project, Some(RESEARCHER), "docs/topics/s/findings/x.md")
        .assert_passes_silently();
    write(&project, Some(RESEARCHER), FINDING).assert_blocks("not under");
}

#[test]
fn notebook_and_edit_writes_are_judged_by_their_paths() {
    let project = Project::new();
    let config = project.at(".accelerator/config.md");
    guard(
        &project.root,
        &call(
            &project.root,
            Some(RESEARCHER),
            "NotebookEdit",
            &json!({ "notebook_path": config, "new_source": "x" }),
        ),
    )
    .assert_blocks("E_RESEARCH_GUARD_WRITE");
    guard(
        &project.root,
        &call(
            &project.root,
            Some(RESEARCHER),
            "Edit",
            &json!({ "file_path": config, "old_string": "a", "new_string": "b" }),
        ),
    )
    .assert_blocks("E_RESEARCH_GUARD_WRITE");
}

#[test]
fn a_researcher_write_without_a_path_is_unreadable() {
    let project = Project::new();
    guard(
        &project.root,
        &call(
            &project.root,
            Some(RESEARCHER),
            "Write",
            &json!({ "content": "x" }),
        ),
    )
    .assert_blocks("E_RESEARCH_GUARD_UNREADABLE: accelerator:researcher");
}

#[test]
fn every_call_not_from_a_researcher_passes() {
    let project = Project::new();
    for agent_type in [None, Some(""), Some("accelerator:reviewer")] {
        bash(&project, agent_type, "ls").assert_passes_silently();
    }
    let mut empty_id = call(
        &project.root,
        Some(RESEARCHER),
        "Bash",
        &json!({ "command": "ls" }),
    );
    empty_id["agent_id"] = json!("");
    guard(&project.root, &empty_id).assert_passes_silently();
}

#[test]
fn another_subagent_may_write_through_dot_dot() {
    let project = Project::new();
    write(
        &project,
        Some("accelerator:reviewer"),
        &project.at("meta/../.accelerator/config.md"),
    )
    .assert_passes_silently();
}

#[test]
fn a_main_thread_write_never_reads_the_config() {
    let project = Project::new().unparseable_config();
    write(&project, None, &project.at(".accelerator/config.md"))
        .assert_passes_silently();
}

#[test]
fn a_configured_researcher_is_confined_beside_the_default() {
    let project = Project::new()
        .config("---\nagents:\n  researcher: custom:researcher\n---\n");
    let custom = bash(&project, Some("custom:researcher"), "ls");
    custom.assert_blocks("custom:researcher (agents.researcher) may run only");
    let default = bash(&project, Some(RESEARCHER), "ls");
    default.assert_blocks("accelerator:researcher may run only");
    assert!(!default.stderr.contains("agents.researcher"));
}

#[test]
fn unreadable_input_passes_with_a_diagnostic() {
    let project = Project::new();
    for stdin in ["{not json", ""] {
        let verdict = guard_with(&project.root, &[], stdin);
        verdict.assert_passes();
        assert!(!verdict.stderr.is_empty(), "{stdin:?}: no diagnostic");
    }
}

fn with_lone_surrogate(
    tool_name: &str,
    field: &str,
    project: &Project,
) -> String {
    let finding = project.at(FINDING);
    let tool_input = match field {
        "command" => {
            r#"{"command":"accelerator research fetch arxiv search 'a\ud800'"}"#
                .to_owned()
        }
        _ => format!(r#"{{"file_path":"{finding}","content":"a\ud800"}}"#),
    };
    format!(
        r#"{{"cwd":"{}","agent_id":"agent-1","agent_type":"{RESEARCHER}","tool_name":"{tool_name}","tool_input":{tool_input}}}"#,
        project.root.display()
    )
}

#[test]
fn a_lone_surrogate_inside_the_tool_input_blocks_the_researcher() {
    let project = Project::new();
    project.mkdir("meta/research/topics/s/findings");
    for (tool_name, field) in [("Bash", "command"), ("Write", "content")] {
        guard_with(
            &project.root,
            &[],
            &with_lone_surrogate(tool_name, field, &project),
        )
        .assert_blocks("E_RESEARCH_GUARD_UNREADABLE");
    }
}

#[test]
fn an_unparseable_config_still_confines_the_default_researcher() {
    let project = Project::new().unparseable_config();
    bash(&project, Some(RESEARCHER), "ls")
        .assert_blocks("E_RESEARCH_GUARD_COMMAND");
}

#[test]
fn an_unparseable_config_falls_back_to_the_default_topics() {
    let project = Project::new().unparseable_config();
    project.mkdir("meta/research/topics/s/findings");
    let verdict = write(&project, Some(RESEARCHER), FINDING);
    verdict.assert_passes();
    assert!(
        verdict.stderr.contains("could not read the configuration"),
        "{}",
        verdict.stderr
    );
}

#[test]
fn the_launcher_flags_and_unknown_flags_never_block() {
    let project = Project::new();
    let payload = call(
        &project.root,
        Some("accelerator:reviewer"),
        "Bash",
        &json!({ "command": "ls" }),
    )
    .to_string();
    assert!(guard_with(
        &project.root,
        &["--fail-safe", "--non-blocking"],
        &payload
    )
    .passed());
    let unknown = guard_with(&project.root, &["--bogus"], &payload);
    unknown.assert_passes();
    assert!(!unknown.stderr.is_empty(), "an unknown flag is reported");
}

#[cfg(feature = "test-loopback")]
#[test]
fn a_guard_failure_blocks_only_a_confined_call() {
    let project = Project::new();
    let run = |agent_type: &str| {
        let mut child = Command::new(BINARY)
            .arg("guard")
            .current_dir(&project.root)
            .env("ACCELERATOR_RESEARCH_TEST_GUARD_PANIC", "1")
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the guard");
        let payload = call(
            &project.root,
            Some(agent_type),
            "Bash",
            &json!({ "command": "ls" }),
        );
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(payload.to_string().as_bytes())
            .expect("write stdin");
        let output = child.wait_with_output().expect("wait");
        Verdict {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    };
    run(RESEARCHER).assert_blocks(
        "E_RESEARCH_GUARD_INTERNAL: accelerator:researcher tool call blocked",
    );
    run("accelerator:reviewer").assert_passes();
}
