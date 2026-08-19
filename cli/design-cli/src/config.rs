//! The configuration reads the executor makes.
//!
//! Library calls rather than nested `accelerator config` invocations, so the
//! executor pays no launcher bootstrap of its own.

use std::path::Path;

use ::config::env_beats_config;
use ::config::ConfigAccess as _;
use ::config::Key;
use ::config::Source;
use design::executor::launch::LaunchFailure;
use design::runtime::browser_path;
use design::runtime::browser_path::HatchDecision;

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
    let key = Key::parse("paths.tmp").map_err(|error| failed(&error))?;
    Ok(composed
        .service
        .effective_nonempty(&key, None)
        .map_err(|error| failed(&error))?
        .rendered())
}

/// The `design.browser_path` hatch, after the environment override and the
/// security policy.
///
/// The environment beats the personal-level value; a team-level (repo-tracked)
/// value is passed to the policy as ignored, never as the chosen value, so it
/// cannot name the browser the daemon launches. `repo_root` is the repository
/// being inventoried, against which a repo-inside value is refused.
///
/// # Errors
///
/// A [`LaunchFailure::Failed`] when the configuration cannot be composed or the
/// key cannot be resolved.
pub fn resolve_browser_hatch(
    cwd: &Path,
    repo_root: &Path,
) -> Result<HatchDecision, LaunchFailure> {
    let failed = |error: &dyn std::fmt::Display| {
        LaunchFailure::Failed(kernel::Error::Failed(format!(
            "could not resolve design.browser_path: {error}"
        )))
    };
    let composed =
        config_adapters::compose(cwd, config_adapters::LegacyPolicy::Reject)
            .map_err(|error| failed(&error))?;
    let key =
        Key::parse("design.browser_path").map_err(|error| failed(&error))?;
    let resolution = composed
        .service
        .effective_nonempty(&key, None)
        .map_err(|error| failed(&error))?;
    let (personal, team_level_present) = match resolution.source() {
        Source::Personal => (resolution.configured_value(), false),
        Source::Team => (None, true),
        Source::Catalogue | Source::Unset => (None, false),
    };
    let environment = std::env::var("ACCELERATOR_DESIGN_BROWSER_PATH").ok();
    let chosen = env_beats_config(environment.as_deref(), personal.as_deref());
    Ok(browser_path::vet(
        chosen.as_deref(),
        team_level_present,
        repo_root,
        &|path| std::fs::canonicalize(path).ok(),
    ))
}
