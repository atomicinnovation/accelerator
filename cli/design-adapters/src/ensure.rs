//! Invoking the launcher's `cache ensure` to materialise the runtime trees.
//!
//! The launcher holds the embedded signing key, so it owns materialisation and
//! the executor shells out to it. Discovery prefers the launcher's own exported
//! path, then the plugin root, then `PATH`: the dev-override configuration the
//! container fixtures use deliberately does not export the variable, so an
//! absent variable falls back rather than failing. A failure surfaces as the
//! launcher's own enumerated cause token, which the domain maps onto a downgrade
//! reason; a spawn error or an unparseable envelope yields an empty token the
//! domain maps to `artifact-unavailable`.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

/// The launcher binary's name, and the variable naming its resolved path.
const LAUNCHER_BASENAME: &str = "accelerator";
const LAUNCHER_PATH_VAR: &str = "ACCELERATOR_LAUNCHER_PATH";

/// A materialised tree: the sealed directory and the lease the executor holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTree {
    pub artifact: String,
    pub path: PathBuf,
    pub lease: PathBuf,
}

/// The outcome of a `cache ensure`.
#[derive(Debug, PartialEq, Eq)]
pub enum EnsureOutcome {
    Ready(Vec<ResolvedTree>),
    /// The launcher's cause token — empty when none was parseable — for the
    /// domain to classify onto a downgrade reason.
    Failed(String),
}

/// The ordered launcher candidates: the exported path, the plugin root's `bin`,
/// then each `PATH` directory. Pure, so the order is testable.
#[must_use]
pub fn launcher_candidates(
    exported: Option<&Path>,
    plugin_root: Option<&Path>,
    path_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = exported {
        candidates.push(path.to_path_buf());
    }
    if let Some(root) = plugin_root {
        candidates.push(root.join("bin").join(LAUNCHER_BASENAME));
    }
    for dir in path_dirs {
        candidates.push(dir.join(LAUNCHER_BASENAME));
    }
    candidates
}

/// The first candidate that exists, under an injected existence predicate.
#[must_use]
pub fn select_launcher(
    candidates: &[PathBuf],
    exists: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    candidates.iter().find(|path| exists(path)).cloned()
}

/// Locate the launcher from the exported path, the plugin root and `PATH`.
#[must_use]
pub fn discover_launcher(plugin_root: Option<&Path>) -> Option<PathBuf> {
    let exported = std::env::var_os(LAUNCHER_PATH_VAR)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default();
    let candidates =
        launcher_candidates(exported.as_deref(), plugin_root, &path_dirs);
    select_launcher(&candidates, &|path| path.is_file())
}

/// Run `accelerator cache ensure <artifacts...>` and interpret the result.
#[must_use]
pub fn ensure(launcher: &Path, artifacts: &[&str]) -> EnsureOutcome {
    let output = Command::new(launcher)
        .arg("cache")
        .arg("ensure")
        .args(artifacts)
        .output();
    let Ok(output) = output else {
        // A launcher we located but could not run: no cause to report.
        return EnsureOutcome::Failed(String::new());
    };
    if output.status.success() {
        EnsureOutcome::Ready(parse_resolved(&output.stdout))
    } else {
        EnsureOutcome::Failed(parse_cause(&output.stderr))
    }
}

/// Parse the tab-separated `name\t<tree>\t<lease>` success lines, skipping any
/// line that does not carry all three non-empty fields.
fn parse_resolved(stdout: &[u8]) -> Vec<ResolvedTree> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let artifact = fields.next()?;
            let path = fields.next()?;
            let lease = fields.next()?;
            (!artifact.is_empty() && !path.is_empty() && !lease.is_empty())
                .then(|| ResolvedTree {
                    artifact: artifact.to_owned(),
                    path: PathBuf::from(path),
                    lease: PathBuf::from(lease),
                })
        })
        .collect()
}

/// Extract the `cause` token from the launcher's JSON failure envelope, scanning
/// each line so surrounding output does not defeat the parse. An empty string
/// when none is found — the domain maps that to `artifact-unavailable`.
fn parse_cause(stderr: &[u8]) -> String {
    for line in String::from_utf8_lossy(stderr).lines() {
        let parsed =
            serde_json::from_str::<serde_json::Value>(line.trim()).ok();
        if let Some(cause) = parsed
            .as_ref()
            .and_then(|value| value.get("cause"))
            .and_then(serde_json::Value::as_str)
        {
            return cause.to_owned();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::ensure;
    use super::launcher_candidates;
    use super::parse_cause;
    use super::parse_resolved;
    use super::select_launcher;
    use super::EnsureOutcome;
    use super::ResolvedTree;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn candidates_run_exported_then_plugin_root_then_path() {
        let candidates = launcher_candidates(
            Some(Path::new("/exported/accelerator")),
            Some(Path::new("/plugin")),
            &[PathBuf::from("/a"), PathBuf::from("/b")],
        );
        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/exported/accelerator"),
                PathBuf::from("/plugin/bin/accelerator"),
                PathBuf::from("/a/accelerator"),
                PathBuf::from("/b/accelerator"),
            ]
        );
    }

    #[test]
    fn candidates_skip_the_unset_sources() {
        let candidates =
            launcher_candidates(None, None, &[PathBuf::from("/a")]);
        assert_eq!(candidates, vec![PathBuf::from("/a/accelerator")]);
    }

    #[test]
    fn select_returns_the_first_existing_candidate() {
        let candidates = vec![
            PathBuf::from("/missing/accelerator"),
            PathBuf::from("/present/accelerator"),
        ];
        let chosen = select_launcher(&candidates, &|path| {
            path == Path::new("/present/accelerator")
        });
        assert_eq!(chosen, Some(PathBuf::from("/present/accelerator")));
    }

    #[test]
    fn resolved_lines_parse_into_trees_and_skip_malformed_ones() {
        let stdout = b"driver\t/cache/trees/driver-abc\t/cache/trees/driver-abc.lease\nbrowser\t/cache/trees/browser-def\t/cache/trees/browser-def.lease\nmalformed-line-without-tabs\n";
        let resolved = parse_resolved(stdout);
        assert_eq!(
            resolved,
            vec![
                ResolvedTree {
                    artifact: "driver".to_owned(),
                    path: PathBuf::from("/cache/trees/driver-abc"),
                    lease: PathBuf::from("/cache/trees/driver-abc.lease"),
                },
                ResolvedTree {
                    artifact: "browser".to_owned(),
                    path: PathBuf::from("/cache/trees/browser-def"),
                    lease: PathBuf::from("/cache/trees/browser-def.lease"),
                },
            ]
        );
    }

    #[test]
    fn a_cause_is_extracted_from_the_envelope_among_other_output() {
        let stderr = b"warning: something\n{\"error\":\"ensure-failed\",\"cause\":\"disk-shortfall\",\"artifact\":\"driver\",\"message\":\"no space\"}\n";
        assert_eq!(parse_cause(stderr), "disk-shortfall");
    }

    #[test]
    fn no_envelope_yields_an_empty_cause() {
        assert_eq!(parse_cause(b"cache: command not found\n"), "");
    }

    fn fake_launcher(dir: &Path, script: &str) -> Result<PathBuf, TestError> {
        use std::os::unix::fs::PermissionsExt as _;
        let path = dir.join("accelerator");
        std::fs::write(&path, script)?;
        std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(0o755),
        )?;
        Ok(path)
    }

    #[test]
    fn a_successful_ensure_returns_the_resolved_trees() -> Result<(), TestError>
    {
        let work = tempfile::tempdir()?;
        let launcher = fake_launcher(
            work.path(),
            "#!/bin/sh\nprintf 'driver\\t/t/driver\\t/t/driver.lease\\n'\n",
        )?;
        let outcome = ensure(&launcher, &["driver"]);
        assert_eq!(
            outcome,
            EnsureOutcome::Ready(vec![ResolvedTree {
                artifact: "driver".to_owned(),
                path: PathBuf::from("/t/driver"),
                lease: PathBuf::from("/t/driver.lease"),
            }])
        );
        Ok(())
    }

    #[test]
    fn a_failed_ensure_returns_the_cause_token() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let launcher = fake_launcher(
            work.path(),
            "#!/bin/sh\n>&2 printf '{\"error\":\"ensure-failed\",\"cause\":\"cache-unwritable\",\"artifact\":\"driver\",\"message\":\"x\"}\\n'\nexit 1\n",
        )?;
        assert_eq!(
            ensure(&launcher, &["driver"]),
            EnsureOutcome::Failed("cache-unwritable".to_owned())
        );
        Ok(())
    }
}
