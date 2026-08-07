//! The composition-root helper every dispatch branch that needs
//! configuration calls, rather than re-deriving `config_adapters::compose`
//! and its error mapping independently.

use std::path::Path;
use std::path::PathBuf;

use config_adapters::FileConfigStore;
use config_adapters::LegacyPolicy;

/// The composed configuration service plus the discovered project root.
pub struct Composed {
    pub service: config::ConfigService<FileConfigStore, FileConfigStore>,
    pub project_root: PathBuf,
}

/// Wires the configuration ports at `cwd`'s project root, failing closed on
/// the legacy `.claude/accelerator.md` layout — matching bash's uniform
/// treatment of config/schema-resolution failures as ordinary (exit 1)
/// failures, not usage errors.
///
/// # Errors
///
/// A [`kernel::Error`] when the discovered root carries the legacy layout, or
/// a config level cannot be read.
pub fn compose(cwd: &Path) -> Result<Composed, kernel::Error> {
    let project_root = FileConfigStore::discover_root(cwd);
    let composed = config_adapters::compose(cwd, LegacyPolicy::Reject)?;
    Ok(Composed {
        service: composed.service,
        project_root,
    })
}

/// Resolves `paths.decisions`, absolute paths used as-is and relative paths
/// resolved against the discovered project root — matching
/// `adr-next-number.sh`'s own resolution rule.
///
/// # Errors
///
/// A [`kernel::Error`] when the key cannot be resolved.
pub fn resolve_decisions_dir(
    composed: &Composed,
) -> Result<PathBuf, kernel::Error> {
    let raw = config::paths::resolve_with_fallback(
        &composed.service,
        "decisions",
        None,
    )?;
    let path = PathBuf::from(raw);
    Ok(if path.is_absolute() {
        path
    } else {
        composed.project_root.join(path)
    })
}
