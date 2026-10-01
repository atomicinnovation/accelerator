//! Placing a write target below the research topics directory, as it
//! resolves on the real filesystem.

use std::io::ErrorKind;
use std::path::Component;
use std::path::Path;

use corpus::topic_research::is_finding_path;
use research::confinement::FindingsScope;
use research::confinement::PathRejection;
use research::confinement::TopicsRelativePath;

/// Resolves `path` against `cwd` and places it below `topics`, refusing a
/// `.` or `..` component and any symlink below the topics directory.
///
/// A missing component ends the walk, so a first write before its
/// directories exist is placed.
pub fn locate(
    path: &str,
    cwd: &Path,
    topics: &Path,
) -> Result<TopicsRelativePath, PathRejection> {
    let target = cwd.join(path);
    if has_dot_component(&target) {
        return Err(PathRejection::DotComponent);
    }
    let canonical_topics = std::fs::canonicalize(topics)
        .map_err(|_| PathRejection::NotUnderTopics)?;
    let ancestor = target
        .ancestors()
        .filter(|ancestor| {
            std::fs::canonicalize(ancestor)
                .is_ok_and(|canonical| canonical == canonical_topics)
        })
        .last()
        .ok_or(PathRejection::NotUnderTopics)?;
    let below = target
        .strip_prefix(ancestor)
        .map_err(|_| PathRejection::NotUnderTopics)?;
    refuse_symlinks(ancestor, below)?;
    relative_text(below).map(TopicsRelativePath::new)
}

fn has_dot_component(path: &Path) -> bool {
    path.to_string_lossy()
        .split('/')
        .any(|segment| segment == "." || segment == "..")
}

fn refuse_symlinks(ancestor: &Path, below: &Path) -> Result<(), PathRejection> {
    let mut walked = ancestor.to_path_buf();
    for component in below.components() {
        walked.push(component);
        match std::fs::symlink_metadata(&walked) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(PathRejection::Symlink(
                    component.as_os_str().to_string_lossy().into_owned(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => break,
            Err(_) => return Err(PathRejection::Uninspectable),
        }
    }
    Ok(())
}

fn relative_text(below: &Path) -> Result<String, PathRejection> {
    below
        .components()
        .map(|component| match component {
            Component::Normal(segment) => segment
                .to_str()
                .map(str::to_owned)
                .ok_or(PathRejection::Uninspectable),
            _ => Err(PathRejection::Uninspectable),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|segments| segments.join("/"))
}

/// The corpus's own rule for which topics-relative paths are findings.
pub struct CorpusFindings;

impl FindingsScope for CorpusFindings {
    fn contains(&self, target: &TopicsRelativePath) -> bool {
        is_finding_path(target.as_str())
    }
}
