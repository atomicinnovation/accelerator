//! Shared config-resolution helpers used by more than one subcommand's
//! adapter wiring: the `work.id_pattern`/`work.default_project_code`
//! scheme, and the configured work-item directory.

use std::path::Path;
use std::path::PathBuf;

use ::config::ConfigAccess;
use ::config::Key;
use corpus::WorkItemIdScheme;

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

pub fn resolve_scheme(
    config: &dyn ConfigAccess,
) -> Result<WorkItemIdScheme, kernel::Error> {
    let id_pattern = effective_nonempty(config, "work.id_pattern")?;
    let default_project_code =
        effective_nonempty(config, "work.default_project_code")?;
    Ok(WorkItemIdScheme {
        id_pattern,
        default_project_code: (!default_project_code.is_empty())
            .then_some(default_project_code),
    })
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
