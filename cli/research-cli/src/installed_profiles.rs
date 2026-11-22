//! Where the plugin installs its research profiles.

use std::path::PathBuf;

const PROFILES_IN_PLUGIN: &str = "skills/research/profiles";

/// The profiles directory under the plugin root the launcher exports.
///
/// # Errors
///
/// `E_RESEARCH_PLUGIN_ROOT` when no plugin root is set.
pub fn profiles_dir(plugin_root: Option<PathBuf>) -> Result<PathBuf, String> {
    plugin_root
        .filter(|root| !root.as_os_str().is_empty())
        .map(|root| root.join(PROFILES_IN_PLUGIN))
        .ok_or_else(|| {
            "E_RESEARCH_PLUGIN_ROOT: ACCELERATOR_PLUGIN_ROOT is not set, so \
             the research profiles cannot be found; run accelerator through \
             its launcher"
                .to_owned()
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::profiles_dir;

    #[test]
    fn the_profiles_live_under_the_plugin_root() {
        assert_eq!(
            profiles_dir(Some("/plugin".into())),
            Ok(PathBuf::from("/plugin/skills/research/profiles"))
        );
    }

    #[test]
    fn an_unset_or_empty_plugin_root_names_the_variable() {
        for unset in [None, Some("".into())] {
            let resolved = profiles_dir(unset);
            assert!(
                resolved.as_ref().is_err_and(
                    |message| message.contains("ACCELERATOR_PLUGIN_ROOT")
                ),
                "{resolved:?}"
            );
        }
    }
}
