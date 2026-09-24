//! The git excludes jj-cli layers beneath a snapshot's `.gitignore` files:
//! the file `core.excludesFile` names (or git's XDG default), then the
//! backing repository's `info/exclude`.

use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use jj_lib::gitignore::GitIgnoreFile;
use jj_lib::repo_path::RepoPath;
use jj_lib::store::Store;
use tracing::warn;

/// The ignores a snapshot starts from, before any `.gitignore` in the tree.
///
/// Reads `core.excludesFile` as jj-cli does: as a raw string, whatever git's
/// ownership trust says of the file it came from. An excludes file that cannot
/// be read is warned about and skipped, so the snapshot reports more changes
/// rather than failing.
pub(super) fn base_ignores(
    workspace_root: &Path,
    store: &Store,
) -> Arc<GitIgnoreFile> {
    let backing = jj_lib::git::get_git_repo(store).ok();
    let configured =
        backing.as_ref().map_or_else(global_excludes_file, |repo| {
            configured_excludes_file(&repo.config_snapshot())
        });
    let home = etcetera::home_dir().ok();
    let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME");
    let mut ignores = GitIgnoreFile::empty();
    if let Some(path) = excludes_file_path(
        configured.as_deref(),
        workspace_root,
        home.as_deref(),
        xdg_config_home.as_deref(),
    ) {
        ignores = chained_or_skipped(&ignores, &path);
    }
    if let Some(repo) = &backing {
        ignores = chained_or_skipped(
            &ignores,
            &repo.path().join("info").join("exclude"),
        );
    }
    ignores
}

fn configured_excludes_file(config: &gix::config::File<'_>) -> Option<String> {
    config
        .string("core.excludesFile")
        .map(|value| value.to_string())
}

fn global_excludes_file() -> Option<String> {
    match gix::config::File::from_globals() {
        Ok(config) => configured_excludes_file(&config),
        Err(error) => {
            warn!(%error, "could not read the global git config; no excludes file");
            None
        }
    }
}

fn chained_or_skipped(
    ignores: &Arc<GitIgnoreFile>,
    path: &Path,
) -> Arc<GitIgnoreFile> {
    ignores
        .chain_with_file(RepoPath::root(), path.to_path_buf())
        .unwrap_or_else(|error| {
            warn!(
                path = %path.display(),
                %error,
                "could not read a git excludes file; skipping it"
            );
            ignores.clone()
        })
}

fn excludes_file_path(
    configured: Option<&str>,
    workspace_root: &Path,
    home: Option<&Path>,
    xdg_config_home: Option<&OsStr>,
) -> Option<PathBuf> {
    configured.map_or_else(
        || git_default_excludes_file(home, xdg_config_home),
        |configured| configured_excludes_path(configured, workspace_root, home),
    )
}

fn configured_excludes_path(
    configured: &str,
    workspace_root: &Path,
    home: Option<&Path>,
) -> Option<PathBuf> {
    configured.strip_prefix("~/").map_or_else(
        || Some(workspace_root.join(configured)),
        |under_home| home.map(|home| home.join(under_home)),
    )
}

fn git_default_excludes_file(
    home: Option<&Path>,
    xdg_config_home: Option<&OsStr>,
) -> Option<PathBuf> {
    xdg_config_home.filter(|xdg| !xdg.is_empty()).map_or_else(
        || home.map(|home| home.join(".config/git/ignore")),
        |xdg| Some(Path::new(xdg).join("git/ignore")),
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;
    use std::path::PathBuf;

    use super::excludes_file_path;

    const ROOT: &str = "/work/repo";
    const HOME: &str = "/home/user";

    fn resolved(
        configured: Option<&str>,
        home: Option<&str>,
        xdg: Option<&str>,
    ) -> Option<PathBuf> {
        excludes_file_path(
            configured,
            Path::new(ROOT),
            home.map(Path::new),
            xdg.map(OsStr::new),
        )
    }

    #[test]
    fn an_absolute_configured_path_is_used_as_given() {
        assert_eq!(
            resolved(Some("/etc/ignore"), Some(HOME), None),
            Some(PathBuf::from("/etc/ignore"))
        );
    }

    #[test]
    fn a_tilde_path_resolves_against_home() {
        assert_eq!(
            resolved(Some("~/ignore-file"), Some(HOME), None),
            Some(PathBuf::from("/home/user/ignore-file"))
        );
    }

    #[test]
    fn a_tilde_path_without_home_resolves_to_nothing() {
        assert_eq!(resolved(Some("~/ignore-file"), None, None), None);
    }

    #[test]
    fn a_relative_configured_path_resolves_against_the_workspace_root() {
        assert_eq!(
            resolved(Some("ignore-file"), Some(HOME), None),
            Some(PathBuf::from("/work/repo/ignore-file"))
        );
    }

    #[test]
    fn unset_with_a_non_empty_xdg_uses_its_git_ignore() {
        assert_eq!(
            resolved(None, Some(HOME), Some("/xdg")),
            Some(PathBuf::from("/xdg/git/ignore"))
        );
    }

    #[test]
    fn unset_with_an_empty_or_unset_xdg_uses_the_home_default() {
        for xdg in [Some(""), None] {
            assert_eq!(
                resolved(None, Some(HOME), xdg),
                Some(PathBuf::from("/home/user/.config/git/ignore")),
                "{xdg:?}"
            );
        }
    }

    #[test]
    fn unset_without_home_or_xdg_resolves_to_nothing() {
        assert_eq!(resolved(None, None, None), None);
    }
}
