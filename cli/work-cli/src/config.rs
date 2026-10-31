//! Shared config-resolution helpers used by more than one subcommand's
//! adapter wiring: the announced ID scheme, and the configured work-item
//! directory.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use ::config::Key;
use work::id_scheme::WorkItemIdScheme;
use work_adapters::config::read_scheme;
use work_adapters::config::resolve_scheme;
use work_adapters::config::SchemeError;
use work_adapters::config::SchemeReading;
use work_adapters::config::LEGACY_KEY_SETTING as LEGACY_PREFIX_KEY;

pub fn effective_nonempty(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<String, kernel::Error> {
    let parsed = Key::parse(key)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(config
        .effective_nonempty(&parsed, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?
        .rendered())
}

fn announced(reading: SchemeReading) -> WorkItemIdScheme {
    ::config::emit_deprecation_once(
        LEGACY_PREFIX_KEY,
        reading.deprecation.as_deref(),
    );
    reading.scheme
}

/// The scheme a command that writes or allocates IDs needs, with the
/// deprecated key alias's notice printed once.
///
/// # Errors
///
/// A [`SchemeError`] from
/// [`work_adapters::config::resolve_scheme`].
pub fn announced_scheme_with_key(
    config: &dyn ConfigAccess,
) -> Result<WorkItemIdScheme, SchemeError> {
    resolve_scheme(config).map(announced)
}

/// The scheme as configured, whether or not a key pattern has its key, with
/// the deprecated key alias's notice printed once.
///
/// # Errors
///
/// A [`SchemeError`] from [`work_adapters::config::read_scheme`].
pub fn announced_scheme(
    config: &dyn ConfigAccess,
) -> Result<WorkItemIdScheme, SchemeError> {
    read_scheme(config).map(announced)
}

pub fn templates_dir(
    config: &dyn ConfigAccess,
) -> Result<String, kernel::Error> {
    effective_nonempty(config, "paths.templates")
}

pub fn configured_override(
    config: &dyn ConfigAccess,
    key: &str,
) -> Result<Option<String>, kernel::Error> {
    let parsed = Key::parse(key)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    Ok(config
        .effective_nonempty(&parsed, None)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?
        .configured_value())
}

pub fn resolve_work_dir(
    config: &dyn ConfigAccess,
    root: &Path,
) -> Result<PathBuf, kernel::Error> {
    let dirs = ::config::paths::doc_type_dirs(config)
        .map_err(|error| kernel::Error::Failed(error.to_string()))?;
    let relative = dirs
        .into_iter()
        .find(|resolved| resolved.doc_type == "work-item")
        .map(|resolved| resolved.dir)
        .ok_or_else(|| {
            kernel::Error::Failed(
                "no work-item doc-type directory configured".to_owned(),
            )
        })?;
    Ok(root.join(relative))
}
