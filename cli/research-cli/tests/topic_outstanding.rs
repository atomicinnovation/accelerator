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
const SEARCH_CAP: &str = "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION";

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
    command
        .env_remove(ACCELERATOR_CAP)
        .env_remove(HARNESS_CAP)
        .env_remove(SEARCH_CAP);
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

fn unanswered(stem: &str, question: &str, profile: &str, stage: &str) -> Value {
    json!({
        "stem": stem,
        "question": question,
        "profile": profile,
        "stage": stage,
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
            "search_cap": {"searches": 200, "source": "default"},
            "remaining": 0,
            "unfinished": [],
            "trims": [],
            "shallow": [],
            "unanswered": [unanswered(
                "01-how-do-attention-heads-specialise-openalex",
                HEADS,
                "openalex",
                "single_pass",
            )],
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
            "search_cap": {"searches": 200, "source": "default"},
            "remaining": 0,
            "unfinished": [],
            "trims": [],
            "shallow": [],
            "unanswered": [unanswered(
                "03-what-is-the-third-focus-area-web",
                "What is the third focus area?",
                "web",
                "single_pass",
            )],
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
                "search_cap": {"searches": 200, "source": "default"},
                "remaining": 0,
                "unfinished": [],
                "trims": [],
                "shallow": shallow,
                "unanswered": [],
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
    for depth in ["0", "-1", "11", "many", "99999999999"] {
        assert_exits_1(
            &project.outstanding_with(&["--depth", depth])?,
            &format!(
                "E_TOPIC_RESEARCH_DEPTH: --depth must be 1 to 10, got \
                 '{depth}'"
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

fn replaced_notice(run: &str) -> String {
    format!(
        "replaced the ledger of run {run}; if it is still going it stops at \
         its next batch"
    )
}

fn replaced_notice_with_quarantine(run: &str) -> String {
    format!(
        "{}; before acting on this plan, rename each replaced_run quarantine \
         path to .<name>.invalid beside it, never deleting it",
        replaced_notice(run)
    )
}

const CORRUPT_LEDGER: &str = "replaced a corrupt run ledger; if its run is \
                              still going it stops at its next batch, and \
                              notes its last batch writes may show as \
                              unexpected";

#[test]
fn starting_over_a_stale_ledger_reports_it_without_a_warning(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, first) = started(&project, &[])?;

    let output = project.outstanding_with(&["--start"])?;

    let plan: Value = serde_json::from_slice(&output.stdout)?;
    assert_ne!(plan["run"], json!(first));
    assert_eq!(plan["warnings"], json!([]));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("warning: {}\n", replaced_notice(&first))
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

    assert_eq!(plan["warnings"], json!([CORRUPT_LEDGER]));
    assert_eq!(plan.get("replaced_run"), None);
    Ok(())
}

fn left(spawn: &str, question: &str) -> Value {
    json!({"spawn": spawn, "question": question, "profile": "web"})
}

fn finding_at(project: &Project, stem: &str) -> Result<String, TestError> {
    absolute(project, &format!("{TOPICS}/s/findings/{stem}.md"))
}

fn write_finding(
    project: &Project,
    stem: &str,
    content: &str,
) -> Result<(), TestError> {
    project.write(&format!("{TOPICS}/s/findings/{stem}.md"), content)
}

const FAILING_VALIDATION: &str = "---\ntype: \"topic-research\"\n\
                                  kind: \"finding\"\nquestion: \"A?\"\n\
                                  source_profile: \"web\"\n\
                                  status: \"draft\"\n---\n";

fn keys_of(plan: &Value) -> Result<Vec<String>, TestError> {
    let mut keys: Vec<String> = plan
        .as_object()
        .ok_or("not an object")?
        .keys()
        .cloned()
        .collect();
    keys.sort();
    Ok(keys)
}

#[test]
fn starting_over_a_stale_ledger_reports_its_last_batch_by_class(
) -> Result<(), TestError> {
    let project = small_set(
        "replaced-classes",
        "- [ ] C?\n- [ ] D?\n- [ ] E?\n- [ ] A?\n- [ ] B?",
    )?;
    project.write_note("01-a-web", "1", "A?", &[])?;
    project.write_note("02-b-web", "1", "B?", &[])?;
    let args = ["--depth", "2", "--concurrency", "5"];
    let (offered, first) = started(&project, &args)?;
    assert_eq!(
        spawns_of(&offered)?,
        [
            "03-c-web:1",
            "04-d-web:1",
            "05-e-web:1",
            "01-a-web",
            "02-b-web"
        ]
    );
    write_finding(
        &project,
        "02-b-web",
        &finding("02-b-web", "B?", "depth: 2\n"),
    )?;
    write_finding(&project, "01-a-web", FAILING_VALIDATION)?;
    project.write_note("03-c-web", "1", "C?", &[])?;
    project.write(
        &note_path("04-d-web", "1"),
        &level_note_with(
            "04-d-web",
            "1",
            "question: \"D?\"\nlevel: 2\nfollow_ups: []\n",
        ),
    )?;

    let (plan, _) = started(&project, &args)?;

    assert_eq!(
        plan["replaced_run"],
        json!({
            "run": first,
            "valid": [left("03-c-web:1", "C?"), left("02-b-web", "B?")],
            "invalid": [left("04-d-web:1", "D?"), left("01-a-web", "A?")],
            "unwritten": [left("05-e-web:1", "E?")],
            "unasked": [],
            "quarantine": [finding_at(&project, "01-a-web")?],
            "owed": [],
        })
    );
    let mut expected_keys = keys_of(&offered)?;
    expected_keys.push("replaced_run".to_owned());
    expected_keys.sort();
    assert_eq!(keys_of(&plan)?, expected_keys);
    Ok(())
}

#[test]
fn an_invalid_or_misdirected_finding_from_a_replaced_batch_is_listed_for_quarantine(
) -> Result<(), TestError> {
    let project = small_set("replaced-invalid", "- [ ] A?\n- [ ] B?")?;
    started(&project, &[])?;
    write_finding(&project, "01-a-web", FAILING_VALIDATION)?;
    write_finding(
        &project,
        "02-b-web",
        &finding("02-b-web", "Z?", "depth: 1\n"),
    )?;

    let (plan, _) = started(&project, &[])?;

    let replaced = &plan["replaced_run"];
    assert_eq!(
        replaced["invalid"],
        json!([left("01-a-web", "A?"), left("02-b-web", "B?")])
    );
    assert_eq!(
        replaced["quarantine"],
        json!([
            finding_at(&project, "01-a-web")?,
            finding_at(&project, "02-b-web")?,
        ])
    );
    assert_eq!(spawns_of(&plan)?, ["01-a-web", "03-b-web"]);
    Ok(())
}

#[test]
fn a_stale_ledger_with_findings_to_quarantine_asks_a_hand_run_to_rename_them(
) -> Result<(), TestError> {
    let project = small_set("replaced-hand-run", "- [ ] A?\n- [ ] B?")?;
    let (_, first) = started(&project, &[])?;
    write_finding(&project, "01-a-web", FAILING_VALIDATION)?;

    let (plan, stderr) = project.plan_with(&["--start"])?;

    assert_eq!(plan["warnings"], json!([]));
    assert_eq!(
        stderr,
        format!("warning: {}\n", replaced_notice_with_quarantine(&first))
    );
    Ok(())
}

fn sibling_answered(tag: &str) -> Result<Project, TestError> {
    let project = small_set(tag, "- [ ] A?\n- [ ] B?")?;
    started(&project, &[])?;
    write_finding(
        &project,
        "02-b-web",
        &finding("02-b-web", "A?", "depth: 1\n"),
    )?;
    Ok(project)
}

fn quarantine_as_the_report_would(plan: &Value) -> Result<(), TestError> {
    for path in plan["replaced_run"]["quarantine"]
        .as_array()
        .ok_or("no quarantine")?
    {
        let path = Path::new(path.as_str().ok_or("not a path")?);
        let name = path.file_name().ok_or("no name")?.to_string_lossy();
        fs::rename(path, path.with_file_name(format!(".{name}.invalid")))?;
    }
    Ok(())
}

#[test]
fn a_finding_answering_a_sibling_pair_is_listed_for_quarantine(
) -> Result<(), TestError> {
    let project = sibling_answered("replaced-sibling")?;

    let (plan, _) = started(&project, &[])?;

    let replaced = &plan["replaced_run"];
    assert_eq!(replaced["invalid"], json!([left("02-b-web", "B?")]));
    assert_eq!(replaced["unwritten"], json!([left("01-a-web", "A?")]));
    assert_eq!(
        replaced["quarantine"],
        json!([finding_at(&project, "02-b-web")?])
    );
    assert_eq!(spawns_of(&plan)?, ["02-a-web", "03-b-web"]);
    assert_eq!(plan["batch"], json!(0));
    Ok(())
}

#[test]
fn a_quarantine_never_strands_an_offered_spawn() -> Result<(), TestError> {
    let project = sibling_answered("replaced-stranded")?;
    let one_at_a_time = ["--concurrency", "1"];
    let (plan, run) = started(&project, &one_at_a_time)?;
    assert_eq!(spawns_of(&plan)?, ["02-a-web"]);
    quarantine_as_the_report_would(&plan)?;
    write_finding(
        &project,
        "02-a-web",
        &finding("02-a-web", "A?", "depth: 1\n"),
    )?;

    let next =
        continued(&project, &run, &["--concurrency", "1", "--spawned", "0"])?;

    assert_eq!(spawns_of(&next)?, ["03-b-web"]);
    assert_eq!(next["unfinished"], json!([]));
    Ok(())
}

fn failing_validation_still_naming_its_question(
    tag: &str,
) -> Result<Project, TestError> {
    let project = small_set(tag, "- [ ] A?\n- [ ] B?")?;
    started(&project, &[])?;
    write_finding(&project, "01-a-web", FAILING_VALIDATION)?;
    Ok(project)
}

#[test]
fn the_first_plan_after_a_replaced_batch_matches_the_plan_once_quarantined(
) -> Result<(), TestError> {
    let left_invalid = [
        sibling_answered("replaced-equivalent-misdirected")?,
        failing_validation_still_naming_its_question(
            "replaced-equivalent-failing",
        )?,
    ];
    for project in left_invalid {
        let (plan, run) = started(&project, &[])?;
        quarantine_as_the_report_would(&plan)?;

        let continued = continued(&project, &run, &[])?;
        let (unclaimed, _) = project.plan_with(&[])?;

        for quarantined in [continued, unclaimed] {
            assert_eq!(quarantined["items"], plan["items"]);
            assert_eq!(
                unbudgeted_pairs(&quarantined)?,
                unbudgeted_pairs(&plan)?
            );
        }
    }
    Ok(())
}

fn unbudgeted_pairs(plan: &Value) -> Result<Vec<Value>, TestError> {
    let mut pairs = plan["pairs"].as_array().ok_or("no pairs")?.clone();
    for pair in &mut pairs {
        pair.as_object_mut()
            .ok_or("not a pair")?
            .remove("search_budget");
    }
    Ok(pairs)
}

#[test]
fn quarantined_leftovers_keep_their_planned_stems_for_the_rest_of_the_run(
) -> Result<(), TestError> {
    let project = small_set("replaced-fresh", "- [ ] A?\n- [ ] B?")?;
    started(&project, &[])?;
    write_finding(&project, "01-a-web", FAILING_VALIDATION)?;
    write_finding(
        &project,
        "02-b-web",
        &finding("02-b-web", "Z?", "depth: 1\n"),
    )?;
    let (plan, run) = started(&project, &[])?;
    quarantine_as_the_report_would(&plan)?;

    let next = continued(&project, &run, &[])?;
    let (query, _) = project.plan_with(&[])?;

    assert_eq!(spawns_of(&next)?, ["01-a-web", "03-b-web"]);
    assert_eq!(next.get("replaced_run"), None);
    assert_eq!(query["warnings"], json!([]));
    assert_eq!(
        query["items"],
        json!([item(6, "A?", false), item(7, "B?", false)])
    );
    Ok(())
}

fn left_invalid_by_a_replaced_batch(tag: &str) -> Result<Project, TestError> {
    let project = small_set(tag, "- [ ] A?\n- [ ] B?")?;
    started(&project, &[])?;
    write_finding(&project, "01-a-web", FAILING_VALIDATION)?;
    write_finding(
        &project,
        "02-b-web",
        &finding("02-b-web", "Z?", "depth: 1\n"),
    )?;
    Ok(project)
}

#[test]
fn a_second_start_before_the_renames_still_quarantines_the_first_starts_findings(
) -> Result<(), TestError> {
    let project = left_invalid_by_a_replaced_batch("replaced-twice")?;
    let (recovering, first) = started(&project, &[])?;

    let (plan, _) = started(&project, &[])?;

    let replaced = &plan["replaced_run"];
    assert_eq!(replaced["run"], json!(first));
    assert_eq!(replaced["owed"], json!(["02-b-web"]));
    assert_eq!(
        replaced["quarantine"],
        json!([
            finding_at(&project, "01-a-web")?,
            finding_at(&project, "02-b-web")?,
        ])
    );
    assert_eq!(replaced["invalid"], json!([left("01-a-web", "A?")]));
    assert_eq!(replaced["unwritten"], json!([left("03-b-web", "B?")]));
    assert_eq!(plan["pairs"], recovering["pairs"]);
    Ok(())
}

fn quarantine_owed(run: &str, paths: &[String]) -> String {
    format!(
        "E_TOPIC_RESEARCH_QUARANTINE_OWED: run {run} still owes the \
         quarantine of {}; rename each to .<name>.invalid beside it, never \
         deleting it, then re-run conduct",
        paths.join(", ")
    )
}

#[test]
fn continuing_a_run_before_its_owed_renames_exits_1_naming_each_path(
) -> Result<(), TestError> {
    let project = left_invalid_by_a_replaced_batch("replaced-owed")?;
    let (_, run) = started(&project, &[])?;
    let findings = project.set("s").join("findings");
    fs::rename(
        findings.join("01-a-web.md"),
        findings.join(".01-a-web.md.invalid"),
    )?;
    let before = fs::read(project.ledger())?;

    let output =
        project.outstanding_with(&["--run", &run, "--spawned", "0"])?;

    assert_exits_1(
        &output,
        &quarantine_owed(&run, &[finding_at(&project, "02-b-web")?]),
    );
    assert_eq!(fs::read(project.ledger())?, before);
    Ok(())
}

#[test]
fn a_run_owes_nothing_once_its_quarantine_is_renamed() -> Result<(), TestError>
{
    let project = left_invalid_by_a_replaced_batch("replaced-settled")?;
    let (plan, run) = started(&project, &[])?;
    let stored: Value =
        serde_json::from_str(&fs::read_to_string(project.ledger())?)?;
    assert_eq!(stored["owed_quarantine"], json!(["02-b-web"]));
    quarantine_as_the_report_would(&plan)?;

    continued(&project, &run, &[])?;
    let (restarted, _) = started(&project, &[])?;

    let stored: Value =
        serde_json::from_str(&fs::read_to_string(project.ledger())?)?;
    assert_eq!(stored["owed_quarantine"], json!([]));
    assert_eq!(restarted["replaced_run"]["owed"], json!([]));
    assert_eq!(restarted["replaced_run"]["quarantine"], json!([]));
    Ok(())
}

#[test]
fn a_stem_the_recovering_plan_offers_again_is_not_owed() -> Result<(), TestError>
{
    let project = small_set("replaced-reoffered", "- [ ] A?")?;
    started(&project, &[])?;
    write_finding(
        &project,
        "01-a-web",
        &finding("01-a-web", "A?", "depth: 1\n")
            .replace("\"web\"", "\"openalex\""),
    )?;
    let (plan, run) = started(&project, &[])?;
    assert_eq!(spawns_of(&plan)?, ["01-a-web"]);
    quarantine_as_the_report_would(&plan)?;
    write_finding(
        &project,
        "01-a-web",
        &finding("01-a-web", "A?", "depth: 1\n"),
    )?;

    let next = continued(&project, &run, &["--spawned", "0"])?;

    assert_eq!(next["pairs"], json!([]));
    assert_eq!(next["items"], json!([item(6, "A?", true)]));
    Ok(())
}

#[test]
fn a_finding_whose_question_left_the_outline_is_left_in_place(
) -> Result<(), TestError> {
    let project = small_set("replaced-reworded", "- [ ] A?")?;
    started(&project, &[])?;
    write_finding(
        &project,
        "01-a-web",
        &finding("01-a-web", "A?", "depth: 1\n"),
    )?;
    project.write(&format!("{TOPICS}/s/outline.md"), &outline("- [ ] A2?"))?;

    let (plan, _) = started(&project, &[])?;

    assert_eq!(
        plan["replaced_run"]["unasked"],
        json!([left("01-a-web", "A?")])
    );
    assert_eq!(plan["replaced_run"]["quarantine"], json!([]));
    Ok(())
}

#[test]
fn a_finding_whose_question_was_reworded_to_the_same_normalised_form_is_kept(
) -> Result<(), TestError> {
    let project = small_set("replaced-refolded", "- [ ] A?")?;
    started(&project, &[])?;
    write_finding(
        &project,
        "01-a-web",
        &finding("01-a-web", "A?", "depth: 1\n"),
    )?;
    project.write(
        &format!("{TOPICS}/s/outline.md"),
        &outline("- [ ] \u{FF21}?"),
    )?;

    let (plan, _) = started(&project, &[])?;

    assert_eq!(
        plan["replaced_run"]["valid"],
        json!([left("01-a-web", "\u{FF21}?")])
    );
    assert_eq!(plan["replaced_run"]["quarantine"], json!([]));
    Ok(())
}

#[test]
fn a_node_under_a_root_marker_naming_another_question_is_unwritten(
) -> Result<(), TestError> {
    let project = small_set("replaced-marked-root", "- [ ] A?")?;
    let args = ["--depth", "2"];
    let (offered, _) = started(&project, &args)?;
    assert_eq!(spawns_of(&offered)?, ["01-a-web:1"]);
    project.write(
        &format!("{TOPICS}/s/findings/01-a-web.levels/.1.md.invalid"),
        &level_note_with(
            "01-a-web",
            "1",
            "question: \"Other?\"\nlevel: 1\nfollow_ups: []\n",
        ),
    )?;

    let (plan, _) = started(&project, &args)?;

    assert_eq!(
        plan["replaced_run"]["unwritten"],
        json!([left("01-a-web:1", "A?")])
    );
    assert_eq!(plan["replaced_run"]["valid"], json!([]));
    Ok(())
}

#[test]
fn an_unwritten_spawn_whose_question_left_the_outline_names_no_pair(
) -> Result<(), TestError> {
    let project = small_set("replaced-unasked", "- [ ] A?")?;
    started(&project, &[])?;
    project.write(&format!("{TOPICS}/s/outline.md"), &outline("- [ ] A2?"))?;

    let (plan, _) = started(&project, &[])?;

    assert_eq!(
        plan["replaced_run"]["unasked"],
        json!([{"spawn": "01-a-web", "question": null, "profile": null}])
    );
    Ok(())
}

#[test]
fn a_valid_finding_from_a_replaced_batch_counts_its_pair_answered(
) -> Result<(), TestError> {
    let project = a_set()?;
    started(&project, &[])?;
    write_finding(
        &project,
        "01-a-web",
        &finding("01-a-web", "A?", "depth: 1\n"),
    )?;

    let (plan, _) = started(&project, &[])?;

    assert_eq!(
        plan["replaced_run"]["valid"],
        json!([left("01-a-web", "A?")])
    );
    assert_eq!(plan["pairs"], json!([]));
    assert_eq!(plan["items"][0]["complete"], json!(true));
    Ok(())
}

fn nothing_left(run: &str) -> Value {
    json!({
        "run": run,
        "valid": [],
        "invalid": [],
        "unwritten": [],
        "unasked": [],
        "quarantine": [],
        "owed": [],
    })
}

#[test]
fn a_stale_ledger_with_an_empty_last_batch_still_names_its_run(
) -> Result<(), TestError> {
    let project = a_set()?;
    write_finding(
        &project,
        "01-a-web",
        &finding("01-a-web", "A?", "depth: 1\n"),
    )?;
    let (_, first) = started(&project, &[])?;
    let stored: Value =
        serde_json::from_str(&fs::read_to_string(project.ledger())?)?;
    assert_eq!(stored["pending"], json!({"number": 0, "spawns": []}));

    let (plan, _) = started(&project, &[])?;

    assert_eq!(plan["replaced_run"], nothing_left(&first));
    Ok(())
}

#[test]
fn a_stale_ledger_with_fields_it_does_not_know_still_reports_its_last_batch(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, first) = started(&project, &[])?;
    let mut stored: Value =
        serde_json::from_str(&fs::read_to_string(project.ledger())?)?;
    stored["future"] = json!({"anything": 1});
    stored["pending"]["future"] = json!(true);
    fs::write(project.ledger(), stored.to_string())?;

    let (plan, _) = started(&project, &[])?;

    assert_eq!(plan["replaced_run"]["run"], json!(first));
    assert_eq!(
        plan["replaced_run"]["unwritten"],
        json!([left("01-a-web", "A?")])
    );
    Ok(())
}

#[test]
fn a_ledger_without_budgets_still_yields_a_replaced_run_report(
) -> Result<(), TestError> {
    let project = small_set("replaced-literal", "- [ ] A?\n- [ ] B?")?;
    write_finding(
        &project,
        "02-b-web",
        &finding("02-b-web", "B?", "depth: 1\n"),
    )?;
    fs::write(
        project.ledger(),
        r#"{
  "run": "earlier-run",
  "pending": {
    "number": 3,
    "spawns": ["01-a-web", "02-b-web"]
  },
  "claims": {
    "A?": 1,
    "B?": 2
  },
  "attempted": [],
  "just_acknowledged": [],
  "seen": {
    "notes": {},
    "answered": []
  }
}
"#,
    )?;

    let (plan, _) = started(&project, &[])?;

    assert_eq!(
        plan["replaced_run"],
        json!({
            "run": "earlier-run",
            "valid": [left("02-b-web", "B?")],
            "invalid": [],
            "unwritten": [left("01-a-web", "A?")],
            "unasked": [],
            "quarantine": [],
            "owed": [],
        })
    );
    Ok(())
}

#[test]
fn no_replaced_run_without_a_ledger_to_replace() -> Result<(), TestError> {
    let project = a_set()?;

    let (first, run) = started(&project, &[])?;
    let ended = project.run(&["end-run", "s", "--run", &run])?;
    assert!(ended.status.success());
    let (after_end, _) = started(&project, &[])?;

    assert_eq!(first.get("replaced_run"), None);
    assert_eq!(after_end.get("replaced_run"), None);
    Ok(())
}

#[test]
fn no_replaced_run_on_a_plain_or_continued_plan() -> Result<(), TestError> {
    let project = a_set()?;
    let (_, run) = started(&project, &[])?;

    let (plain, _) = project.plan_with(&[])?;
    let next = continued(&project, &run, &[])?;

    assert_eq!(plain.get("replaced_run"), None);
    assert_eq!(next.get("replaced_run"), None);
    Ok(())
}

#[test]
fn outline_and_counts_match_disk_after_a_replaced_batch(
) -> Result<(), TestError> {
    let project = small_set("replaced-tick", "- [ ] A?\n- [ ] B?")?;
    started(&project, &[])?;
    write_finding(
        &project,
        "01-a-web",
        &finding("01-a-web", "A?", "depth: 1\n"),
    )?;
    let (_, run) = started(&project, &[])?;
    let ended = project.run(&["end-run", "s", "--run", &run])?;
    assert!(ended.status.success());

    let (plan, _) = project.plan_with(&[])?;

    assert_eq!(
        plan["items"],
        json!([item(6, "A?", true), item(7, "B?", false)])
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
fn an_unknown_set_is_reported_before_an_empty_concurrency(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.configure_team_concurrency("\"\"")?;

    let output = project.outstanding("no-such-set", &installed_profiles())?;

    assert_exits_1(
        &output,
        "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set 'no-such-set'",
    );
    Ok(())
}

#[test]
fn concurrency_warnings_precede_a_corrupt_ledger_notice(
) -> Result<(), TestError> {
    let project = a_set()?;
    project.configure_team_concurrency("24")?;
    fs::write(project.ledger(), "{")?;

    let (plan, stderr) = project.plan_with(&["--start"])?;

    assert_warned(
        &plan,
        &stderr,
        &[
            clamped_to_the_default(TEAM_KNOB, "24"),
            CORRUPT_LEDGER.to_owned(),
        ],
    );
    Ok(())
}

#[test]
fn stderr_orders_a_clamp_a_replaced_ledger_and_a_trim() -> Result<(), TestError>
{
    let project = a_set()?;
    project.configure_team_concurrency("24")?;
    project.write_note(
        "01-a-web",
        "1",
        "A?",
        &["B?", "C?", "D?", "E?", "F?"],
    )?;
    let (_, first) = started(&project, &["--depth", "2"])?;

    let (plan, stderr) = project.plan_with(&["--start", "--depth", "2"])?;

    let clamp = clamped_to_the_default(TEAM_KNOB, "24");
    assert_eq!(plan["warnings"], json!([clamp]));
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        [
            format!("warning: {clamp}"),
            format!("warning: {}", replaced_notice(&first)),
            "warning: level note 01-a-web.levels/1 records 5 follow-ups, \
             over its cap of 4; trimmed 1"
                .to_owned(),
        ]
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

fn unhonoured_search_cap(value: &str) -> String {
    format!(
        "{SEARCH_CAP} is '{value}', not a positive integer — ignoring it; \
         sizing search budgets against the search cap of 200 from the default"
    )
}

fn assert_no_search_cap_notice(plan: &Value, stderr: &str) {
    assert!(!plan["warnings"].to_string().contains(SEARCH_CAP), "{plan}");
    assert!(!stderr.contains(SEARCH_CAP), "{stderr}");
}

fn openalex_and_arxiv_set(tag: &str) -> Result<Project, TestError> {
    let project = small_set(
        tag,
        "- [ ] A? — profiles: openalex\n- [ ] B? — profiles: arxiv",
    )?;
    project.write(
        &format!("{TOPICS}/s/brief.md"),
        "---\ntype: \"topic-research\"\nkind: \"brief\"\n\
         source_profiles: [\"openalex\", \"arxiv\"]\n---\n",
    )?;
    Ok(project)
}

#[test]
fn every_plan_reports_the_search_cap_and_its_source() -> Result<(), TestError> {
    let rows: [(&[(&str, &str)], Value); 2] = [
        (&[], json!({"searches": 200, "source": "default"})),
        (
            &[(SEARCH_CAP, "500")],
            json!({"searches": 500, "source": SEARCH_CAP}),
        ),
    ];
    for (variables, reported) in rows {
        let project = a_set()?;

        let (plan, _) = project.plan_in(&[], variables)?;

        assert_eq!(plan["search_cap"], reported, "{variables:?}");
    }
    Ok(())
}

#[test]
fn an_unhonoured_search_cap_is_reported_when_a_run_starts_with_web_pairs(
) -> Result<(), TestError> {
    let project = a_set()?;

    let (plan, stderr) =
        project.plan_in(&["--start"], &[(SEARCH_CAP, "abc")])?;

    assert_eq!(
        plan["search_cap"],
        json!({"searches": 200, "source": "default"})
    );
    assert_warned(&plan, &stderr, &[unhonoured_search_cap("abc")]);
    Ok(())
}

#[test]
fn a_zero_search_cap_is_unhonoured() -> Result<(), TestError> {
    let project = a_set()?;

    let (plan, stderr) = project.plan_in(&["--start"], &[(SEARCH_CAP, "0")])?;

    assert_warned(&plan, &stderr, &[unhonoured_search_cap("0")]);
    Ok(())
}

#[test]
fn an_unhonoured_search_cap_is_not_reported_after_the_start(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (first, _) = project.plan_in(&["--start"], &[(SEARCH_CAP, "abc")])?;
    let run = first["run"].as_str().ok_or("no run")?;

    let (plan, stderr) = project
        .plan_in(&["--run", run, "--spawned", "0"], &[(SEARCH_CAP, "abc")])?;

    assert_no_search_cap_notice(&plan, &stderr);
    Ok(())
}

#[test]
fn a_run_without_web_pairs_reports_no_search_cap_notice(
) -> Result<(), TestError> {
    let project = openalex_and_arxiv_set("no-web-pairs")?;

    let (plan, stderr) =
        project.plan_in(&["--start"], &[(SEARCH_CAP, "abc")])?;

    assert_eq!(plan["pairs"].as_array().map(Vec::len), Some(2), "{plan}");
    assert_no_search_cap_notice(&plan, &stderr);
    Ok(())
}

#[test]
fn start_notices_keep_their_order() -> Result<(), TestError> {
    let project = web_set("notice-order", 20, 0, 0)?;
    fs::write(project.ledger(), "{")?;

    let (plan, stderr) = project.plan_in(
        &["--concurrency", "24", "--start", "--depth", "3"],
        &[(ACCELERATOR_CAP, "8"), (SEARCH_CAP, "abc")],
    )?;

    assert_warned(
        &plan,
        &stderr,
        &[
            clamped_by(FLAG, "24", 8, ACCELERATOR_CAP),
            CORRUPT_LEDGER.to_owned(),
            unhonoured_search_cap("abc"),
            floor_warning(260, 190, 200, "the default"),
        ],
    );
    Ok(())
}

#[test]
fn a_start_that_cannot_plan_still_reports_its_concurrency_notices(
) -> Result<(), TestError> {
    let project = a_set()?;
    let missing = project.root.join("no-profiles");

    let output = project.run_in(
        &[
            "outstanding",
            "s",
            "--profiles-dir",
            missing.to_str().ok_or("non-utf8")?,
            "--concurrency",
            "24",
            "--start",
        ],
        &[(ACCELERATOR_CAP, "8")],
    )?;

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "warning: {}",
            clamped_by(FLAG, "24", 8, ACCELERATOR_CAP)
        )),
        "{stderr}"
    );
    assert!(stderr.contains("no profiles directory at"), "{stderr}");
    assert!(output.stdout.is_empty());
    Ok(())
}

fn web_set(
    tag: &str,
    web_pairs: usize,
    openalex_pairs: usize,
    arxiv_pairs: usize,
) -> Result<Project, TestError> {
    let groups = [
        ("web", web_pairs),
        ("openalex", openalex_pairs),
        ("arxiv", arxiv_pairs),
    ];
    let items: Vec<String> = groups
        .iter()
        .flat_map(|(profile, pairs)| {
            (1..=*pairs).map(move |n| {
                format!("- [ ] {profile} question {n}? — profiles: {profile}")
            })
        })
        .collect();
    let profiles: Vec<&str> = groups
        .iter()
        .filter(|(_, pairs)| *pairs > 0)
        .map(|(profile, _)| *profile)
        .collect();
    let project = small_set(tag, &items.join("\n"))?;
    project.write(
        &format!("{TOPICS}/s/brief.md"),
        &format!(
            "---\ntype: \"topic-research\"\nkind: \"brief\"\n\
             source_profiles: {}\n---\n",
            json!(profiles)
        ),
    )?;
    Ok(project)
}

fn floor_warning(
    demand: u64,
    available: u32,
    cap: u32,
    source: &str,
) -> String {
    format!(
        "the worst-case demand of {demand} web searches exceeds the \
         {available} left by the search cap of {cap} from {source} after a \
         reserve of {} — every search budget is 1, so expect a halt, then \
         run /clear and re-run conduct; for later runs, raise {SEARCH_CAP} \
         and restart Claude Code, or lower the depth",
        cap - available
    )
}

type Budgets = &'static [(u32, u32)];
type Variables = &'static [(&'static str, &'static str)];

fn budgets(levels: &[(u32, u32)]) -> Value {
    levels
        .iter()
        .map(|(level, budget)| json!({"level": level, "budget": budget}))
        .collect()
}

fn assert_budgeted(plan: &Value, levels: &[(u32, u32)], demand: u64) {
    assert_eq!(plan["search_budgets"], budgets(levels), "{plan}");
    assert_eq!(plan["search_demand"], json!(demand), "{plan}");
}

fn assert_unbudgeted(plan: &Value) {
    assert_eq!(plan.get("search_budgets"), None, "{plan}");
    assert_eq!(plan.get("search_demand"), None, "{plan}");
    assert!(!plan.to_string().contains("search_budget"), "{plan}");
}

fn budgeted_start(
    project: &Project,
    depth: &str,
    variables: &[(&str, &str)],
) -> Result<(Value, String), TestError> {
    project.plan_in(&["--start", "--depth", depth], variables)
}

fn stored_ledger(project: &Project) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&fs::read_to_string(
        project.ledger(),
    )?)?)
}

#[test]
fn a_started_run_sizes_search_budgets_over_every_web_pair(
) -> Result<(), TestError> {
    let project = web_set("budget-ten", 10, 0, 0)?;

    let (plan, stderr) = budgeted_start(&project, "3", &[])?;

    assert_budgeted(&plan, &[(1, 3), (2, 2), (3, 1)], 190);
    assert_warned(&plan, &stderr, &[]);
    Ok(())
}

#[test]
fn budgets_grow_for_few_pairs() -> Result<(), TestError> {
    let project = web_set("budget-three", 3, 0, 0)?;

    let (plan, _) = budgeted_start(&project, "3", &[])?;

    assert_budgeted(&plan, &[(1, 15), (2, 6), (3, 3)], 189);
    Ok(())
}

#[test]
fn a_raised_cap_raises_budgets() -> Result<(), TestError> {
    let project = web_set("budget-raised", 10, 0, 0)?;

    let (plan, _) = budgeted_start(&project, "3", &[(SEARCH_CAP, "500")])?;

    assert_budgeted(&plan, &[(1, 11), (2, 5), (3, 2)], 470);
    assert_eq!(
        plan["search_cap"],
        json!({"searches": 500, "source": SEARCH_CAP})
    );
    Ok(())
}

#[test]
fn deep_runs_keep_deeper_levels_at_or_below_shallower_ones(
) -> Result<(), TestError> {
    let rows: [(usize, &str, Variables, Budgets, u64); 3] = [
        (
            10,
            "5",
            &[(SEARCH_CAP, "500")],
            &[(1, 7), (2, 4), (3, 1), (4, 1), (5, 1)],
            470,
        ),
        (10, "2", &[], &[(1, 7), (2, 3)], 190),
        (2, "5", &[], &[(1, 11), (2, 5), (3, 3), (4, 3), (5, 2)], 190),
    ];
    for (pairs, depth, variables, levels, demand) in rows {
        let project = web_set("budget-deep", pairs, 0, 0)?;

        let (plan, _) = budgeted_start(&project, depth, variables)?;

        assert_budgeted(&plan, levels, demand);
    }
    Ok(())
}

#[test]
fn budgets_of_one_that_overrun_warn_once() -> Result<(), TestError> {
    let project = web_set("budget-floor", 20, 0, 0)?;

    let (plan, stderr) = budgeted_start(&project, "3", &[])?;

    assert_budgeted(&plan, &[(1, 1), (2, 1), (3, 1)], 260);
    assert_warned(
        &plan,
        &stderr,
        &[floor_warning(260, 190, 200, "the default")],
    );
    assert_eq!(spawns_of(&plan)?.len(), 20);
    let run = plan["run"].as_str().ok_or("no run")?;

    let (next, stderr) =
        project.plan_with(&["--run", run, "--spawned", "0", "--depth", "3"])?;

    assert_warned(&next, &stderr, &[]);
    Ok(())
}

#[test]
fn the_reserve_alone_can_force_the_floor() -> Result<(), TestError> {
    let project = web_set("budget-reserve-floor", 15, 0, 0)?;

    let (plan, stderr) = budgeted_start(&project, "3", &[])?;

    assert_budgeted(&plan, &[(1, 1), (2, 1), (3, 1)], 195);
    assert_warned(
        &plan,
        &stderr,
        &[floor_warning(195, 190, 200, "the default")],
    );
    Ok(())
}

#[test]
fn a_cap_whose_reserve_still_fits_ones_does_not_warn() -> Result<(), TestError>
{
    let project = web_set("budget-reserve-fits", 14, 0, 0)?;

    let (plan, stderr) = budgeted_start(&project, "3", &[(SEARCH_CAP, "192")])?;

    assert_budgeted(&plan, &[(1, 1), (2, 1), (3, 1)], 182);
    assert_warned(&plan, &stderr, &[]);
    Ok(())
}

#[test]
fn non_web_pairs_add_no_demand() -> Result<(), TestError> {
    let project = web_set("budget-mixed", 8, 2, 0)?;

    let (plan, _) = budgeted_start(&project, "3", &[])?;

    assert_budgeted(&plan, &[(1, 7), (2, 2), (3, 1)], 184);
    for pair in plan["pairs"].as_array().ok_or("no pairs")? {
        if pair["profile"] == json!("openalex") {
            for node in nodes_of(pair)? {
                assert_eq!(node.get("search_budget"), None, "{node}");
            }
        }
    }
    Ok(())
}

#[test]
fn an_unhonoured_cap_sizes_against_the_default() -> Result<(), TestError> {
    let project = web_set("budget-unhonoured", 1, 0, 0)?;
    let unhonoured = [(SEARCH_CAP, "abc")];

    let (plan, stderr) = budgeted_start(&project, "3", &unhonoured)?;

    assert_budgeted(&plan, &[(1, 42), (2, 19), (3, 9)], 190);
    assert_warned(&plan, &stderr, &[unhonoured_search_cap("abc")]);
    let run = plan["run"].as_str().ok_or("no run")?;

    let (next, stderr) = project.plan_in(
        &["--run", run, "--spawned", "0", "--depth", "3"],
        &unhonoured,
    )?;

    assert_warned(&next, &stderr, &[]);
    Ok(())
}

#[test]
fn an_unhonoured_zero_cap_warns_against_the_default() -> Result<(), TestError> {
    let project = web_set("budget-zero", 20, 0, 0)?;

    let (plan, stderr) = budgeted_start(&project, "3", &[(SEARCH_CAP, "0")])?;

    assert_warned(
        &plan,
        &stderr,
        &[
            unhonoured_search_cap("0"),
            floor_warning(260, 190, 200, "the default"),
        ],
    );
    Ok(())
}

#[test]
fn a_run_without_web_pairs_sizes_nothing() -> Result<(), TestError> {
    let project = openalex_and_arxiv_set("budget-no-web")?;
    let unhonoured = [(SEARCH_CAP, "abc")];

    let (plan, stderr) = budgeted_start(&project, "3", &unhonoured)?;
    let run = plan["run"].as_str().ok_or("no run")?;
    let (next, next_stderr) = project.plan_in(
        &["--run", run, "--spawned", "0", "--depth", "3"],
        &unhonoured,
    )?;

    for (plan, stderr) in [(&plan, &stderr), (&next, &next_stderr)] {
        assert_unbudgeted(plan);
        assert_warned(plan, stderr, &[]);
    }
    assert_eq!(
        stored_ledger(&project)?["search_budgets"],
        json!({"levels": {}, "demand": 0})
    );
    Ok(())
}

#[test]
fn a_re_run_sizes_only_the_remaining_tree() -> Result<(), TestError> {
    let project = web_set("budget-re-run", 1, 0, 0)?;
    let stem = "01-web-question-1-web";
    project.write_note(
        stem,
        "1",
        "web question 1?",
        &["B?", "C?", "D?", "E?"],
    )?;
    project.write_note(stem, "2-1", "B?", &["X?"])?;
    project.write_note(stem, "2-2", "C?", &["Y?", "Z?"])?;

    let (plan, _) = budgeted_start(&project, "3", &[])?;

    assert_budgeted(&plan, &[(2, 35), (3, 17)], 189);
    Ok(())
}

#[test]
fn the_reserve_rounds_up() -> Result<(), TestError> {
    let rows: [(&str, Budgets, u64); 2] = [
        ("201", &[(1, 42), (2, 19), (3, 9)], 190),
        ("60", &[(1, 13), (2, 5), (3, 3)], 57),
    ];
    for (cap, levels, demand) in rows {
        let project = web_set("budget-rounding", 1, 0, 0)?;

        let (plan, _) = budgeted_start(&project, "3", &[(SEARCH_CAP, cap)])?;

        assert_budgeted(&plan, levels, demand);
    }
    Ok(())
}

#[test]
fn a_single_pass_run_budgets_its_one_level() -> Result<(), TestError> {
    let project = web_set("budget-single-pass", 10, 0, 0)?;

    let (plan, _) = budgeted_start(&project, "1", &[])?;

    assert_budgeted(&plan, &[(1, 19)], 190);
    let pairs = plan["pairs"].as_array().ok_or("no pairs")?;
    assert_eq!(pairs.len(), 10);
    for pair in pairs {
        assert_eq!(pair["stage"], json!("single_pass"), "{pair}");
        assert_eq!(pair["search_budget"], json!(19), "{pair}");
    }
    Ok(())
}

const LEDGER_BEFORE_BUDGETS: &str = r#"{
  "run": "earlier-run",
  "pending": {"number": 0, "spawns": []},
  "claims": {},
  "attempted": [],
  "just_acknowledged": [],
  "seen": {"notes": {}, "answered": []}
}
"#;

#[test]
fn a_ledger_without_budgets_is_sized_by_the_next_plan() -> Result<(), TestError>
{
    let project = web_set("budget-pre-ledger", 20, 0, 0)?;
    fs::write(project.ledger(), LEDGER_BEFORE_BUDGETS)?;

    let (plan, stderr) =
        project.plan_with(&["--run", "earlier-run", "--depth", "3"])?;

    assert_budgeted(&plan, &[(1, 1), (2, 1), (3, 1)], 260);
    assert_warned(
        &plan,
        &stderr,
        &[floor_warning(260, 190, 200, "the default")],
    );
    assert_eq!(
        stored_ledger(&project)?["search_budgets"],
        json!({"levels": {"1": 1, "2": 1, "3": 1}, "demand": 260})
    );

    let (next, stderr) = project.plan_with(&[
        "--run",
        "earlier-run",
        "--spawned",
        "0",
        "--depth",
        "3",
    ])?;

    assert_warned(&next, &stderr, &[]);
    assert_budgeted(&next, &[(1, 1), (2, 1), (3, 1)], 260);
    Ok(())
}

#[test]
fn later_plans_keep_the_stored_budgets() -> Result<(), TestError> {
    let project = web_set("budget-kept", 10, 0, 0)?;
    let (plan, _) = budgeted_start(&project, "3", &[])?;
    let run = plan["run"].as_str().ok_or("no run")?;
    for n in 1..=10 {
        project.write_note(
            &format!("{n:02}-web-question-{n}-web"),
            "1",
            &format!("web question {n}?"),
            &[&format!("Follow-up {n}?")],
        )?;
    }

    let next = continued(&project, run, &["--spawned", "0", "--depth", "3"])?;

    assert_budgeted(&next, &[(1, 3), (2, 2), (3, 1)], 190);
    assert_eq!(
        stored_ledger(&project)?["search_budgets"],
        json!({"levels": {"1": 3, "2": 2, "3": 1}, "demand": 190})
    );
    let nodes = every_node(&next)?;
    assert_eq!(nodes.len(), 10);
    for node in nodes {
        assert_eq!(node["level"], json!(2), "{node}");
        assert_eq!(node["search_budget"], json!(2), "{node}");
    }
    Ok(())
}

#[test]
fn every_web_spawn_carries_its_budget() -> Result<(), TestError> {
    let project = web_set("budget-per-spawn", 2, 1, 0)?;
    project.write_note(
        "01-web-question-1-web",
        "1",
        "web question 1?",
        &["B?", "C?"],
    )?;

    let (plan, _) = budgeted_start(&project, "2", &[])?;

    let budget_at = |level: &Value| {
        plan["search_budgets"]
            .as_array()
            .and_then(|levels| {
                levels.iter().find(|entry| &entry["level"] == level)
            })
            .map(|entry| entry["budget"].clone())
    };
    let mut web_levels = Vec::new();
    for pair in plan["pairs"].as_array().ok_or("no pairs")? {
        for node in nodes_of(pair)? {
            if pair["profile"] == json!("web") {
                web_levels.push(node["level"].clone());
                assert_eq!(
                    Some(node["search_budget"].clone()),
                    budget_at(&node["level"]),
                    "{node}"
                );
            } else {
                assert_eq!(node.get("search_budget"), None, "{node}");
            }
        }
    }
    assert_eq!(web_levels, [json!(2), json!(2), json!(1)]);
    Ok(())
}

#[test]
fn a_level_note_with_a_search_budget_is_read_like_any_other(
) -> Result<(), TestError> {
    let project = web_set("budget-stamped", 1, 0, 0)?;
    let stem = "01-web-question-1-web";
    project.write(
        &note_path(stem, "1"),
        &level_note_with(
            stem,
            "1",
            "question: \"web question 1?\"\nlevel: 1\nsearch_budget: 3\n\
             follow_ups: [\"B?\", \"C?\"]\n",
        ),
    )?;

    let (plan, _) = budgeted_start(&project, "2", &[])?;

    assert_eq!(lineages_of(&plan)?, ["2-1", "2-2"]);
    Ok(())
}

#[test]
fn query_plans_carry_no_budgets() -> Result<(), TestError> {
    let project = web_set("budget-query", 1, 0, 0)?;

    let (plan, _) = project.plan_with(&["--depth", "3"])?;

    assert_eq!(spawns_of(&plan)?.len(), 1);
    assert_unbudgeted(&plan);
    Ok(())
}

const WITHHOLD: &str = "--withhold-profile";

fn halting_set(tag: &str, items: &str) -> Result<Project, TestError> {
    let project = small_set(tag, items)?;
    project.write(
        &format!("{TOPICS}/s/brief.md"),
        "---\ntype: \"topic-research\"\nkind: \"brief\"\n\
         source_profiles: [\"web\", \"openalex\", \"arxiv\"]\n---\n",
    )?;
    Ok(project)
}

const OPENALEX_THEN_TWO_WEB: &str = "- [ ] A? — profiles: openalex\n\
                                     - [ ] B? — profiles: web\n\
                                     - [ ] C? — profiles: web";

fn attempted(project: &Project) -> Result<Vec<String>, TestError> {
    Ok(stored_ledger(project)?["attempted"]
        .as_array()
        .ok_or("no attempted")?
        .iter()
        .filter_map(|spawn| spawn.as_str().map(str::to_owned))
        .collect())
}

fn assert_withholds(
    project: &Project,
    plan: &Value,
    profiles: &[&str],
) -> Result<(), TestError> {
    for profile in profiles {
        let suffix = format!("-{profile}");
        assert!(
            !spawns_of(plan)?.iter().any(|spawn| spawn.contains(&suffix)),
            "{plan}"
        );
        assert!(
            !attempted(project)?
                .iter()
                .any(|spawn| spawn.contains(&suffix)),
            "{:?}",
            attempted(project)?
        );
    }
    assert_eq!(plan["remaining"], json!(0), "{plan}");
    assert_eq!(plan["withheld"], json!(profiles), "{plan}");
    assert_eq!(stored_ledger(project)?["withheld"], json!(profiles));
    Ok(())
}

#[test]
fn a_withheld_profile_is_not_offered_and_its_spawns_stay_unattempted(
) -> Result<(), TestError> {
    let project = halting_set("withheld", OPENALEX_THEN_TWO_WEB)?;
    let depth = ["--depth", "2", "--concurrency", "1"];
    let (first, run) = started(&project, &depth)?;
    assert_eq!(spawns_of(&first)?, ["01-a-openalex:1"]);

    let plan = continued(
        &project,
        &run,
        &[&depth[..], &["--spawned", "0", WITHHOLD, "web"]].concat(),
    )?;

    assert_withholds(&project, &plan, &["web"])
}

#[test]
fn a_withheld_profile_stays_withheld_without_the_flag() -> Result<(), TestError>
{
    let project = halting_set("still-withheld", OPENALEX_THEN_TWO_WEB)?;
    let depth = ["--depth", "2", "--concurrency", "1"];
    let (_, run) = started(&project, &depth)?;
    continued(
        &project,
        &run,
        &[&depth[..], &["--spawned", "0", WITHHOLD, "web"]].concat(),
    )?;

    let plan =
        continued(&project, &run, &[&depth[..], &["--spawned", "1"]].concat())?;

    assert_withholds(&project, &plan, &["web"])
}

#[test]
fn every_repeated_withheld_profile_is_kept() -> Result<(), TestError> {
    let project = halting_set(
        "withheld-twice",
        &format!("{OPENALEX_THEN_TWO_WEB}\n- [ ] D? — profiles: arxiv"),
    )?;
    let depth = ["--depth", "2", "--concurrency", "1"];
    let (_, run) = started(&project, &depth)?;

    let plan = continued(
        &project,
        &run,
        &[
            &depth[..],
            &["--spawned", "0", WITHHOLD, "web", WITHHOLD, "arxiv"],
        ]
        .concat(),
    )?;

    assert_withholds(&project, &plan, &["arxiv", "web"])
}

#[test]
fn a_halted_run_still_offers_non_web_researchers_and_composers(
) -> Result<(), TestError> {
    let project = halting_set(
        "halted-still-offers",
        "- [ ] A? — profiles: web\n- [ ] B? — profiles: openalex",
    )?;
    project.write_note("01-a-web", "1", "A?", &["X?"])?;
    project.write_note("01-a-web", "2-1", "X?", &[])?;
    let (_, run) = started(&project, &["--depth", "2"])?;

    let plan = continued(&project, &run, &["--depth", "2", WITHHOLD, "web"])?;

    assert_eq!(spawns_of(&plan)?, ["01-a-web", "02-b-openalex:1"]);
    assert_eq!(plan["withheld"], json!(["web"]));
    Ok(())
}

#[test]
fn withholding_a_re_offered_batch_leaves_its_spawns_unattempted(
) -> Result<(), TestError> {
    let project = halting_set(
        "withheld-re-offer",
        "- [ ] A? — profiles: web\n- [ ] B? — profiles: openalex",
    )?;
    let depth = ["--depth", "2"];
    let (first, run) = started(&project, &depth)?;
    assert_eq!(spawns_of(&first)?, ["01-a-web:1", "02-b-openalex:1"]);

    let plan =
        continued(&project, &run, &[&depth[..], &[WITHHOLD, "web"]].concat())?;

    assert_withholds(&project, &plan, &["web"])
}

#[test]
fn withholding_needs_a_run_and_a_profile() -> Result<(), TestError> {
    let project = a_set()?;
    let needs_run =
        "E_TOPIC_RESEARCH_WITHHOLD_PROFILE: --withhold-profile needs --run";
    let cases: [(&[&str], &str); 3] = [
        (&[WITHHOLD, "web"], needs_run),
        (&["--start", WITHHOLD, "web"], needs_run),
        (
            &["--run", "x", WITHHOLD, ""],
            "E_TOPIC_RESEARCH_WITHHOLD_PROFILE: --withhold-profile is empty; \
             pass a profile name",
        ),
    ];
    for (args, stderr) in cases {
        assert_exits_1(&project.outstanding_with(args)?, stderr);
    }
    assert!(!project.ledger().exists());
    Ok(())
}

#[test]
fn withholding_an_unknown_profile_exits_1_and_leaves_the_ledger(
) -> Result<(), TestError> {
    let project = halting_set("withheld-unknown", OPENALEX_THEN_TWO_WEB)?;
    let (_, run) = started(&project, &["--depth", "2"])?;
    let ledger = fs::read(project.ledger())?;

    let output = project
        .outstanding_with(&["--run", &run, "--depth", "2", WITHHOLD, "Web"])?;

    assert_exits_1(
        &output,
        "E_TOPIC_RESEARCH_WITHHOLD_PROFILE: unknown profile 'Web'; known: \
         arxiv, openalex, web",
    );
    assert_eq!(fs::read(project.ledger())?, ledger);
    Ok(())
}

#[test]
fn a_run_plan_withholds_nothing_until_told_to() -> Result<(), TestError> {
    let project = a_set()?;

    let (plan, run) = started(&project, &[])?;
    let (query, _) = project.plan_with(&[])?;
    let continued = continued(&project, &run, &[])?;

    assert_eq!(plan["withheld"], json!([]));
    assert_eq!(continued["withheld"], json!([]));
    assert_eq!(query.get("withheld"), None);
    assert_eq!(stored_ledger(&project)?.get("withheld"), None);
    Ok(())
}

#[test]
fn every_plan_lists_every_unanswered_pair_beyond_the_window(
) -> Result<(), TestError> {
    let project = web_set("unanswered", 3, 0, 0)?;
    let limited = ["--concurrency", "1"];

    let (query, _) = project.plan_with(&limited)?;
    let (run, _) = started(&project, &limited)?;

    for plan in [query, run] {
        assert_eq!(plan["pairs"].as_array().map(Vec::len), Some(1), "{plan}");
        assert_eq!(
            plan["unanswered"].as_array().map(Vec::len),
            Some(3),
            "{plan}"
        );
    }
    Ok(())
}

#[test]
fn the_tick_plan_lists_every_web_pair_left_unfinished() -> Result<(), TestError>
{
    let project =
        small_set("tick-unanswered", "- [ ] A?\n- [ ] B?\n- [ ] C?\n- [ ] D?")?;
    project.write_note("01-a-web", "1", "A?", &["X?"])?;
    project.write_note("02-b-web", "1", "B?", &[])?;
    let deep = ["--depth", "3", "--concurrency", "1"];
    let (first, run) = started(&project, &deep)?;
    assert_eq!(spawns_of(&first)?, ["03-c-web:1"]);
    continued(&project, &run, &[&deep[..], &["--spawned", "0"]].concat())?;
    assert!(project
        .run(&["end-run", "s", "--run", &run])?
        .status
        .success());

    let (tick, _) = project.plan_with(&["--depth", "3"])?;

    assert_eq!(
        tick["unanswered"],
        json!([
            unanswered("01-a-web", "A?", "web", "research_nodes"),
            unanswered("02-b-web", "B?", "web", "compose"),
            unanswered("03-c-web", "C?", "web", "research_nodes"),
            unanswered("04-d-web", "D?", "web", "research_nodes"),
        ])
    );
    Ok(())
}

#[test]
fn a_re_run_after_a_halted_run_ends_offers_the_failed_nodes_afresh(
) -> Result<(), TestError> {
    let project = a_set()?;
    let (_, run) = started(&project, &["--depth", "2"])?;
    continued(
        &project,
        &run,
        &["--depth", "2", "--spawned", "0", WITHHOLD, "web"],
    )?;
    assert!(project
        .run(&["end-run", "s", "--run", &run])?
        .status
        .success());

    let (plan, _) = started(&project, &["--depth", "2"])?;

    assert_eq!(plan.get("replaced_run"), None, "{plan}");
    assert_eq!(plan["withheld"], json!([]));
    assert_eq!(spawns_of(&plan)?, ["01-a-web:1"]);
    Ok(())
}
