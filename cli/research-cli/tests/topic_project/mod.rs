//! A throwaway project holding a topic-research set, for running
//! `accelerator-research topic` against.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use serde_json::json;
use serde_json::Value;

pub type TestError = Box<dyn std::error::Error>;

const BIN: &str = env!("CARGO_BIN_EXE_accelerator-research");
pub const TOPICS: &str = "meta/research/topics";
pub const ACCELERATOR_CAP: &str = "ACCELERATOR_SUBAGENT_CAP";
pub const HARNESS_CAP: &str = "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS";
pub const SEARCH_CAP: &str = "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION";
pub const PLUGIN_ROOT: &str = "ACCELERATOR_PLUGIN_ROOT";

/// The repository's own plugin root, whose `skills/research/profiles` the
/// launcher would hand an installed `accelerator`.
pub fn installed_plugin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub struct Project {
    _dir: tempfile::TempDir,
    pub root: PathBuf,
}

impl Project {
    pub fn new(tag: &str) -> Result<Self, TestError> {
        let dir = tempfile::Builder::new()
            .prefix(&format!("research-topic-{tag}-"))
            .tempdir()?;
        let root = dir.path().canonicalize()?;
        fs::create_dir_all(root.join(".git"))?;
        Ok(Self { _dir: dir, root })
    }

    pub fn with_fixture_set(fixture: &str) -> Result<Self, TestError> {
        let project = Self::new("fixture")?;
        copy_tree(&fixtures().join(fixture), &project.set(fixture))?;
        Ok(project)
    }

    pub fn set(&self, slug: &str) -> PathBuf {
        self.root.join(TOPICS).join(slug)
    }

    pub fn write(
        &self,
        relative: &str,
        content: &str,
    ) -> Result<(), TestError> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    pub fn run(&self, verb: &str, args: &[&str]) -> Result<Output, TestError> {
        let mut all = vec![verb];
        all.extend_from_slice(args);
        self.run_in(&all, &[])
    }

    pub fn run_in(
        &self,
        args: &[&str],
        cap: &[(&str, &str)],
    ) -> Result<Output, TestError> {
        Ok(self.command(args).envs(cap.iter().copied()).output()?)
    }

    pub fn run_from(
        &self,
        plugin_root: &Path,
        verb: &str,
        args: &[&str],
    ) -> Result<Output, TestError> {
        let mut all = vec![verb];
        all.extend_from_slice(args);
        Ok(self.command(&all).env(PLUGIN_ROOT, plugin_root).output()?)
    }

    pub fn run_without_a_plugin_root(
        &self,
        verb: &str,
        args: &[&str],
    ) -> Result<Output, TestError> {
        let mut all = vec![verb];
        all.extend_from_slice(args);
        Ok(self.command(&all).env_remove(PLUGIN_ROOT).output()?)
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = command_without_an_inherited_cap();
        command.current_dir(&self.root).arg("topic").args(args);
        command
    }

    pub fn outstanding(
        &self,
        slug: &str,
        plugin_root: &Path,
    ) -> Result<Output, TestError> {
        self.run_from(plugin_root, "outstanding", &[slug])
    }

    pub fn outstanding_with(&self, args: &[&str]) -> Result<Output, TestError> {
        self.outstanding_in(args, &[])
    }

    pub fn outstanding_in(
        &self,
        args: &[&str],
        cap: &[(&str, &str)],
    ) -> Result<Output, TestError> {
        let mut all = vec!["outstanding", "s"];
        all.extend_from_slice(args);
        self.run_in(&all, cap)
    }

    pub fn plan_with(
        &self,
        args: &[&str],
    ) -> Result<(Value, String), TestError> {
        self.plan_in(args, &[])
    }

    pub fn plan_in(
        &self,
        args: &[&str],
        cap: &[(&str, &str)],
    ) -> Result<(Value, String), TestError> {
        let output = self.outstanding_in(args, cap)?;
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(output.status.success(), "{stderr}");
        Ok((serde_json::from_slice(&output.stdout)?, stderr))
    }

    pub fn write_note(
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

    pub fn ledger(&self) -> PathBuf {
        self.set("s").join(".conduct-run.json")
    }

    pub fn plan(&self, slug: &str) -> Result<Value, TestError> {
        self.plan_from(slug, &installed_plugin())
    }

    pub fn plan_from(
        &self,
        slug: &str,
        plugin_root: &Path,
    ) -> Result<Value, TestError> {
        let output = self.outstanding(slug, plugin_root)?;
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
        .env_remove(SEARCH_CAP)
        .env(PLUGIN_ROOT, installed_plugin());
    command
}

/// The committed topic-research sets live beside the frontmatter goldens that
/// hold them to the schema.
fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../corpus-cli/tests/fixtures")
}

pub fn copy_tree(from: &Path, to: &Path) -> Result<(), TestError> {
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

pub fn tree(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, TestError> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                files.insert(path.clone(), Vec::new());
                pending.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path)?);
            }
        }
    }
    Ok(files)
}

pub fn note_path(stem: &str, lineage: &str) -> String {
    format!("{TOPICS}/s/findings/{stem}.levels/{lineage}.md")
}

pub fn level_note_with(stem: &str, lineage: &str, fields: &str) -> String {
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

pub fn level_note(
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

pub fn started(
    project: &Project,
    args: &[&str],
) -> Result<(Value, String), TestError> {
    let mut all = vec!["--start"];
    all.extend_from_slice(args);
    let (plan, _) = project.plan_with(&all)?;
    let run = plan["run"].as_str().ok_or("no run")?.to_owned();
    Ok((plan, run))
}

pub fn continued(
    project: &Project,
    run: &str,
    args: &[&str],
) -> Result<Value, TestError> {
    let mut all = vec!["--run", run];
    all.extend_from_slice(args);
    Ok(project.plan_with(&all)?.0)
}

pub fn stored_ledger(project: &Project) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&fs::read_to_string(
        project.ledger(),
    )?)?)
}

pub fn assert_exits_1(output: &Output, stderr: &str) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim_end(), stderr);
    assert!(output.stdout.is_empty());
}
