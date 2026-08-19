//! The security policy for the `design.browser_path` hatch.
//!
//! The hatch names a browser executable the daemon launches directly, so a
//! value an untrusted repository could choose is a code-execution vector. Two
//! barriers apply, both here: a team-level (repo-tracked) configuration value is
//! ignored — only the personal, gitignored level is honoured — and a value
//! resolving inside the repository being inventoried is refused. The precedence
//! between the environment override and the personal configuration value is
//! computed by the composition root through the shared config helper; this
//! function receives the already-chosen value and applies the security policy.

use std::path::Path;
use std::path::PathBuf;

/// The hatch decision after the security policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HatchDecision {
    /// The browser executable to launch, or `None` to prefer the bundled tree.
    pub browser: Option<PathBuf>,
    /// User-facing warnings: a team-level value ignored, or a repo-inside value
    /// refused. Rendered to stderr; never fatal.
    pub warnings: Vec<String>,
}

/// Apply the hatch security policy to an already-chosen value.
///
/// `chosen` is the environment-beats-personal-configuration value the
/// composition root computed (a whitespace-only value already treated as
/// absent). `team_level_present` is true when a team-level (repo-tracked) value
/// was set and thereby ignored. `canonicalise` resolves a path to its real
/// location, or `None` when it cannot — a path that does not exist cannot be
/// launched, so it is not refused on the repo-inside ground.
#[must_use]
pub fn vet(
    chosen: Option<&str>,
    team_level_present: bool,
    repo_root: &Path,
    canonicalise: &dyn Fn(&Path) -> Option<PathBuf>,
) -> HatchDecision {
    let mut warnings = Vec::new();
    if team_level_present {
        warnings.push(
            "design.browser_path is set at the team level and ignored; set it \
             in the personal config (.accelerator/config.local.md) instead"
                .to_owned(),
        );
    }
    let Some(value) = chosen else {
        return HatchDecision {
            browser: None,
            warnings,
        };
    };
    let path = PathBuf::from(value);
    if resolves_inside(&path, repo_root, canonicalise) {
        warnings.push(format!(
            "design.browser_path ({value}) resolves inside the repository being \
             inventoried and is refused; point it at a browser outside the \
             repository"
        ));
        return HatchDecision {
            browser: None,
            warnings,
        };
    }
    HatchDecision {
        browser: Some(path),
        warnings,
    }
}

/// Whether `path` — or the directory holding it — lands inside `repo_root` once
/// resolved. The containing-directory arm is what refuses a symlink committed
/// inside the repository that points at a binary outside it: the file the repo
/// controls is still the one named.
fn resolves_inside(
    path: &Path,
    repo_root: &Path,
    canonicalise: &dyn Fn(&Path) -> Option<PathBuf>,
) -> bool {
    let Some(root) = canonicalise(repo_root) else {
        return false;
    };
    if canonicalise(path).is_some_and(|real| real.starts_with(&root)) {
        return true;
    }
    path.parent().is_some_and(|parent| {
        canonicalise(parent).is_some_and(|real| real.starts_with(&root))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::vet;

    /// A canonicaliser that maps a fixed table of inputs to real locations, and
    /// returns `None` for anything else (a non-existent path).
    fn canonicaliser(
        table: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&Path) -> Option<PathBuf> {
        move |path: &Path| {
            table
                .iter()
                .find(|(input, _)| Path::new(input) == path)
                .map(|(_, real)| PathBuf::from(real))
        }
    }

    #[test]
    fn neither_set_prefers_the_bundled_tree_without_warning() {
        let decision =
            vet(None, false, Path::new("/repo"), &canonicaliser(&[]));
        assert_eq!(decision.browser, None);
        assert!(decision.warnings.is_empty());
    }

    #[test]
    fn a_chosen_value_outside_the_repo_is_honoured() {
        let decision = vet(
            Some("/usr/bin/chromium"),
            false,
            Path::new("/repo"),
            &canonicaliser(&[
                ("/usr/bin/chromium", "/usr/bin/chromium"),
                ("/repo", "/repo"),
            ]),
        );
        assert_eq!(decision.browser, Some(PathBuf::from("/usr/bin/chromium")));
        assert!(decision.warnings.is_empty());
    }

    #[test]
    fn a_team_level_value_is_ignored_with_a_warning_naming_the_personal_route()
    {
        // The team-level value is filtered out at the composition root, so
        // `chosen` is None; the flag drives the advisory.
        let decision = vet(None, true, Path::new("/repo"), &canonicaliser(&[]));
        assert_eq!(decision.browser, None);
        assert_eq!(decision.warnings.len(), 1);
        assert!(decision.warnings[0].contains("team level"));
        assert!(decision.warnings[0].contains("config.local.md"));
    }

    #[test]
    fn a_value_inside_the_repo_is_refused_with_a_warning() {
        let decision = vet(
            Some("/repo/tools/chromium"),
            false,
            Path::new("/repo"),
            &canonicaliser(&[
                ("/repo/tools/chromium", "/repo/tools/chromium"),
                ("/repo", "/repo"),
            ]),
        );
        assert_eq!(decision.browser, None, "a repo-inside value is refused");
        assert_eq!(decision.warnings.len(), 1);
        assert!(decision.warnings[0].contains("inside the repository"));
    }

    #[test]
    fn a_symlink_inside_the_repo_pointing_out_is_still_refused() {
        // The file lives in the repo though it resolves outside it, so the repo
        // still names the executable: refuse on the containing directory.
        let decision = vet(
            Some("/repo/chromium"),
            false,
            Path::new("/repo"),
            &canonicaliser(&[
                ("/repo/chromium", "/opt/evil/chromium"),
                ("/repo", "/repo"),
            ]),
        );
        assert_eq!(decision.browser, None);
        assert_eq!(decision.warnings.len(), 1);
        assert!(decision.warnings[0].contains("inside the repository"));
    }

    #[test]
    fn a_non_existent_value_is_passed_through_to_fail_at_launch() {
        // Nothing to execute, so it is not a security refusal; the broken hatch
        // surfaces when the daemon cannot launch it.
        let decision = vet(
            Some("/gone/chromium"),
            false,
            Path::new("/repo"),
            &canonicaliser(&[("/repo", "/repo")]),
        );
        assert_eq!(decision.browser, Some(PathBuf::from("/gone/chromium")));
    }
}
