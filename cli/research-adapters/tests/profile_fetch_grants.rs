//! Repository-integrity guards on the fetch grant: every skill that injects an
//! academic profile, and so spawns researchers who call the fetch, must grant
//! the command the research guard permits, because the researchers inherit
//! their parent's allow rules.

use std::path::Path;
use std::path::PathBuf;

use corpus::frontmatter_validation::template_shape::extract_frontmatter;
use research::confinement::PERMITTED_PREFIX;
use research_adapters::topic_research::profile_skill_path;

type TestError = Box<dyn std::error::Error>;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn read(relative: &str) -> Result<String, TestError> {
    Ok(std::fs::read_to_string(repo_root()?.join(relative))?)
}

const PROFILES_DIR: &str = "skills/research/profiles";

fn fetch_grant() -> String {
    format!("Bash({PERMITTED_PREFIX}*)")
}

fn allowed_tools(skill: &str) -> Vec<String> {
    let frontmatter = extract_frontmatter(skill);
    let mut tools = Vec::new();
    let mut in_list = false;
    for line in frontmatter.lines() {
        if line.starts_with("allowed-tools:") {
            in_list = true;
        } else if in_list {
            match line.trim_start().strip_prefix("- ") {
                Some(tool) => tools.push(tool.trim().to_owned()),
                None => in_list = false,
            }
        }
    }
    tools
}

#[test]
fn research_topic_grants_the_guards_permitted_command() -> Result<(), TestError>
{
    let skill = read("skills/research/research-topic/SKILL.md")?;
    assert!(
        allowed_tools(&skill).contains(&fetch_grant()),
        "research-topic's allowed-tools must grant {}, which the researchers \
         it spawns inherit",
        fetch_grant()
    );
    Ok(())
}

fn skill_files(dir: &Path) -> Result<Vec<PathBuf>, TestError> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(skill_files(&path)?);
        } else if path.file_name().is_some_and(|name| name == "SKILL.md") {
            found.push(path);
        }
    }
    Ok(found)
}

fn academic_profiles() -> Result<Vec<String>, TestError> {
    let root = repo_root()?;
    let mut academic = Vec::new();
    for entry in std::fs::read_dir(root.join(PROFILES_DIR))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let Some(profile) = name.strip_suffix("-profile") else {
            continue;
        };
        let skill = profile_skill_path(&root.join(PROFILES_DIR), profile);
        if std::fs::read_to_string(skill)?.contains(PERMITTED_PREFIX.trim_end())
        {
            academic.push(profile.to_owned());
        }
    }
    Ok(academic)
}

/// Whether `skill` injects `profile`, by name or through the `<profile>`
/// placeholder a per-pair injection is written with.
fn injects(skill: &str, profile: &str) -> bool {
    [profile, "<profile>"].iter().any(|name| {
        skill.contains(
            profile_skill_path(Path::new(PROFILES_DIR), name)
                .to_string_lossy()
                .as_ref(),
        )
    })
}

#[test]
fn every_skill_injecting_an_academic_profile_grants_its_fetch(
) -> Result<(), TestError> {
    let root = repo_root()?;
    let academic = academic_profiles()?;
    assert!(!academic.is_empty(), "no profile invokes the fetch");
    let mut injecting = 0;
    for path in skill_files(&root.join("skills"))? {
        if path.starts_with(root.join(PROFILES_DIR)) {
            continue;
        }
        let skill = std::fs::read_to_string(&path)?;
        if academic.iter().any(|profile| injects(&skill, profile)) {
            injecting += 1;
            assert!(
                allowed_tools(&skill).contains(&fetch_grant()),
                "{} injects an academic profile without granting {}",
                path.display(),
                fetch_grant()
            );
        }
    }
    assert!(injecting > 0, "no skill injects an academic profile");
    Ok(())
}
