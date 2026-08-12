//! The one configuration read the executor makes.
//!
//! `run.sh` shelled out to `bin/accelerator config path tmp`, paying a whole
//! nested launcher bootstrap on every invocation. In process it is a library
//! call.

use std::path::Path;

use ::config::ConfigAccess as _;
use ::config::Key;
use design::executor::launch::LaunchFailure;

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
