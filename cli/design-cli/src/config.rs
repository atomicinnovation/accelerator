//! The configuration reads the executor makes.
//!
//! Library calls rather than nested `accelerator config` invocations, so the
//! executor pays no launcher bootstrap of its own.

use std::path::Path;

use ::config::consent::reportable;
use ::config::consent::resolve_executable_path;
use ::config::consent::ExecutablePathKey;
use ::config::consent::ExecutablePaths;
use ::config::consent::ProvenanceContext;
use ::config::consent::Refusal;
use ::config::consent::Rejection;
use ::config::consent::RepositoryRoots;
use ::config::consent::Usable;
use ::config::ConfigAccess as _;
use ::config::Key;
use config_adapters::credentials::SystemEnvironment;
use config_adapters::credentials::PERSONAL_CONFIG_RELATIVE;
use config_adapters::paths::SystemExecutablePaths;
use config_adapters::FileConfigStore;
use consent_adapters::VcsConfigFileTracking;
use design::executor::launch::LaunchFailure;
use design::runtime::browser_path::HatchDecision;

const BROWSER_PATH: &str = "design.browser_path";

/// The configured temporary directory, relative to the repository root.
///
/// # Errors
///
/// A [`LaunchFailure::Failed`] when the configuration cannot be composed or the
/// key cannot be resolved.
pub fn resolve_tmp_dir(cwd: &Path) -> Result<String, LaunchFailure> {
    let failed = |error: &dyn std::fmt::Display| {
        LaunchFailure::Failed(kernel::Error::Failed(format!(
            "could not resolve paths.tmp: {error}"
        )))
    };
    let composed =
        config_adapters::compose(cwd, config_adapters::LegacyPolicy::Reject)
            .map_err(|error| failed(&error))?;
    composed.report_ignored_personal_file();
    let key = Key::parse("paths.tmp").map_err(|error| failed(&error))?;
    Ok(composed
        .service
        .effective_nonempty(&key, None)
        .map_err(|error| failed(&error))?
        .rendered())
}

/// The `design.browser_path` hatch for the project enclosing `cwd`.
///
/// # Errors
///
/// A [`LaunchFailure::Failed`] when the configuration cannot be composed or
/// read.
pub fn browser_hatch(cwd: &Path) -> Result<HatchDecision, LaunchFailure> {
    let composed =
        config_adapters::compose(cwd, config_adapters::LegacyPolicy::Reject)
            .map_err(|error| hatch_failure(&error))?;
    composed.report_ignored_personal_file();
    let config_root = FileConfigStore::discover_root(cwd);
    let provenance = ProvenanceContext {
        config: &composed.service,
        tracking: &VcsConfigFileTracking,
        environment: &SystemEnvironment,
        personal_config: config_root.join(PERSONAL_CONFIG_RELATIVE),
    };
    resolve_browser_hatch(
        &provenance,
        &consent_adapters::repository_roots(&config_root, cwd),
        &SystemExecutablePaths,
    )
}

/// The hatch the consent policy admits, the bundled browser standing in for
/// a refused or absent one.
///
/// # Errors
///
/// A [`LaunchFailure::Failed`] when a config level cannot be read, after its
/// refusals so far are printed as warnings.
pub fn resolve_browser_hatch(
    provenance: &ProvenanceContext<'_>,
    roots: &RepositoryRoots,
    paths: &dyn ExecutablePaths,
) -> Result<HatchDecision, LaunchFailure> {
    let key = ExecutablePathKey::declared(BROWSER_PATH)
        .map_err(|error| hatch_failure(&error))?;
    let consented = resolve_executable_path(provenance, &key, roots, paths)
        .map_err(|aborted| {
            for warning in reportable(&aborted.warnings) {
                eprintln!("warning: {warning}");
            }
            hatch_failure(&aborted.error)
        })?;
    Ok(match consented.or_fallback(None) {
        Usable::Value {
            value,
            warnings,
            notice,
        } => HatchDecision {
            browser: Some(value),
            warnings: rendered(&warnings),
            notice: notice.map(|notice| notice.to_string()),
        },
        Usable::Refused(Rejection { fatal, warnings }) => HatchDecision {
            browser: None,
            warnings: rendered(
                &std::iter::once(fatal).chain(warnings).collect::<Vec<_>>(),
            ),
            notice: None,
        },
        Usable::Absent => HatchDecision {
            browser: None,
            warnings: Vec::new(),
            notice: None,
        },
    })
}

fn rendered(refusals: &[Refusal]) -> Vec<String> {
    reportable(refusals).map(ToString::to_string).collect()
}

fn hatch_failure(error: &dyn std::fmt::Display) -> LaunchFailure {
    LaunchFailure::Failed(kernel::Error::Failed(format!(
        "could not resolve {BROWSER_PATH}: {error}"
    )))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use std::collections::BTreeMap;
    use std::os::unix::fs::symlink;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::Path;
    use std::path::PathBuf;

    use config::consent::Environment;
    use config::consent::ProvenanceContext;
    use config_adapters::credentials::PERSONAL_CONFIG_RELATIVE;
    use config_adapters::paths::SystemExecutablePaths;
    use config_adapters::LegacyPolicy;
    use consent_adapters::VcsConfigFileTracking;
    use design::runtime::browser_path::HatchDecision;
    use vcs_test_support::hermetic::Hermetic;

    use super::resolve_browser_hatch;

    struct FixedEnvironment(BTreeMap<String, String>);

    impl Environment for FixedEnvironment {
        fn read(&self, name: &str) -> Option<String> {
            self.0.get(name).cloned()
        }
    }

    struct Scratch {
        work: tempfile::TempDir,
        git: Hermetic,
        checkout: PathBuf,
        environment: FixedEnvironment,
    }

    impl Scratch {
        fn new() -> Self {
            let work = tempfile::Builder::new()
                .prefix("design-hatch-")
                .tempdir()
                .unwrap();
            let git = Hermetic::rooted_at(work.path()).unwrap();
            let checkout = work.path().join("repo");
            std::fs::create_dir_all(checkout.join(".accelerator")).unwrap();
            git.git(&["init", "--quiet"], &checkout).unwrap();
            Self {
                work,
                git,
                checkout,
                environment: FixedEnvironment(BTreeMap::new()),
            }
        }

        fn in_linked_worktree() -> Self {
            let mut scratch = Self::new();
            scratch
                .git
                .git(
                    &["commit", "--quiet", "--allow-empty", "-m", "init"],
                    &scratch.checkout,
                )
                .unwrap();
            let linked = scratch.work.path().join("linked");
            scratch
                .git
                .git(
                    &["worktree", "add", "-q", linked.to_str().unwrap()],
                    &scratch.checkout,
                )
                .unwrap();
            std::fs::create_dir_all(linked.join(".accelerator")).unwrap();
            scratch.checkout = linked;
            scratch
        }

        fn main_checkout(&self) -> PathBuf {
            self.work.path().join("repo")
        }

        fn outside(&self, relative: &str) -> PathBuf {
            let path = self.work.path().join("outside").join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "").unwrap();
            path
        }

        fn team(self, value: &str) -> Self {
            std::fs::write(
                self.checkout.join(".accelerator/config.md"),
                format!("---\ndesign:\n  browser_path: \"{value}\"\n---\n"),
            )
            .unwrap();
            self
        }

        fn personal(self, value: &str) -> Self {
            self.personal_with_mode(value, 0o600)
        }

        fn personal_with_mode(self, value: &str, mode: u32) -> Self {
            let path = self.checkout.join(PERSONAL_CONFIG_RELATIVE);
            std::fs::write(
                &path,
                format!("---\ndesign:\n  browser_path: \"{value}\"\n---\n"),
            )
            .unwrap();
            std::fs::set_permissions(
                &path,
                std::fs::Permissions::from_mode(mode),
            )
            .unwrap();
            self
        }

        fn tracked(self) -> Self {
            self.git
                .git(&["add", PERSONAL_CONFIG_RELATIVE], &self.checkout)
                .unwrap();
            self
        }

        fn env(mut self, value: &str) -> Self {
            self.environment.0.insert(
                "ACCELERATOR_DESIGN_BROWSER_PATH".to_owned(),
                value.to_owned(),
            );
            self
        }

        fn hatch(&self) -> HatchDecision {
            let composed =
                config_adapters::compose(&self.checkout, LegacyPolicy::Reject)
                    .unwrap();
            let provenance = ProvenanceContext {
                config: &composed.service,
                tracking: &VcsConfigFileTracking,
                environment: &self.environment,
                personal_config: self.checkout.join(PERSONAL_CONFIG_RELATIVE),
            };
            let roots = consent_adapters::repository_roots(
                &self.checkout,
                &self.checkout,
            );
            resolve_browser_hatch(&provenance, &roots, &SystemExecutablePaths)
                .unwrap_or_else(|_| panic!("the hatch could not be resolved"))
        }
    }

    fn canonical(path: &Path) -> PathBuf {
        path.canonicalize().unwrap()
    }

    fn codes(decision: &HatchDecision) -> Vec<&str> {
        decision
            .warnings
            .iter()
            .map(|warning| warning.split(':').next().unwrap_or_default())
            .collect()
    }

    #[test]
    fn a_team_only_value_warns_and_leaves_the_bundled_browser() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("chrome");
        let decision = scratch.team(chrome.to_str().unwrap()).hatch();

        assert_eq!(decision.browser, None);
        assert_eq!(codes(&decision), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    }

    #[test]
    fn a_blank_personal_value_does_not_mask_the_team_warning() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("chrome");
        let decision = scratch
            .team(chrome.to_str().unwrap())
            .personal("  ")
            .hatch();

        assert_eq!(decision.browser, None);
        assert_eq!(codes(&decision), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    }

    #[test]
    fn a_relative_environment_value_warns_and_falls_back() {
        let decision = Scratch::new().env("./chromium").hatch();

        assert_eq!(decision.browser, None);
        assert_eq!(codes(&decision), ["E_EXECUTABLE_PATH_RELATIVE"]);
    }

    #[test]
    fn a_relative_environment_value_falls_through_to_a_valid_personal_one() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("chrome");
        let expected = canonical(&chrome);
        let decision = scratch
            .personal(chrome.to_str().unwrap())
            .env("chromium")
            .hatch();

        assert_eq!(decision.browser, Some(expected));
        assert_eq!(codes(&decision), ["E_EXECUTABLE_PATH_RELATIVE"]);
        assert_eq!(decision.notice, None);
    }

    #[test]
    fn a_tracked_personal_file_warns_and_falls_back() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("chrome");
        let decision =
            scratch.personal(chrome.to_str().unwrap()).tracked().hatch();

        assert_eq!(decision.browser, None);
        assert_eq!(codes(&decision), ["E_CONSENT_KEY_TRACKED"]);
    }

    #[test]
    fn a_path_inside_a_linked_worktrees_main_checkout_is_refused() {
        let scratch = Scratch::in_linked_worktree();
        let inside = scratch.main_checkout().join("bin/chrome");
        std::fs::create_dir_all(inside.parent().unwrap()).unwrap();
        std::fs::write(&inside, "").unwrap();
        let decision = scratch.personal(inside.to_str().unwrap()).hatch();

        assert_eq!(decision.browser, None);
        assert_eq!(codes(&decision), ["E_EXECUTABLE_PATH_INSIDE_REPOSITORY"]);
    }

    #[test]
    fn a_relative_personal_value_warns_and_falls_back() {
        let decision = Scratch::new().personal("bin/chromium").hatch();

        assert_eq!(decision.browser, None);
        assert_eq!(codes(&decision), ["E_EXECUTABLE_PATH_RELATIVE"]);
    }

    #[test]
    fn an_environment_value_beside_an_insecure_personal_file_is_used() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("chrome");
        let expected = canonical(&chrome);
        let decision = scratch
            .personal_with_mode("/elsewhere/chrome", 0o644)
            .env(chrome.to_str().unwrap())
            .hatch();

        assert_eq!(decision.browser, Some(expected));
        assert!(decision.warnings.is_empty(), "{:?}", decision.warnings);
        assert_eq!(
            decision.notice,
            Some(format!(
                "notice: design.browser_path taken from \
                 ACCELERATOR_DESIGN_BROWSER_PATH: {}",
                chrome.display()
            ))
        );
    }

    #[test]
    fn an_insecure_personal_files_value_is_ignored_for_the_bundled_browser() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("chrome");
        let decision = scratch
            .personal_with_mode(chrome.to_str().unwrap(), 0o644)
            .hatch();

        assert_eq!(decision.browser, None);
        assert!(decision.warnings.is_empty(), "{:?}", decision.warnings);
    }

    #[test]
    fn nothing_set_is_the_bundled_browser_without_warning_or_notice() {
        let decision = Scratch::new().hatch();

        assert_eq!(
            decision,
            HatchDecision {
                browser: None,
                warnings: Vec::new(),
                notice: None,
            }
        );
    }

    #[test]
    fn a_personal_value_carries_no_notice() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("chrome");
        let expected = canonical(&chrome);
        let decision = scratch.personal(chrome.to_str().unwrap()).hatch();

        assert_eq!(decision.browser, Some(expected));
        assert_eq!(decision.notice, None);
        assert!(decision.warnings.is_empty());
    }

    #[test]
    fn an_outside_symlink_with_a_relative_target_yields_its_canonical_target() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("Caskroom/chromium/chromium");
        let expected = canonical(&chrome);
        let link = scratch.work.path().join("outside/bin/chromium");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink("../Caskroom/chromium/chromium", &link).unwrap();
        let decision = scratch.personal(link.to_str().unwrap()).hatch();

        assert_eq!(decision.browser, Some(expected));
    }

    #[test]
    fn a_relative_target_symlink_pointing_into_the_repository_is_refused() {
        let scratch = Scratch::new();
        let inside = scratch.checkout.join("chromium");
        std::fs::write(&inside, "").unwrap();
        let link = scratch.work.path().join("outside/bin/chromium");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink("../../repo/chromium", &link).unwrap();
        let decision = scratch.personal(link.to_str().unwrap()).hatch();

        assert_eq!(decision.browser, None);
        assert_eq!(codes(&decision), ["E_EXECUTABLE_PATH_INSIDE_REPOSITORY"]);
    }

    #[test]
    fn a_two_hop_chain_outside_the_repository_yields_the_final_target() {
        let scratch = Scratch::new();
        let chrome = scratch.outside("real/chrome");
        let expected = canonical(&chrome);
        let middle = scratch.work.path().join("outside/middle");
        let first = scratch.work.path().join("outside/first");
        symlink(&chrome, &middle).unwrap();
        symlink("middle", &first).unwrap();
        let decision = scratch.personal(first.to_str().unwrap()).hatch();

        assert_eq!(decision.browser, Some(expected));
    }

    #[test]
    fn a_self_referencing_link_and_a_two_link_cycle_are_refused() {
        for (link, cycle) in [
            ("self", &[("self", "self")][..]),
            ("a", &[("a", "b"), ("b", "a")][..]),
        ] {
            let scratch = Scratch::new();
            let outside = scratch.work.path().join("outside");
            std::fs::create_dir_all(&outside).unwrap();
            for (from, to) in cycle {
                symlink(outside.join(to), outside.join(from)).unwrap();
            }
            let decision = scratch
                .personal(outside.join(link).to_str().unwrap())
                .hatch();

            assert_eq!(decision.browser, None, "{link}");
            assert_eq!(
                codes(&decision),
                ["E_EXECUTABLE_PATH_INSIDE_REPOSITORY"]
            );
        }
    }
}
