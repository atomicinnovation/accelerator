//! Outbound (driven) adapters for the launcher shell.

pub mod exec;
pub mod resolve;
pub mod tls;

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::launch::core::{derive_override_var, ResolutionError};

/// Resolve the `ACCELERATOR_<SUB>_BIN` offline/air-gapped override for a
/// subcommand, if one is set — reading the real process environment.
///
/// This is the escape hatch honoured by every resolver *before* any fetch: for
/// subcommand `<sub>`, if the derived variable names a non-empty path, that path
/// is returned verbatim with no fetch and no checksum. Trusted-as-the-invoking
/// user by design — anyone who can set the launcher's environment can already run
/// arbitrary code as that user — so an active override is logged at `info`.
///
/// A non-UTF-8 subcommand name cannot derive a variable, so it yields `None`
/// (and normal resolution then reports it unresolved).
///
/// # Errors
///
/// [`ResolutionError::InvalidOverrideName`] if the name cannot derive a valid,
/// collision-free variable.
pub fn override_path(name: &OsStr) -> Result<Option<PathBuf>, ResolutionError> {
    override_path_from(name, |var| std::env::var_os(var))
}

/// The pure core of [`override_path`], with the environment lookup injected so
/// the derivation + selection is unit-testable in-process (no real env, no
/// races under threaded test runners).
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
        // A lookup that would return a path is ignored: the eligibility guard
        // fails closed before the variable is ever consulted.
        let resolved = override_path_from(OsStr::new("9lives"), |_| {
            Some(OsString::from("/should/not/be/used"))
        });
        assert!(matches!(
            resolved,
            Err(ResolutionError::InvalidOverrideName { .. })
        ));
    }
}
