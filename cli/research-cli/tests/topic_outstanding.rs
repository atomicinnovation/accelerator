//! `accelerator-research topic outstanding` black-box CLI coverage: the round
//! plan a committed set yields, as the JSON `conduct` consumes.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use research::topic::layout::finding_path::is_finding_path;
use research::topic::layout::finding_path::is_level_note_path;
use serde_json::json;
use serde_json::Value;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-research");
const TOPICS: &str = "meta/research/topics";
const ACCELERATOR_CAP: &str = "ACCELERATOR_SUBAGENT_CAP";
const HARNESS_CAP: &str = "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS";

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix(&format!("research-topic-{tag}-"))
            .tempdir()?;
        let root = dir.path().canonicalize()?;
        fs::create_dir_all(root.join(".git"))?;
        Ok(Self { _dir: dir, root })
    }

    fn with_fixture_set(self, fixture: &str) -> Result<Self, TestError> {
        copy_tree(&fixtures().join(fixture), &self.set(fixture))?;
        Ok(self)
    }

    fn set(&self, slug: &str) -> PathBuf {
        self.root.join(TOPICS).join(slug)
    }

    fn write(&self, relative: &str, content: &str) -> Result<(), TestError> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn outstanding(
        &self,
        slug: &str,
        profiles_dir: &Path,
    ) -> Result<Output, TestError> {
        Ok(command_without_an_inherited_cap()
            .current_dir(&self.root)
            .args(["topic", "outstanding", slug, "--profiles-dir"])
            .arg(profiles_dir)
            .output()?)
    }

    fn run(&self, args: &[&str]) -> Result<Output, TestError> {
        self.run_in(args, &[])
    }

    fn run_in(
        &self,
        args: &[&str],
        cap: &[(&str, &str)],
    ) -> Result<Output, TestError> {
        Ok(command_without_an_inherited_cap()
            .current_dir(&self.root)
            .envs(cap.iter().copied())
            .arg("topic")
            .args(args)
            .output()?)
    }

    fn outstanding_with(&self, args: &[&str]) -> Result<Output, TestError> {
        self.outstanding_in(args, &[])
    }

    fn outstanding_in(
        &self,
        args: &[&str],
        cap: &[(&str, &str)],
    ) -> Result<Output, TestError> {
        let profiles = installed_profiles();
        let mut all = vec![
            "outstanding",
            "s",
            "--profiles-dir",
            profiles.to_str().ok_or("non-utf8")?,
        ];
        all.extend_from_slice(args);
        self.run_in(&all, cap)
    }

    fn plan_with(&self, args: &[&str]) -> Result<(Value, String), TestError> {
        self.plan_in(args, &[])
    }

    fn plan_in(
        &self,
        args: &[&str],
        cap: &[(&str, &str)],
    ) -> Result<(Value, String), TestError> {
        let output = self.outstanding_in(args, cap)?;
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(output.status.success(), "{stderr}");
        Ok((serde_json::from_slice(&output.stdout)?, stderr))
    }

    fn write_note(
        &self,
        stem: &str,
        lineage: &str,
        question: &str,
        follow_ups: &[&str],
    ) -> Result<(), TestError> {
        self.write(
            &note_path(stem, lineage),
            &level_note(stem, lineage, question, follow_ups),
        )
    }

    fn ledger(&self) -> PathBuf {
        self.set("s").join(".conduct-run.json")
    }

    fn plan(&self, slug: &str) -> Result<Value, TestError> {
        self.plan_with_profiles(slug, &installed_profiles())
    }

    fn plan_with_profiles(
        &self,
        slug: &str,
        profiles_dir: &Path,
    ) -> Result<Value, TestError> {
        let output = self.outstanding(slug, profiles_dir)?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(serde_json::from_slice(&output.stdout)?)
    }
}

fn command_without_an_inherited_cap() -> Command {
    let mut command = Command::new(BIN);
    command.env_remove(ACCELERATOR_CAP).env_remove(HARNESS_CAP);
    command
}

/// The committed topic-research sets live beside the frontmatter goldens that
/// hold them to the schema.
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../corpus-cli/tests/fixtures")
}

fn installed_profiles() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../skills/research/profiles")
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), TestError> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn path_in(set: &Path, name: &str) -> Result<String, TestError> {
    Ok(set
        .join("findings")
        .join(name)
        .to_str()
        .ok_or("non-utf8")?
        .to_owned())
}

fn item(line: u64, question: &str, complete: bool) -> Value {
    json!({"line": line, "question": question, "complete": complete})
}

fn pair(question: &str, profile: &str, path: &str) -> Value {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    json!({
        "question": question,
        "profile": profile,
        "path": path,
        "stage": "single_pass",
        "spawn": stem,
    })
}

const HEADS: &str = "How do attention heads specialise?";
const LONG_CONTEXT: &str = "What limits long-context attention?";

#[test]
fn a_multi_profile_set_leaves_only_its_quarantined_pair_outstanding(
) -> Result<(), TestError> {
    let slug = "topic-research-multiprofile-set";
    let project = Project::new("multiprofile")?.with_fixture_set(slug)?;
    let set = project.set(slug);

    let plan = project.plan(slug)?;

    assert_eq!(
        plan,
        json!({
            "items": [
                item(22, HEADS, false),
                item(23, LONG_CONTEXT, true),
            ],
            "pairs": [pair(
                HEADS,
                "openalex",
                &path_in(
                    &set,
                    "01-how-do-attention-heads-specialise-openalex.md",
                )?,
            )],
            "skipped": [],
            "warnings": [],
            "depth": 1,
            "concurrency": 20,
            "subagent_cap": {"slots": 20, "source": "default"},
            "remaining": 0,
            "unfinished": [],
            "trims": [],
            "shallow": [],
        })
    );
    Ok(())
}

#[test]
fn a_quarantined_single_profile_pair_reuses_its_markers_index(
) -> Result<(), TestError> {
    let slug = "topic-research-quarantine-set";
    let project = Project::new("quarantine")?.with_fixture_set(slug)?;
    let set = project.set(slug);

    let plan = project.plan(slug)?;

    assert_eq!(
        plan,
        json!({
            "items": [
                item(22, "What is the first focus area?", true),
                item(26, "What is the second focus area?", true),
                item(30, "What is the third focus area?", false),
            ],
            "pairs": [pair(
                "What is the third focus area?",
                "web",
                &path_in(&set, "03-what-is-the-third-focus-area-web.md")?,
            )],
            "skipped": [],
            "warnings": [],
            "depth": 1,
            "concurrency": 20,
            "subagent_cap": {"slots": 20, "source": "default"},
            "remaining": 0,
            "unfinished": [],
            "trims": [],
            "shallow": [],
        })
    );
    Ok(())
}

#[test]
fn a_fully_researched_legacy_set_has_nothing_outstanding(
) -> Result<(), TestError> {
    for slug in ["topic-research-set", "topic-research-multiround-set"] {
        let project = Project::new("legacy")?.with_fixture_set(slug)?;

        let plan = project.plan(slug)?;

        assert_eq!(plan["pairs"], json!([]), "{slug}");
        assert_eq!(plan["skipped"], json!([]), "{slug}");
        assert_eq!(plan["warnings"], json!([]), "{slug}");
        let items = plan["items"].as_array().ok_or("no items")?;
        assert!(!items.is_empty(), "{slug}");
        assert!(
            items.iter().all(|item| item["complete"] == json!(true)),
            "{slug}: {items:?}"
        );
    }
    Ok(())
}

const BRIEF: &str = "---\ntype: \"topic-research\"\nkind: \"brief\"\n\
                     source_profiles: [\"web\", \"openalex\"]\n---\n";

fn outline(items: &str) -> String {
    format!(
        "---\ntype: \"topic-research\"\nkind: \"outline\"\n---\n\n{items}\n"
    )
}

fn small_set(tag: &str, items: &str) -> Result<Project, TestError> {
    let project = Project::new(tag)?;
    project.write(&format!("{TOPICS}/s/manifest.md"), "---\n---\n")?;
    project.write(&format!("{TOPICS}/s/brief.md"), BRIEF)?;
    project.write(&format!("{TOPICS}/s/outline.md"), &outline(items))?;
    Ok(project)
}

#[test]
fn an_invalid_unquarantined_finding_leaves_its_pair_outstanding(
) -> Result<(), TestError> {
    let project = small_set("invalid", "- [x] A?")?;
    project.write(
        &format!("{TOPICS}/s/findings/01-a-web.md"),
        "---\ntype: \"topic-research\"\nkind: \"finding\"\n\
         question: \"A?\"\nsource_profile: \"web\"\nstatus: \"draft\"\n---\n",
    )?;

    let plan = project.plan("s")?;

    assert_eq!(plan["items"], json!([item(6, "A?", false)]));
    assert_eq!(
        plan["pairs"],
        json!([pair(
            "A?",
            "web",
            &path_in(&project.set("s"), "02-a-web.md")?
        )])
    );
    Ok(())
}

#[test]
fn a_profiles_suffix_without_a_separator_is_a_warning() -> Result<(), TestError>
{
    let project = small_set("unseparated", "- [ ] A? profiles: openalex")?;

    let plan = project.plan("s")?;

    assert_eq!(plan["pairs"][0]["profile"], json!("web"));
    let warnings = plan["warnings"].as_array().ok_or("no warnings")?;
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].as_str().is_some_and(|w| w.contains("line 6")),
        "{warnings:?}"
    );
    Ok(())
}

#[test]
fn a_profile_missing_from_the_profiles_directory_is_skipped(
) -> Result<(), TestError> {
    let project =
        small_set("uninstalled", "- [ ] A? — profiles: web, openalex")?;
    project.write("profiles/web-profile/SKILL.md", "web\n")?;
    project.write("profiles/openalex-profile/README.md", "no skill\n")?;

    let plan =
        project.plan_with_profiles("s", &project.root.join("profiles"))?;

    assert_eq!(
        plan["pairs"],
        json!([pair(
            "A?",
            "web",
            &path_in(&project.set("s"), "01-a-web.md")?
        )])
    );
    assert_eq!(
        plan["skipped"],
        json!([{
            "question": "A?",
            "profile": "openalex",
            "reason": "no 'openalex-profile' skill is installed",
        }])
    );
    assert_eq!(plan["items"][0]["complete"], json!(false));
    Ok(())
}

#[test]
fn an_unresolvable_set_exits_1() -> Result<(), TestError> {
    let project = Project::new("missing")?;
    fs::create_dir_all(project.root.join(TOPICS))?;

    let output = project.outstanding("no-such-set", &installed_profiles())?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim_end(),
        "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set 'no-such-set'"
    );
    assert!(output.stdout.is_empty());
    Ok(())
}

#[test]
fn every_allocated_path_is_a_finding_path_the_guard_admits(
) -> Result<(), TestError> {
    let slug = "topic-research-multiprofile-set";
    let project = Project::new("guard-shape")?.with_fixture_set(slug)?;
    let fresh = small_set(
        "guard-shape-fresh",
        "- [ ] A? — profiles: web, openalex\n- [ ] 注意力？",
    )?;
    let topics = [project.root.join(TOPICS), fresh.root.join(TOPICS)];

    let mut checked = 0;
    for (plan, topics) in [
        (project.plan(slug)?, &topics[0]),
        (fresh.plan("s")?, &topics[1]),
    ] {
        for pair in plan["pairs"].as_array().ok_or("no pairs")? {
            let path = PathBuf::from(pair["path"].as_str().ok_or("no path")?);
            let relative = path.strip_prefix(topics)?;
            assert!(
                is_finding_path(relative.to_str().ok_or("non-utf8")?),
                "{}",
                path.display()
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 4);
    Ok(())
}

fn note_path(stem: &str, lineage: &str) -> String {
    format!("{TOPICS}/s/findings/{stem}.levels/{lineage}.md")
}

fn level_note_with(stem: &str, lineage: &str, fields: &str) -> String {
    format!(
        "---\ntype: \"topic-research\"\nid: \"s.{stem}.{lineage}\"\n\
         title: \"A note\"\ndate: \"2026-09-27T00:00:00+00:00\"\n\
         author: \"Fixture Author\"\nproducer: \"research-topic\"\n\
         status: \"complete\"\nkind: \"level-note\"\nround: 1\n\
         source_profile: \"web\"\ndepth: 3\n{fields}\
         tags: [\"research\"]\n\
         last_updated: \"2026-09-27T00:00:00+00:00\"\n\
         last_updated_by: \"Fixture Author\"\nschema_version: 1\n---\n\n\
         # A note\n"
    )
}

fn level_note(
    stem: &str,
    lineage: &str,
    question: &str,
    follow_ups: &[&str],
) -> String {
    let level = lineage.split('-').next().unwrap_or("1");
    level_note_with(
        stem,
        lineage,
        &format!(
            "question: {}\nlevel: {level}\nfollow_ups: {}\n",
            json!(question),
            json!(follow_ups)
        ),
    )
}

fn finding(stem: &str, question: &str, depth: &str) -> String {
    format!(
        "---\ntype: \"topic-research\"\nid: \"{stem}\"\n\
         title: \"A\"\ndate: \"2026-09-27T00:00:00+00:00\"\n\
         author: \"Fixture Author\"\nproducer: \"research-topic\"\n\
         status: \"complete\"\nkind: \"finding\"\nround: 1\n\
         question: {}\nsource_profile: \"web\"\n{depth}\
         tags: [\"research\"]\n\
         last_updated: \"2026-09-27T00:00:00+00:00\"\n\
         last_updated_by: \"Fixture Author\"\nschema_version: 1\n---\n",
        json!(question)
    )
}

fn absolute(project: &Project, relative: &str) -> Result<String, TestError> {
    Ok(project
        .root
        .join(relative)
        .to_str()
        .ok_or("non-utf8")?
        .to_owned())
}

fn nodes_of(pair: &Value) -> Result<Vec<Value>, TestError> {
    Ok(pair["nodes"].as_array().ok_or("no nodes")?.clone())
}

fn lineages_of(plan: &Value) -> Result<Vec<String>, TestError> {
    let mut lineages = Vec::new();
    for pair in plan["pairs"].as_array().ok_or("no pairs")? {
        for node in nodes_of(pair)? {
            lineages
                .push(node["lineage"].as_str().ok_or("lineage")?.to_owned());
        }
    }
    Ok(lineages)
}

fn a_set() -> Result<Project, TestError> {
    small_set("deepen", "- [ ] A?")
}

fn seeded_over_cap_tree(project: &Project) -> Result<(), TestError> {
    let level_two = ["B1?", "B2?", "B3?", "B4?", "B5?", "B6?"];
    project.write_note("01-a-web", "1", "A?", &level_two)?;
    for (position, question) in (1..=4).zip(level_two) {
        let follow_ups: Vec<String> =
            (1..=3).map(|k| format!("C{position}{k}?")).collect();
        let follow_ups: Vec<&str> =
            follow_ups.iter().map(String::as_str).collect();
        project.write_note(
            "01-a-web",
            &format!("2-{position}"),
            question,
            &follow_ups,
        )?;
    }
    Ok(())
}

#[test]
fn a_seeded_tree_over_its_caps_reports_the_eight_level_three_lineages(
) -> Result<(), TestError> {
    let project = a_set()?;
    seeded_over_cap_tree(&project)?;

    let (plan, stderr) = project.plan_with(&["--depth", "3"])?;

    assert_eq!(
        lineages_of(&plan)?,
        [
            "3-1-1", "3-1-2", "3-2-1", "3-2-2", "3-3-1", "3-3-2", "3-4-1",
            "3-4-2",
        ]
    );
    let trims: Vec<&str> = stderr
        .lines()
        .filter(|l| l.starts_with("warning: "))
        .collect();
    assert_eq!(
        trims,
        [
            "warning: level note 01-a-web.levels/1 records 6 follow-ups, \
             over its cap of 4; trimmed 2",
            "warning: level note 01-a-web.levels/2-1 records 3 follow-ups, \
             over its cap of 2; trimmed 1",
            "warning: level note 01-a-web.levels/2-2 records 3 follow-ups, \
             over its cap of 2; trimmed 1",
            "warning: level note 01-a-web.levels/2-3 records 3 follow-ups, \
             over its cap of 2; trimmed 1",
            "warning: level note 01-a-web.levels/2-4 records 3 follow-ups, \
             over its cap of 2; trimmed 1",
        ]
    );
    Ok(())
}

#[test]
fn a_missing_level_two_note_holds_back_level_three() -> Result<(), TestError> {
    let project = a_set()?;
    project.write_note("01-a-web", "1", "A?", &["B?", "C?"])?;
    project.write_note("01-a-web", "2-1", "B?", &["X?"])?;

    let (plan, _) = project.plan_with(&["--depth", "3"])?;

    assert_eq!(
        plan["pairs"],
        json!([{
            "question": "A?",
            "profile": "web",
            "path": absolute(&project, &format!("{TOPICS}/s/findings/01-a-web.md"))?,
            "stage": "research_nodes",
            "nodes": [{
                "lineage": "2-2",
                "level": 2,
                "question": "C?",
                "cap": 2,
                "id": "s.01-a-web.2-2",
                "path": absolute(&project, &note_path("01-a-web", "2-2"))?,
                "known_questions": ["A?", "B?", "C?"],
                "spawn": "01-a-web:2-2",
            }],
        }])
    );
    Ok(())
}

#[test]
fn a_refused_note_is_reported_on_its_node_with_its_reason(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.write_note("01-a-web", "1", "A?", &["B?"])?;
    project.write(
        &note_path("01-a-web", "2-1"),
        &level_note_with(
            "01-a-web",
            "2-1",
            "question: \"B?\"\nlevel: 1\nfollow_ups: []\n",
        ),
    )?;

    let (plan, _) = project.plan_with(&["--depth", "2"])?;

    let nodes = nodes_of(&plan["pairs"][0])?;
    assert_eq!(nodes[0]["lineage"], json!("2-1"));
    assert_eq!(
        nodes[0]["rejected"],
        json!("level does not match its lineage")
    );
    Ok(())
}

#[test]
fn a_schema_invalid_note_is_reported_with_its_first_violation_code(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.write(
        &note_path("01-a-web", "1"),
        &level_note_with("01-a-web", "1", "question: \"A?\"\nlevel: 1\n"),
    )?;

    let (plan, _) = project.plan_with(&["--depth", "2"])?;

    let nodes = nodes_of(&plan["pairs"][0])?;
    assert_eq!(nodes[0]["lineage"], json!("1"));
    assert_eq!(
        nodes[0]["rejected"],
        json!("fails validation: MISSING-EXTRA on follow_ups")
    );
    Ok(())
}

#[test]
fn trims_are_reported_in_the_json_and_on_stderr() -> Result<(), TestError> {
    let project = a_set()?;
    project.write_note(
        "01-a-web",
        "1",
        "A?",
        &["B?", "C?", "D?", "E?", "F?"],
    )?;

    let (plan, stderr) = project.plan_with(&["--depth", "2"])?;

    assert_eq!(
        plan["trims"],
        json!([{"stem": "01-a-web", "lineage": "1", "recorded": 5, "cap": 4}])
    );
    assert_eq!(
        stderr,
        "warning: level note 01-a-web.levels/1 records 5 follow-ups, over \
         its cap of 4; trimmed 1\n"
    );
    Ok(())
}

#[test]
fn a_complete_tree_is_reported_for_composition_with_its_note_paths(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.write_note("01-a-web", "1", "A?", &["B?"])?;
    project.write_note("01-a-web", "2-1", "B?", &[])?;

    let (plan, _) = project.plan_with(&["--depth", "2"])?;

    assert_eq!(
        plan["pairs"],
        json!([{
            "question": "A?",
            "profile": "web",
            "path": absolute(&project, &format!("{TOPICS}/s/findings/01-a-web.md"))?,
            "stage": "compose",
            "notes": [
                absolute(&project, &note_path("01-a-web", "1"))?,
                absolute(&project, &note_path("01-a-web", "2-1"))?,
            ],
            "spawn": "01-a-web",
        }])
    );
    Ok(())
}

fn spawns_of(plan: &Value) -> Result<Vec<String>, TestError> {
    let mut spawns = Vec::new();
    for pair in plan["pairs"].as_array().ok_or("no pairs")? {
        if let Some(nodes) = pair["nodes"].as_array() {
            for node in nodes {
                spawns.push(node["spawn"].as_str().ok_or("spawn")?.to_owned());
            }
        } else {
            spawns.push(pair["spawn"].as_str().ok_or("spawn")?.to_owned());
        }
    }
    Ok(spawns)
}

#[test]
fn a_quarantined_root_note_keeps_its_pairs_index_when_a_focus_area_is_appended(
) -> Result<(), TestError> {
    let project = small_set("held", "- [ ] A?\n- [ ] B?")?;
    project.write(
        &format!("{TOPICS}/s/findings/03-a-web.levels/.1.md.invalid"),
        &level_note("03-a-web", "1", "A?", &[]),
    )?;

    let (plan, _) = project.plan_with(&["--depth", "2"])?;

    assert_eq!(spawns_of(&plan)?, ["03-a-web:1", "04-b-web:1"]);
    Ok(())
}

#[test]
fn a_pair_left_only_as_a_levels_directory_resumes_at_its_index(
) -> Result<(), TestError> {
    let project = small_set("resumed", "- [ ] A?\n- [ ] B?")?;
    project.write_note("03-a-web", "1", "A?", &["X?"])?;
    project.write_note("04-b-web", "1", "B?", &[])?;

    let (plan, _) = project.plan_with(&["--depth", "2"])?;

    assert_eq!(spawns_of(&plan)?, ["03-a-web:2-1", "04-b-web"]);
    assert_eq!(plan["pairs"][1]["stage"], json!("compose"));
    Ok(())
}

#[test]
fn depth_one_composes_from_the_root_note_alone() -> Result<(), TestError> {
    let project = a_set()?;
    project.write_note("01-a-web", "1", "A?", &["B?"])?;

    let (plan, _) = project.plan_with(&[])?;

    assert_eq!(plan["depth"], json!(1));
    assert_eq!(plan["pairs"][0]["stage"], json!("compose"));
    assert_eq!(
        plan["pairs"][0]["notes"],
        json!([absolute(&project, &note_path("01-a-web", "1"))?])
    );
    Ok(())
}

#[test]
fn a_legacy_finding_is_answered_and_shallow_at_depth_three(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.write(
        &format!("{TOPICS}/s/findings/01-a-web.md"),
        &finding("01-a-web", "A?", ""),
    )?;

    for (depth, shallow) in [
        ("3", json!([{"stem": "01-a-web", "depth": 1}])),
        ("1", json!([])),
    ] {
        let (plan, _) = project.plan_with(&["--depth", depth])?;

        assert_eq!(
            plan,
            json!({
                "items": [item(6, "A?", true)],
                "pairs": [],
                "skipped": [],
                "warnings": [],
                "depth": depth.parse::<u32>()?,
                "concurrency": 20,
                "subagent_cap": {"slots": 20, "source": "default"},
                "remaining": 0,
                "unfinished": [],
                "trims": [],
                "shallow": shallow,
            }),
            "--depth {depth}"
        );
    }
    Ok(())
}

fn assert_exits_1(output: &Output, stderr: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim_end(), stderr);
    assert!(output.stdout.is_empty());
}

#[test]
fn an_invalid_depth_exits_1() -> Result<(), TestError> {
    let project = a_set()?;
    for depth in ["0", "-1", "many", "99999999999"] {
        assert_exits_1(
            &project.outstanding_with(&["--depth", depth])?,
            &format!(
                "E_TOPIC_RESEARCH_DEPTH: --depth must be a positive integer, \
                 got '{depth}'"
            ),
        );
    }
    Ok(())
}

fn every_node(plan: &Value) -> Result<Vec<Value>, TestError> {
    let mut nodes = Vec::new();
    for pair in plan["pairs"].as_array().ok_or("no pairs")? {
        nodes.extend(nodes_of(pair)?);
    }
    Ok(nodes)
}

#[test]
fn every_allocated_level_note_path_is_one_the_guard_admits(
) -> Result<(), TestError> {
    let project = small_set("note-shape", "- [ ] A?\n- [ ] 注意力？")?;
    seeded_over_cap_tree(&project)?;
    let topics = project.root.join(TOPICS);

    let (plan, _) = project.plan_with(&["--depth", "3"])?;

    let nodes = every_node(&plan)?;
    assert_eq!(nodes.len(), 9);
    for node in nodes {
        let path = PathBuf::from(node["path"].as_str().ok_or("no path")?);
        let relative = path.strip_prefix(&topics)?;
        assert!(
            is_level_note_path(relative.to_str().ok_or("non-utf8")?),
            "{}",
            path.display()
        );
    }
    Ok(())
}

#[test]
fn every_allocated_level_note_id_is_set_scoped_and_matches_its_path(
) -> Result<(), TestError> {
    let project = small_set("note-id", "- [ ] A?\n- [ ] B?")?;
    seeded_over_cap_tree(&project)?;

    let (plan, _) = project.plan_with(&["--depth", "3"])?;

    let nodes = every_node(&plan)?;
    assert_eq!(nodes.len(), 9);
    for node in nodes {
        let path = PathBuf::from(node["path"].as_str().ok_or("no path")?);
        let lineage =
            path.file_stem().and_then(|s| s.to_str()).ok_or("lineage")?;
        let levels = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .ok_or("levels")?;
        let stem = levels.strip_suffix(".levels").ok_or("not levels")?;
        assert_eq!(node["id"], json!(format!("s.{stem}.{lineage}")));
        assert_eq!(node["lineage"], json!(lineage));
    }
    Ok(())
}

#[test]
fn a_concurrency_of_2_offers_the_first_spawns_and_counts_the_rest(
) -> Result<(), TestError> {
    let project = small_set("limited", "- [ ] A?\n- [ ] B?\n- [ ] C?")?;

    let (plan, _) =
        project.plan_with(&["--depth", "2", "--concurrency", "2"])?;

    assert_eq!(spawns_of(&plan)?, ["01-a-web:1", "02-b-web:1"]);
    assert_eq!(plan["remaining"], json!(1));
    Ok(())
}

fn started(
    project: &Project,
    args: &[&str],
) -> Result<(Value, String), TestError> {
    let mut all = vec!["--start"];
    all.extend_from_slice(args);
    let (plan, _) = project.plan_with(&all)?;
    let run = plan["run"].as_str().ok_or("no run")?.to_owned();
    Ok((plan, run))
}

fn continued(
    project: &Project,
    run: &str,
    args: &[&str],
) -> Result<Value, TestError> {
    let mut all = vec!["--run", run];
    all.extend_from_slice(args);
    Ok(project.plan_with(&all)?.0)
}

#[test]
fn an_attempted_node_left_invalid_is_reported_unfinished_on_the_next_plan_of_its_run(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, run) = started(&project, &["--depth", "2"])?;
    project.write(
        &note_path("01-a-web", "1"),
        &level_note_with("01-a-web", "1", "question: \"A?\"\nlevel: 1\n"),
    )?;

    let plan = continued(&project, &run, &["--depth", "2", "--spawned", "0"])?;

    assert_eq!(plan["pairs"], json!([]));
    assert_eq!(
        plan["unfinished"],
        json!([{
            "spawn": "01-a-web:1",
            "rejected": "fails validation: MISSING-EXTRA on follow_ups",
        }])
    );
    Ok(())
}

#[test]
fn a_plan_without_start_or_run_writes_no_ledger() -> Result<(), TestError> {
    let project = a_set()?;

    project.plan_with(&["--depth", "2", "--concurrency", "1"])?;

    assert!(!project.ledger().exists());
    Ok(())
}

#[test]
fn a_started_run_returns_its_minted_id_and_first_batch() -> Result<(), TestError>
{
    let project = a_set()?;

    let (plan, run) = started(&project, &["--depth", "2"])?;

    assert!(
        !run.is_empty()
            && run
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "{run}"
    );
    assert_eq!(plan["batch"], json!(0));
    assert_eq!(plan["unexpected"], json!([]));
    assert_eq!(spawns_of(&plan)?, ["01-a-web:1"]);
    assert!(project.ledger().exists());
    Ok(())
}

fn replaced_warning(run: &str) -> String {
    format!(
        "replaced the ledger of run {run}; if that run is still going it \
         stops at its next batch, and notes its last batch writes may show \
         as unexpected"
    )
}

#[test]
fn starting_over_a_stale_ledger_replaces_it_with_a_warning(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, first) = started(&project, &[])?;

    let output = project.outstanding_with(&["--start"])?;

    let plan: Value = serde_json::from_slice(&output.stdout)?;
    assert_ne!(plan["run"], json!(first));
    assert_eq!(plan["warnings"], json!([replaced_warning(&first)]));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("warning: {}\n", replaced_warning(&first))
    );
    let stored: Value =
        serde_json::from_str(&fs::read_to_string(project.ledger())?)?;
    assert_eq!(stored["run"], plan["run"]);
    Ok(())
}

#[test]
fn starting_over_a_corrupt_ledger_replaces_it_with_a_warning(
) -> Result<(), TestError> {
    let project = a_set()?;
    fs::write(project.ledger(), "{")?;

    let (plan, _) = started(&project, &[])?;

    assert_eq!(
        plan["warnings"],
        json!([
            "replaced a corrupt run ledger; if its run is still going it \
             stops at its next batch, and notes its last batch writes may \
             show as unexpected"
        ])
    );
    Ok(())
}

fn superseded(run: &str, owner: &str) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN_SUPERSEDED: run {run} was superseded by run \
         {owner}; another conduct run now owns this set, so let it finish"
    )
}

#[test]
fn continuing_a_superseded_run_exits_1_naming_the_owner(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, first) = started(&project, &[])?;
    let (_, second) = started(&project, &[])?;

    assert_exits_1(
        &project.outstanding_with(&["--run", &first])?,
        &superseded(&first, &second),
    );
    Ok(())
}

#[test]
fn a_superseded_continuation_leaves_the_owners_ledger_bytes_unchanged(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, first) = started(&project, &[])?;
    started(&project, &[])?;
    let before = fs::read(project.ledger())?;

    let output =
        project.outstanding_with(&["--run", &first, "--spawned", "0"])?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read(project.ledger())?, before);
    Ok(())
}

#[test]
fn a_repeated_plan_without_spawned_re_offers_the_same_batch(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (first, run) = started(&project, &["--depth", "2"])?;

    let again = continued(&project, &run, &["--depth", "2"])?;

    assert_eq!(spawns_of(&again)?, spawns_of(&first)?);
    assert_eq!(again["batch"], json!(0));
    Ok(())
}

#[test]
fn spawned_acknowledges_the_batch_and_offers_the_next() -> Result<(), TestError>
{
    let project = a_set()?;
    let (_, run) = started(&project, &["--depth", "2"])?;
    project.write_note("01-a-web", "1", "A?", &["B?"])?;

    let next = continued(&project, &run, &["--depth", "2", "--spawned", "0"])?;

    assert_eq!(spawns_of(&next)?, ["01-a-web:2-1"]);
    assert_eq!(next["batch"], json!(1));
    assert_eq!(next["unexpected"], json!([]));
    Ok(())
}

#[test]
fn the_rendered_batch_is_the_number_the_next_spawned_must_pass(
) -> Result<(), TestError> {
    let project = small_set("batch", "- [ ] A?\n- [ ] B?")?;
    let (_, run) = started(&project, &["--depth", "2"])?;
    project.write_note("01-a-web", "1", "A?", &[])?;

    let changed = continued(&project, &run, &["--depth", "2"])?;
    assert_eq!(changed["batch"], json!(1));
    assert_eq!(spawns_of(&changed)?, ["01-a-web", "02-b-web:1"]);

    let stale = continued(&project, &run, &["--depth", "2", "--spawned", "0"])?;
    assert_eq!(stale["batch"], json!(1));
    assert_eq!(spawns_of(&stale)?, ["01-a-web", "02-b-web:1"]);

    let acknowledged =
        continued(&project, &run, &["--depth", "2", "--spawned", "1"])?;
    assert_eq!(acknowledged["batch"], json!(2));
    assert_eq!(spawns_of(&acknowledged)?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_run_keeps_a_failed_pairs_stem_after_a_later_pair_writes(
) -> Result<(), TestError> {
    let project = small_set("pinned", "- [ ] A?\n- [ ] B?")?;
    let (first, run) = started(&project, &[])?;
    assert_eq!(spawns_of(&first)?, ["01-a-web", "02-b-web"]);
    project.write(
        &format!("{TOPICS}/s/findings/02-b-web.md"),
        &finding("02-b-web", "B?", "depth: 1\n"),
    )?;

    let next = continued(&project, &run, &["--spawned", "0"])?;

    assert_eq!(
        next["unfinished"],
        json!([{"spawn": "01-a-web", "rejected": null}])
    );
    assert_eq!(next["unexpected"], json!([]));
    Ok(())
}

#[test]
fn a_resumed_run_does_not_report_notes_present_at_start_as_unexpected(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.write_note("01-a-web", "1", "A?", &["B?"])?;

    let (plan, run) = started(&project, &["--depth", "2"])?;
    let again = continued(&project, &run, &["--depth", "2"])?;

    assert_eq!(plan["unexpected"], json!([]));
    assert_eq!(again["unexpected"], json!([]));
    Ok(())
}

#[test]
fn a_note_written_mid_run_without_being_offered_is_reported_unexpected(
) -> Result<(), TestError> {
    let project = small_set("forged", "- [ ] A?\n- [ ] B?")?;
    let (first, run) =
        started(&project, &["--depth", "2", "--concurrency", "1"])?;
    assert_eq!(spawns_of(&first)?, ["01-a-web:1"]);
    project.write_note("01-a-web", "1", "A?", &[])?;
    project.write_note("02-b-web", "1", "B?", &[])?;

    let next = continued(
        &project,
        &run,
        &["--depth", "2", "--concurrency", "1", "--spawned", "0"],
    )?;

    assert_eq!(next["unexpected"], json!(["02-b-web:1"]));
    Ok(())
}

#[test]
fn an_accepted_note_overwritten_mid_run_is_reported_unexpected(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.write_note("01-a-web", "1", "A?", &["B?"])?;
    let (_, run) = started(&project, &["--depth", "2"])?;
    project.write_note("01-a-web", "1", "A?", &["C?"])?;

    let next = continued(&project, &run, &["--depth", "2"])?;

    assert_eq!(next["unexpected"], json!(["01-a-web:1"]));
    Ok(())
}

#[test]
fn a_ledger_write_failure_exits_1_and_prints_no_plan() -> Result<(), TestError>
{
    use std::os::unix::fs::PermissionsExt as _;

    let project = a_set()?;
    let set = project.set("s");
    fs::set_permissions(&set, fs::Permissions::from_mode(0o555))?;

    let output = project.outstanding_with(&["--start"]);
    fs::set_permissions(&set, fs::Permissions::from_mode(0o755))?;
    let output = output?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let ledger = project.ledger();
    assert!(
        stderr.starts_with(&format!(
            "E_TOPIC_RESEARCH_RUN_LEDGER: could not write {}: ",
            ledger.display()
        )),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
    assert!(!ledger.exists());
    Ok(())
}

fn no_usable_ledger(run: &str) -> String {
    format!(
        "E_TOPIC_RESEARCH_RUN_LEDGER: no usable ledger for run {run}; re-run \
         conduct to start a fresh run"
    )
}

#[test]
fn continuing_with_a_corrupt_ledger_exits_1() -> Result<(), TestError> {
    let project = a_set()?;
    assert_exits_1(
        &project.outstanding_with(&["--run", "r1"])?,
        &no_usable_ledger("r1"),
    );

    fs::write(project.ledger(), "{")?;

    assert_exits_1(
        &project.outstanding_with(&["--run", "r1"])?,
        &no_usable_ledger("r1"),
    );
    Ok(())
}

#[test]
fn end_run_deletes_only_its_own_ledger_and_tolerates_absence(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, run) = started(&project, &[])?;

    assert_exits_1(
        &project.run(&["end-run", "s", "--run", "other"])?,
        &superseded("other", &run),
    );
    assert!(project.ledger().exists());

    for _ in 0..2 {
        let output = project.run(&["end-run", "s", "--run", &run])?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert!(!project.ledger().exists());
    }
    Ok(())
}

#[test]
fn a_malformed_run_or_spawned_exits_1() -> Result<(), TestError> {
    let project = a_set()?;
    let cases: [(&[&str], String); 5] = [
        (
            &["--run", "a b"],
            "E_TOPIC_RESEARCH_RUN: --run must be 1 to 64 letters, digits, \
             '_' or '-', got 'a b'"
                .to_owned(),
        ),
        (
            &["--start", "--run", "x"],
            "E_TOPIC_RESEARCH_RUN: --start and --run cannot be combined"
                .to_owned(),
        ),
        (
            &["--spawned", "1"],
            "E_TOPIC_RESEARCH_SPAWNED: --spawned needs --run".to_owned(),
        ),
        (
            &["--start", "--spawned", "1"],
            "E_TOPIC_RESEARCH_SPAWNED: --spawned needs --run".to_owned(),
        ),
        (
            &["--run", "x", "--spawned", "many"],
            "E_TOPIC_RESEARCH_SPAWNED: --spawned must be a non-negative \
             integer, got 'many'"
                .to_owned(),
        ),
    ];
    for (args, stderr) in cases {
        assert_exits_1(&project.outstanding_with(args)?, &stderr);
    }
    assert!(!project.ledger().exists());
    Ok(())
}

fn arxiv_set(tag: &str) -> Result<Project, TestError> {
    let items = (1..=25)
        .map(|n| format!("- [ ] Question {n}? — profiles: arxiv"))
        .collect::<Vec<_>>()
        .join("\n");
    let project = small_set(tag, &items)?;
    project.write(
        &format!("{TOPICS}/s/brief.md"),
        "---\ntype: \"topic-research\"\nkind: \"brief\"\n\
         source_profiles: [\"arxiv\"]\n---\n",
    )?;
    Ok(project)
}

fn assert_offers_arxiv_spawns(
    plan: &Value,
    offered: usize,
    remaining: usize,
) -> Result<(), TestError> {
    let spawns = spawns_of(plan)?;
    assert_eq!(spawns.len(), offered, "{plan}");
    assert!(
        spawns.iter().all(|spawn| spawn.contains("-arxiv")),
        "{spawns:?}"
    );
    assert_eq!(plan["remaining"], json!(remaining));
    Ok(())
}

#[test]
fn a_run_offers_20_arxiv_spawns_by_default_and_counts_the_rest(
) -> Result<(), TestError> {
    let project = arxiv_set("arxiv-batch")?;

    let (plan, _) = started(&project, &[])?;

    assert_offers_arxiv_spawns(&plan, 20, 5)?;
    assert_eq!(plan["concurrency"], json!(20));
    Ok(())
}

const FLAG: &str = "--concurrency";
const KNOB: &str = "research.topic.concurrency";
const TEAM_KNOB: &str = "research.topic.concurrency in .accelerator/config.md";
const PERSONAL_KNOB: &str =
    "research.topic.concurrency in .accelerator/config.local.md";

impl Project {
    fn configure_team_concurrency(&self, value: &str) -> Result<(), TestError> {
        self.write(".accelerator/config.md", &concurrency_config(value))
    }

    fn configure_personal_concurrency(
        &self,
        value: &str,
    ) -> Result<(), TestError> {
        use std::os::unix::fs::PermissionsExt as _;

        let personal = ".accelerator/config.local.md";
        self.write(personal, &concurrency_config(value))?;
        fs::set_permissions(
            self.root.join(personal),
            fs::Permissions::from_mode(0o600),
        )?;
        Ok(())
    }
}

fn concurrency_config(value: &str) -> String {
    format!("---\nresearch:\n  topic:\n    concurrency: {value}\n---\n")
}

fn clamped_to_the_default(source: &str, given: &str) -> String {
    format!(
        "{source} is {given}, above the subagent cap of 20, Claude Code's \
         default — clamping to 20; lower it, or raise the cap with \
         {HARNESS_CAP}"
    )
}

fn clamped_by(source: &str, given: &str, cap: u32, variable: &str) -> String {
    format!(
        "{source} is {given}, above the subagent cap of {cap} from \
         {variable} — clamping to {cap}; lower it, or raise {variable}"
    )
}

fn not_a_positive_integer(source: &str, given: &str) -> String {
    format!(
        "{source} must be a positive integer, got '{given}' — clamping to 1"
    )
}

fn assert_warned(plan: &Value, stderr: &str, warnings: &[String]) {
    assert_eq!(plan["warnings"], json!(warnings));
    let printed: Vec<String> = warnings
        .iter()
        .map(|warning| format!("warning: {warning}"))
        .collect();
    assert_eq!(stderr.lines().collect::<Vec<_>>(), printed);
}

fn assert_no_concurrency_diagnostic(plan: &Value, stderr: &str) {
    assert_eq!(plan["warnings"], json!([]), "{plan}");
    assert!(
        !stderr
            .lines()
            .any(|line| line.contains(FLAG) || line.contains(KNOB)),
        "{stderr}"
    );
}

#[test]
fn a_configured_concurrency_above_the_cap_clamps_with_a_warning(
) -> Result<(), TestError> {
    let project = arxiv_set("config-above-cap")?;
    project.configure_team_concurrency("24")?;

    let (plan, stderr) = project.plan_with(&["--start"])?;

    assert_offers_arxiv_spawns(&plan, 20, 5)?;
    assert_eq!(plan["concurrency"], json!(20));
    assert_warned(&plan, &stderr, &[clamped_to_the_default(TEAM_KNOB, "24")]);
    Ok(())
}

#[test]
fn a_configured_concurrency_at_the_cap_is_not_clamped() -> Result<(), TestError>
{
    let project = a_set()?;
    project.configure_team_concurrency("20")?;

    let (plan, stderr) = project.plan_with(&["--start"])?;

    assert_eq!(plan["concurrency"], json!(20));
    assert_no_concurrency_diagnostic(&plan, &stderr);
    Ok(())
}

#[test]
fn a_clamp_names_the_variable_that_set_the_cap() -> Result<(), TestError> {
    for variable in [ACCELERATOR_CAP, HARNESS_CAP] {
        let project = a_set()?;
        project.configure_team_concurrency("24")?;

        let (plan, stderr) =
            project.plan_in(&["--start"], &[(variable, "8")])?;

        assert_eq!(plan["concurrency"], json!(8), "{variable}");
        assert_warned(
            &plan,
            &stderr,
            &[clamped_by(TEAM_KNOB, "24", 8, variable)],
        );
    }
    Ok(())
}

#[test]
fn a_plan_reports_the_subagent_cap_and_its_source() -> Result<(), TestError> {
    let rows: [(&[(&str, &str)], Value); 3] = [
        (&[], json!({"slots": 20, "source": "default"})),
        (
            &[(ACCELERATOR_CAP, "8"), (HARNESS_CAP, "30")],
            json!({"slots": 8, "source": ACCELERATOR_CAP}),
        ),
        (
            &[(HARNESS_CAP, "30")],
            json!({"slots": 30, "source": HARNESS_CAP}),
        ),
    ];
    for (variables, reported) in rows {
        let project = a_set()?;

        let (plan, _) = project.plan_in(&[], variables)?;

        assert_eq!(plan["subagent_cap"], reported, "{variables:?}");
    }
    Ok(())
}

#[test]
fn a_raised_cap_admits_a_higher_concurrency() -> Result<(), TestError> {
    let project = arxiv_set("raised-cap")?;
    project.configure_team_concurrency("24")?;

    let (plan, stderr) =
        project.plan_in(&["--start"], &[(HARNESS_CAP, "30")])?;

    assert_offers_arxiv_spawns(&plan, 24, 1)?;
    assert_eq!(plan["concurrency"], json!(24));
    assert_no_concurrency_diagnostic(&plan, &stderr);
    Ok(())
}

#[test]
fn a_lowered_cap_clamps_the_default_silently() -> Result<(), TestError> {
    let project = arxiv_set("lowered-cap")?;

    let (plan, stderr) =
        project.plan_in(&["--start"], &[(HARNESS_CAP, "8")])?;

    assert_offers_arxiv_spawns(&plan, 8, 17)?;
    assert_eq!(plan["concurrency"], json!(8));
    assert_no_concurrency_diagnostic(&plan, &stderr);
    Ok(())
}

#[test]
fn a_concurrency_flag_wins_over_config_and_clamps_with_a_warning(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.configure_team_concurrency("6")?;

    let (plan, stderr) =
        project.plan_with(&["--concurrency", "50", "--start"])?;

    assert_eq!(plan["concurrency"], json!(20));
    assert_warned(&plan, &stderr, &[clamped_to_the_default(FLAG, "50")]);
    Ok(())
}

#[test]
fn a_personal_concurrency_wins_over_the_team_and_warns() -> Result<(), TestError>
{
    let project = a_set()?;
    project.configure_team_concurrency("6")?;
    project.configure_personal_concurrency("24")?;

    let (plan, stderr) = project.plan_with(&["--start"])?;

    assert_eq!(plan["concurrency"], json!(20));
    assert_warned(
        &plan,
        &stderr,
        &[clamped_to_the_default(PERSONAL_KNOB, "24")],
    );
    Ok(())
}

#[test]
fn the_cap_is_read_as_the_harness_reads_it() -> Result<(), TestError> {
    for (cap, slots) in [("007", 7), (" +8 ", 8)] {
        let project = a_set()?;

        let (plan, stderr) =
            project.plan_in(&["--start"], &[(HARNESS_CAP, cap)])?;

        assert_eq!(plan["concurrency"], json!(slots), "{cap:?}");
        assert_no_concurrency_diagnostic(&plan, &stderr);
    }
    Ok(())
}

const CAPS_THE_HARNESS_IGNORES: [&str; 2] = ["0", "abc"];

#[test]
fn a_cap_the_harness_ignores_leaves_the_default_unwarned(
) -> Result<(), TestError> {
    for cap in CAPS_THE_HARNESS_IGNORES {
        let project = a_set()?;

        let (plan, stderr) =
            project.plan_in(&["--start"], &[(HARNESS_CAP, cap)])?;

        assert_eq!(plan["concurrency"], json!(20), "{cap:?}");
        assert_no_concurrency_diagnostic(&plan, &stderr);
    }
    Ok(())
}

#[test]
fn a_cap_the_harness_ignores_still_clamps_a_configured_concurrency(
) -> Result<(), TestError> {
    for cap in CAPS_THE_HARNESS_IGNORES {
        let project = a_set()?;
        project.configure_team_concurrency("24")?;

        let (plan, stderr) =
            project.plan_in(&["--start"], &[(HARNESS_CAP, cap)])?;

        assert_eq!(plan["concurrency"], json!(20), "{cap:?}");
        assert_warned(
            &plan,
            &stderr,
            &[clamped_to_the_default(TEAM_KNOB, "24")],
        );
    }
    Ok(())
}

#[test]
fn the_accelerator_cap_wins_over_the_harness_cap() -> Result<(), TestError> {
    let project = a_set()?;
    project.configure_team_concurrency("24")?;

    let (plan, stderr) = project.plan_in(
        &["--start"],
        &[(ACCELERATOR_CAP, "30"), (HARNESS_CAP, "8")],
    )?;

    assert_eq!(plan["concurrency"], json!(24));
    assert_no_concurrency_diagnostic(&plan, &stderr);
    Ok(())
}

#[test]
fn an_accelerator_cap_the_harness_would_ignore_falls_through_with_a_warning(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.configure_team_concurrency("24")?;

    let (plan, stderr) = project.plan_in(
        &["--start"],
        &[(ACCELERATOR_CAP, "abc"), (HARNESS_CAP, "8")],
    )?;

    assert_eq!(plan["concurrency"], json!(8));
    assert_warned(
        &plan,
        &stderr,
        &[
            format!(
                "{ACCELERATOR_CAP} is 'abc', not a positive integer — \
                 ignoring it"
            ),
            clamped_by(TEAM_KNOB, "24", 8, HARNESS_CAP),
        ],
    );
    Ok(())
}

#[test]
fn a_blank_accelerator_cap_is_unset() -> Result<(), TestError> {
    let project = a_set()?;

    let (plan, stderr) =
        project.plan_in(&["--start"], &[(ACCELERATOR_CAP, "")])?;

    assert_eq!(plan["concurrency"], json!(20));
    assert_eq!(plan["warnings"], json!([]), "{plan}");
    assert!(!stderr.contains(ACCELERATOR_CAP), "{stderr}");
    Ok(())
}

#[test]
fn a_malformed_concurrency_flag_clamps_to_1_with_a_warning(
) -> Result<(), TestError> {
    for given in ["0", "50.5"] {
        let project = arxiv_set("malformed-flag")?;

        let (plan, stderr) =
            project.plan_with(&["--concurrency", given, "--start"])?;

        assert_offers_arxiv_spawns(&plan, 1, 24)?;
        assert_eq!(plan["concurrency"], json!(1), "{given}");
        assert_warned(&plan, &stderr, &[not_a_positive_integer(FLAG, given)]);
    }
    Ok(())
}

#[test]
fn a_malformed_configured_concurrency_clamps_to_1_with_a_warning(
) -> Result<(), TestError> {
    for given in ["0", "many"] {
        let project = a_set()?;
        project.configure_team_concurrency(given)?;

        let (plan, stderr) = project.plan_with(&["--start"])?;

        assert_eq!(plan["concurrency"], json!(1), "{given}");
        assert_warned(
            &plan,
            &stderr,
            &[not_a_positive_integer(TEAM_KNOB, given)],
        );
    }
    Ok(())
}

#[test]
fn an_empty_concurrency_exits_1_and_writes_no_ledger() -> Result<(), TestError>
{
    let team = a_set()?;
    team.configure_team_concurrency("\"\"")?;
    let personal = a_set()?;
    personal.configure_team_concurrency("6")?;
    personal.configure_personal_concurrency("\"\"")?;
    let flagged = a_set()?;
    let cases: [(&Project, &[&str], &str); 3] = [
        (
            &team,
            &[],
            "E_TOPIC_RESEARCH_CONCURRENCY: research.topic.concurrency in \
             .accelerator/config.md is empty; set it to a positive integer or \
             remove it",
        ),
        (
            &personal,
            &[],
            "E_TOPIC_RESEARCH_CONCURRENCY: research.topic.concurrency in \
             .accelerator/config.local.md is empty; set it to a positive \
             integer or remove it",
        ),
        (
            &flagged,
            &["--concurrency", ""],
            "E_TOPIC_RESEARCH_CONCURRENCY: --concurrency is empty; pass a \
             positive integer",
        ),
    ];
    for (project, args, error) in cases {
        for run in [&["--start"][..], &[]] {
            let mut all = args.to_vec();
            all.extend_from_slice(run);

            assert_exits_1(&project.outstanding_with(&all)?, error);
            assert!(!project.ledger().exists(), "{all:?}");
        }
    }
    Ok(())
}

#[test]
fn a_concurrency_flag_stands_in_for_an_empty_configured_value(
) -> Result<(), TestError> {
    for run in [&["--start"][..], &[]] {
        let project = a_set()?;
        project.configure_team_concurrency("\"\"")?;
        let mut all = vec!["--concurrency", "5"];
        all.extend_from_slice(run);

        let (plan, stderr) = project.plan_with(&all)?;

        assert_eq!(plan["concurrency"], json!(5), "{all:?}");
        assert_no_concurrency_diagnostic(&plan, &stderr);
    }
    Ok(())
}

#[test]
fn an_unreadable_config_exits_1() -> Result<(), TestError> {
    let project = a_set()?;
    project.write(".accelerator/config.md", "---\nresearch:\n")?;

    let output = project.outstanding_with(&["--start"])?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("has malformed frontmatter"), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(!project.ledger().exists());
    Ok(())
}

#[test]
fn an_unreadable_config_is_not_reported_as_an_unresolved_set(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.write(".accelerator/config.md", "---\nresearch:\n")?;

    let output = project.outstanding_with(&["--concurrency", "5"])?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("has malformed frontmatter"), "{stderr}");
    assert!(!stderr.contains("E_TOPIC_RESEARCH_UNRESOLVED"), "{stderr}");
    Ok(())
}

#[test]
fn concurrency_warnings_precede_a_replaced_ledger_notice(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.configure_team_concurrency("24")?;
    let (_, first) = started(&project, &[])?;

    let (plan, stderr) = project.plan_with(&["--start"])?;

    assert_warned(
        &plan,
        &stderr,
        &[
            clamped_to_the_default(TEAM_KNOB, "24"),
            replaced_warning(&first),
        ],
    );
    Ok(())
}

#[test]
fn concurrency_warnings_are_raised_only_when_a_run_starts(
) -> Result<(), TestError> {
    let project = arxiv_set("warn-on-start")?;
    project.configure_team_concurrency("24")?;
    let (_, run) = started(&project, &[])?;

    let (next, stderr) =
        project.plan_with(&["--run", &run, "--spawned", "0"])?;

    assert_offers_arxiv_spawns(&next, 5, 0)?;
    assert_eq!(next["unfinished"].as_array().map(Vec::len), Some(20));
    assert_no_concurrency_diagnostic(&next, &stderr);

    let (query, stderr) = project.plan_with(&[])?;

    assert_offers_arxiv_spawns(&query, 20, 5)?;
    assert_no_concurrency_diagnostic(&query, &stderr);
    Ok(())
}

#[test]
fn limit_is_no_longer_an_outstanding_flag() -> Result<(), TestError> {
    let project = a_set()?;

    let output = project.outstanding_with(&["--limit", "2"])?;

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument '--limit'"), "{stderr}");
    Ok(())
}

#[test]
fn subagent_cap_is_not_an_outstanding_flag() -> Result<(), TestError> {
    let project = a_set()?;

    let output = project.outstanding_with(&["--subagent-cap", "8"])?;

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument '--subagent-cap'"),
        "{stderr}"
    );
    Ok(())
}
