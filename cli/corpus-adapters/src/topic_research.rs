//! Reading a `topic-research` set from disk into the inputs its round plan
//! is derived from.

use std::path::Path;
use std::path::PathBuf;

use corpus::scan::DirReader;
use corpus::scan::FileReader;
use corpus::topic_research::round::Finding;
use corpus::topic_research::round::Outline;
use corpus::topic_research::round::QuarantineMarker;
use corpus::topic_research::round::RoundInputs;
use corpus::topic_research::round::DEFAULT_PROFILE;
use corpus::FrontmatterValue;
use corpus::Mapping;
use corpus::Scalar;

use crate::document::parse;
use crate::document::FrontmatterState;
use crate::frontmatter_validation::validate_path;

const PROFILE_SUFFIX: &str = "-profile";
const QUARANTINE_SUFFIX: &str = ".invalid";

/// Where the skill defining profile `name` lives under a profiles directory.
#[must_use]
pub fn profile_skill_path(profiles_dir: &Path, name: &str) -> PathBuf {
    profiles_dir
        .join(format!("{name}{PROFILE_SUFFIX}"))
        .join("SKILL.md")
}

/// The name of every profile with a skill under `profiles_dir`, sorted.
///
/// # Errors
///
/// A [`kernel::Error`] when `profiles_dir` is missing or cannot be listed.
pub fn available_profiles<F: DirReader + FileReader>(
    profiles_dir: &Path,
    fs: &F,
) -> Result<Vec<String>, kernel::Error> {
    let entries = fs.list(profiles_dir)?.ok_or_else(|| {
        kernel::Error::Failed(format!(
            "no profiles directory at {}",
            profiles_dir.display()
        ))
    })?;
    let mut names = Vec::new();
    for entry in entries {
        if let Some(name) = entry.strip_suffix(PROFILE_SUFFIX) {
            if fs.read(&profile_skill_path(profiles_dir, name))?.is_some() {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// The outline, findings, quarantine markers, and brief profiles of the set
/// rooted at `set_root`.
///
/// A finding is retained when it validates; an absent outline has no items,
/// and a brief naming no profiles names `web`.
///
/// # Errors
///
/// A [`kernel::Error`] when a directory cannot be listed or a file read fails
/// for a reason other than absence.
pub fn read_round_inputs<F: DirReader + FileReader>(
    set_root: &Path,
    profiles_dir: &Path,
    fs: &F,
) -> Result<RoundInputs, kernel::Error> {
    let outline = fs
        .read(&set_root.join("outline.md"))?
        .map_or_else(Outline::default, |text| Outline::parse(&text));
    let source_profiles = fs
        .read(&set_root.join("brief.md"))?
        .and_then(|text| string_list(&frontmatter(&text)?, "source_profiles"))
        .unwrap_or_else(|| vec![DEFAULT_PROFILE.to_owned()]);
    let (findings, markers) = read_findings(&set_root.join("findings"), fs)?;
    Ok(RoundInputs {
        outline,
        findings,
        markers,
        source_profiles,
        available_profiles: available_profiles(profiles_dir, fs)?,
    })
}

fn read_findings<F: DirReader + FileReader>(
    findings_dir: &Path,
    fs: &F,
) -> Result<(Vec<Finding>, Vec<QuarantineMarker>), kernel::Error> {
    let mut names = fs.list(findings_dir)?.unwrap_or_default();
    names.sort();
    let mut findings = Vec::new();
    let mut markers = Vec::new();
    for name in names {
        let path = findings_dir.join(&name);
        if name.starts_with('.') && name.ends_with(QUARANTINE_SUFFIX) {
            let question = fs
                .read(&path)?
                .and_then(|text| string(&frontmatter(&text)?, "question"));
            markers.push(QuarantineMarker::new(&name, question));
        } else if !name.starts_with('.') && is_markdown(&path) {
            findings.push(read_finding(&path, &name, fs)?);
        }
    }
    Ok((findings, markers))
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

fn read_finding<F: FileReader>(
    path: &Path,
    name: &str,
    fs: &F,
) -> Result<Finding, kernel::Error> {
    if !validate_path(path, fs)?.is_empty() {
        return Ok(Finding::invalid(name));
    }
    let answer =
        fs.read(path)?
            .and_then(|text| frontmatter(&text))
            .and_then(|fields| {
                Some((
                    string(&fields, "question")?,
                    string(&fields, "source_profile")?,
                ))
            });
    Ok(answer.map_or_else(
        || Finding::invalid(name),
        |(question, profile)| Finding::retained(name, &question, &profile),
    ))
}

fn frontmatter(text: &str) -> Option<Mapping> {
    match parse(text.as_bytes()).state {
        FrontmatterState::Parsed(mapping) => Some(mapping),
        FrontmatterState::Absent | FrontmatterState::Malformed => None,
    }
}

fn string(fields: &Mapping, key: &str) -> Option<String> {
    match fields.get(key)? {
        FrontmatterValue::Scalar(Scalar::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn string_list(fields: &Mapping, key: &str) -> Option<Vec<String>> {
    let FrontmatterValue::Sequence(items) = fields.get(key)? else {
        return None;
    };
    items
        .iter()
        .map(|item| match item {
            FrontmatterValue::Scalar(Scalar::String(value)) => {
                Some(value.clone())
            }
            _ => None,
        })
        .collect()
}
