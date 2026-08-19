//! `accelerator design notices` — surface each vendored tree's `NOTICES/`.
//!
//! The trees are materialised on demand, so this reads whatever is present: the
//! launcher exports `ACCELERATOR_TREE_<NAME>` for each resolved tree, and this
//! lists the `NOTICES/` directory and the components it covers. With nothing
//! materialised it rejects rather than printing an empty success, so a user
//! knows to run an inventory crawl (or `accelerator cache ensure`) first.

use std::fmt::Write as _;
use std::path::Path;
use std::path::PathBuf;

use crate::report::Report;

/// The vendored tree artifacts, matching the launcher's compiled-in set.
pub const ARTIFACTS: [&str; 2] = ["driver", "browser"];

/// One artifact and the tree the launcher resolved for it, if any.
pub struct Artifact {
    pub name: &'static str,
    pub tree: Option<PathBuf>,
}

/// Render the notices report from resolved trees and an injected lister.
///
/// `list_components` returns the sorted component names under a `NOTICES/`
/// directory, or `None` when that directory is absent or unreadable — so a
/// pointer to a half-materialised tree is skipped rather than crashing.
pub fn notices(
    artifacts: &[Artifact],
    list_components: &dyn Fn(&Path) -> Option<Vec<String>>,
) -> Report {
    let mut rendered = String::new();
    let mut any = false;
    for artifact in artifacts {
        let Some(tree) = &artifact.tree else {
            continue;
        };
        let directory = tree.join("NOTICES");
        let Some(components) = list_components(&directory) else {
            continue;
        };
        any = true;
        let _ =
            writeln!(rendered, "{}: {}", artifact.name, directory.display());
        for component in components {
            let _ = writeln!(rendered, "  {component}");
        }
    }
    if any {
        Report::Accepted {
            stdout: rendered,
            stderr: String::new(),
        }
    } else {
        Report::rejected(
            "no vendored runtime is materialised — run an inventory crawl \
             or `accelerator cache ensure` first",
        )
    }
}

/// Resolve the requested artifacts from the launcher-exported tree variables.
#[must_use]
pub fn run(artifact: Option<&str>) -> Report {
    let requested: Vec<Artifact> = ARTIFACTS
        .iter()
        .filter(|name| artifact.is_none_or(|wanted| wanted == **name))
        .map(|name| Artifact {
            name,
            tree: tree_from_env(name),
        })
        .collect();
    notices(&requested, &list_components_on_disk)
}

fn tree_from_env(name: &str) -> Option<PathBuf> {
    let variable = format!("ACCELERATOR_TREE_{}", name.to_uppercase());
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn list_components_on_disk(directory: &Path) -> Option<Vec<String>> {
    let mut components: Vec<String> = std::fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    components.sort();
    Some(components)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::notices;
    use super::Artifact;
    use crate::report::Report;

    fn lister(
        entries: &[(&'static str, &[&'static str])],
    ) -> impl Fn(&Path) -> Option<Vec<String>> {
        let table: Vec<(String, Vec<String>)> = entries
            .iter()
            .map(|(dir, comps)| {
                (
                    (*dir).to_owned(),
                    comps.iter().map(|c| (*c).to_owned()).collect(),
                )
            })
            .collect();
        move |directory: &Path| {
            let key = directory.to_string_lossy().into_owned();
            table
                .iter()
                .find(|(dir, _)| *dir == key)
                .map(|(_, comps)| comps.clone())
        }
    }

    #[test]
    fn every_materialised_tree_lists_its_components() {
        let artifacts = [
            Artifact {
                name: "driver",
                tree: Some(PathBuf::from("/d")),
            },
            Artifact {
                name: "browser",
                tree: Some(PathBuf::from("/b")),
            },
        ];
        let report = notices(
            &artifacts,
            &lister(&[
                ("/d/NOTICES", &["node", "playwright-core"]),
                ("/b/NOTICES", &["chromium"]),
            ]),
        );
        let Report::Accepted { stdout, .. } = report else {
            unreachable!("expected an acceptance");
        };
        assert!(stdout.contains("driver: /d/NOTICES"));
        assert!(stdout.contains("  node"));
        assert!(stdout.contains("  playwright-core"));
        assert!(stdout.contains("browser: /b/NOTICES"));
        assert!(stdout.contains("  chromium"));
    }

    #[test]
    fn a_single_requested_artifact_is_listed_alone() {
        let artifacts = [Artifact {
            name: "driver",
            tree: Some(PathBuf::from("/d")),
        }];
        let report = notices(&artifacts, &lister(&[("/d/NOTICES", &["node"])]));
        let Report::Accepted { stdout, .. } = report else {
            unreachable!("expected an acceptance");
        };
        assert!(stdout.contains("driver"));
        assert!(!stdout.contains("browser"));
    }

    #[test]
    fn nothing_materialised_is_a_rejection() {
        let artifacts = [Artifact {
            name: "driver",
            tree: None,
        }];
        let report = notices(&artifacts, &lister(&[]));
        assert!(matches!(report, Report::Rejected { .. }));
    }

    #[test]
    fn a_tree_without_a_notices_directory_is_skipped_not_crashed() {
        let artifacts = [Artifact {
            name: "driver",
            tree: Some(PathBuf::from("/absent")),
        }];
        // The lister returns None for the absent directory.
        let report = notices(&artifacts, &lister(&[]));
        assert!(matches!(report, Report::Rejected { .. }));
    }
}
