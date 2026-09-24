//! `accelerator-corpus topic-research outstanding` black-box CLI coverage:
//! the round plan a committed set yields, as the JSON `conduct` consumes.

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use corpus::topic_research::is_finding_path;
use serde_json::json;
use serde_json::Value;

type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-corpus");
const TOPICS: &str = "meta/research/topics";

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Project {
    fn new(tag: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix(&format!("corpus-topic-research-{tag}-"))
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
        Ok(Command::new(BIN)
            .current_dir(&self.root)
            .args(["topic-research", "outstanding", slug, "--profiles-dir"])
            .arg(profiles_dir)
            .output()?)
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

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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
    json!({"question": question, "profile": profile, "path": path})
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
