//! Whether `ask` and `report` may consult a committed set, or the first gate
//! it fails.

use std::fs;
use std::process::Output;

use serde_json::json;
use serde_json::Value;

mod topic_project;

use topic_project::tree;
use topic_project::Project;
use topic_project::TestError;
use topic_project::TOPICS;

const SET: &str = "topic-research-multiround-set";
const ARXIV_ITEM: &str = "Which planning preprints appeared this year?";

impl Project {
    fn multiround() -> Result<Self, TestError> {
        Self::with_fixture_set(SET)
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), TestError> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn edit(
        &self,
        document: &str,
        from: &str,
        to: &str,
    ) -> Result<(), TestError> {
        let path = self.set(SET).join(document);
        let text = fs::read_to_string(&path)?;
        assert!(text.contains(from), "{from:?} in {document}");
        fs::write(path, text.replacen(from, to, 1))?;
        Ok(())
    }

    fn append_to_outline(&self, line: &str) -> Result<(), TestError> {
        let path = self.set(SET).join("outline.md");
        let text = fs::read_to_string(&path)?;
        fs::write(path, format!("{text}{line}\n"))?;
        Ok(())
    }

    fn consultable_output(&self, slug: &str) -> Result<Output, TestError> {
        self.run("consultable", &[slug])
    }

    fn consultable(&self) -> Result<Value, TestError> {
        json_of(&self.consultable_output(SET)?)
    }

    fn plan(&self) -> Result<Value, TestError> {
        json_of(&self.run("outstanding", &[SET])?)
    }

    fn counts(&self) -> Result<Value, TestError> {
        json_of(&self.run("counts", &[SET])?)
    }

    fn absolute(&self, relative: &str) -> String {
        self.set(SET).join(relative).to_string_lossy().into_owned()
    }
}

fn json_of(output: &Output) -> Result<Value, TestError> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn consultable_names_a_missing_profiles_directory_before_any_gate(
) -> Result<(), TestError> {
    let project = Project::multiround()?;
    project.edit(
        "manifest.md",
        "status: \"synthesised\"",
        "status: \"researching\"",
    )?;

    let output = project.run_from(&project.root, "consultable", &[SET])?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "no profiles directory at {}",
            project.root.join("skills/research/profiles").display()
        )),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn consultable_without_a_plugin_root_fails_naming_it() -> Result<(), TestError>
{
    let project = Project::multiround()?;

    let output = project.run_without_a_plugin_root("consultable", &[SET])?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ACCELERATOR_PLUGIN_ROOT"), "{stderr}");
    Ok(())
}

#[test]
fn consultable_refuses_a_set_still_researching() -> Result<(), TestError> {
    let project = Project::multiround()?;
    project.edit(
        "manifest.md",
        "status: \"synthesised\"",
        "status: \"researching\"",
    )?;

    let verdict = project.consultable()?;

    assert_eq!(
        verdict,
        json!({
            "consultable": false,
            "gate": "status",
            "status": "researching",
        })
    );
    Ok(())
}

#[test]
fn consultable_refuses_a_stale_set_before_planning() -> Result<(), TestError> {
    let project = Project::multiround()?;
    project.edit("manifest.md", "finding_count: 3", "finding_count: 9")?;

    let verdict = project.consultable()?;

    assert_eq!(
        verdict,
        json!({
            "consultable": false,
            "gate": "freshness",
            "mismatches": [
                "manifest.md records finding_count 9 but 3 findings are on disk"
            ],
        })
    );
    Ok(())
}

#[test]
fn consultable_fails_when_an_open_set_s_plan_cannot_be_read(
) -> Result<(), TestError> {
    let project = Project::multiround()?;
    fs::write(project.set(SET).join("outline.md"), [0xff, 0xfe])?;

    let output = project.consultable_output(SET)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("outline.md"), "{stderr}");
    assert!(output.stdout.is_empty(), "{:?}", output.stdout);
    Ok(())
}

#[test]
fn consultable_holds_back_an_item_with_only_skipped_pairs(
) -> Result<(), TestError> {
    let project = Project::multiround()?;
    project
        .append_to_outline(&format!("- [ ] {ARXIV_ITEM} — profiles: arxiv"))?;

    let verdict = project.consultable()?;
    let plan = project.plan()?;

    let skipped_reasons: Vec<Value> = plan["skipped"]
        .as_array()
        .ok_or("skipped")?
        .iter()
        .filter(|skip| skip["question"] == json!(ARXIV_ITEM))
        .map(|skip| skip["reason"].clone())
        .collect();
    assert_eq!(
        verdict,
        json!({
            "consultable": false,
            "gate": "conducted",
            "unanswered": [],
            "held_back": [{"question": ARXIV_ITEM, "reasons": skipped_reasons}],
        })
    );
    assert!(!skipped_reasons.is_empty());
    Ok(())
}

#[test]
fn consultable_ignores_the_concurrency_config() -> Result<(), TestError> {
    let project = Project::multiround()?;
    project.write(
        ".accelerator/config.md",
        "---\nresearch:\n  topic:\n    concurrency: \"\"\n---\n",
    )?;

    let outstanding = project.run("outstanding", &[SET])?;
    let verdict = project.consultable()?;

    assert_eq!(outstanding.status.code(), Some(1));
    assert_eq!(verdict["consultable"], json!(true));
    Ok(())
}

#[test]
fn consultable_lists_an_unanswered_pair() -> Result<(), TestError> {
    let project = Project::multiround()?;
    fs::remove_file(project.set(SET).join("findings/03-third-focus.md"))?;
    project.edit("manifest.md", "round_count: 2", "round_count: 1")?;
    project.edit("manifest.md", "finding_count: 3", "finding_count: 2")?;

    let verdict = project.consultable()?;

    assert_eq!(
        verdict,
        json!({
            "consultable": false,
            "gate": "conducted",
            "unanswered": [{
                "question": "What is the third focus area?",
                "profile": "web",
            }],
            "held_back": [],
        })
    );
    Ok(())
}

#[test]
fn consultable_opens_a_synthesised_fully_conducted_set() -> Result<(), TestError>
{
    let project = Project::multiround()?;

    let verdict = project.consultable()?;

    assert_eq!(verdict["consultable"], json!(true), "{verdict}");
    Ok(())
}

#[test]
fn consultable_lists_the_documents_to_read() -> Result<(), TestError> {
    let project = Project::multiround()?;
    let set = format!("{TOPICS}/{SET}");
    project.write(&format!("{set}/findings/.04-x-web.md.invalid"), "x")?;
    project
        .write(&format!("{set}/findings/01-first-focus.levels/1.md"), "x")?;
    project.write(&format!("{set}/reports/how-agents-plan.md"), "x")?;
    project.write(&format!("{set}/reports/.draft.md"), "x")?;
    project.write(&format!("{set}/reports/notes.txt"), "x")?;

    let verdict = project.consultable()?;
    let counts = project.counts()?;

    assert_eq!(
        verdict,
        json!({
            "consultable": true,
            "findings": [
                project.absolute("findings/01-first-focus.md"),
                project.absolute("findings/02-second-focus.md"),
                project.absolute("findings/03-third-focus.md"),
            ],
            "reports": [project.absolute("reports/how-agents-plan.md")],
        })
    );
    assert_eq!(counts["derived"]["report_count"], json!(1));
    Ok(())
}

#[test]
fn consultable_writes_nothing() -> Result<(), TestError> {
    let project = Project::multiround()?;
    let before = tree(&project.root)?;

    project.consultable()?;

    assert_eq!(tree(&project.root)?, before);
    assert!(!project.set(SET).join(".conduct-run.json").exists());
    Ok(())
}

#[test]
fn consultable_for_an_unknown_set_fails() -> Result<(), TestError> {
    let project = Project::multiround()?;

    let output = project.consultable_output("no-such-set")?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim_end(),
        "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set 'no-such-set'"
    );
    Ok(())
}
