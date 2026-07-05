//! Outbound (driven) adapters for the launcher shell.

pub mod exec;
pub mod resolve;
pub mod tls;

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::launch::core::{derive_override_var, ResolutionError};

/// The `ACCELERATOR_<SUB>_BIN` offline override for a subcommand, if set.
///
/// Honoured before any fetch, returning the path unverified (trusted as the
/// invoking user). A non-UTF-8 name yields `None`.
///
/// # Errors
///
/// [`ResolutionError::InvalidOverrideName`] if the name cannot derive a valid
/// variable.
pub fn override_path(name: &OsStr) -> Result<Option<PathBuf>, ResolutionError> {
    override_path_from(name, |var| std::env::var_os(var))
}

/// [`override_path`] with the environment lookup injected for in-process tests.
fn override_path_from(
    name: &OsStr,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Result<Option<PathBuf>, ResolutionError> {
    let Some(name_str) = name.to_str() else {
        return Ok(None);
    };
    let var = derive_override_var(name_str)?;
    match lookup(&var) {
        Some(value) if !value.is_empty() => {
            let path = PathBuf::from(value);
            tracing::info!(
                subcommand = name_str,
                variable = var.as_str(),
                path = %path.display(),
                "resolving via ACCELERATOR_<SUB>_BIN override (no fetch)"
            );
            Ok(Some(path))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use crate::launch::core::ResolutionError;

    use super::override_path_from;

    #[test]
    fn a_set_override_variable_resolves_to_its_path() {
        let resolved = override_path_from(OsStr::new("frobnicate"), |var| {
            (var == "ACCELERATOR_FROBNICATE_BIN")
                .then(|| OsString::from("/opt/frob"))
        });
        assert_eq!(
            resolved.ok().flatten(),
            Some(std::path::PathBuf::from("/opt/frob"))
        );
    }

    #[test]
    fn an_unset_override_variable_yields_none() {
        let resolved = override_path_from(OsStr::new("frobnicate"), |_| None);
        assert!(matches!(resolved, Ok(None)));
    }

    #[test]
    fn an_empty_override_variable_is_treated_as_unset() {
        let resolved = override_path_from(OsStr::new("frobnicate"), |_| {
            Some(OsString::new())
        });
        assert!(matches!(resolved, Ok(None)));
    }

    #[test]
    fn an_ineligible_name_is_a_named_error_even_with_a_set_lookup() {
        let resolved = override_path_from(OsStr::new("9lives"), |_| {
            Some(OsString::from("/should/not/be/used"))
        });
        assert!(matches!(
            resolved,
            Err(ResolutionError::InvalidOverrideName { .. })
        ));
    }
}
