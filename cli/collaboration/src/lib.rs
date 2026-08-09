//! The PR-helper domain: resolving a pull request's base (upstream)
//! repository, and updating a pull request's body, against a mockable
//! hosted forge's REST surface.

use std::path::Path;

pub mod base_repo;
pub mod update_body;

/// The owner and repository name identifying a repository on some hosted
/// forge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerRepo {
    pub owner: String,
    pub repo: String,
}

/// Recognizes and parses a remote URL according to one forge's own
/// conventions.
///
/// A port rather than a function living directly in this crate, and
/// deliberately narrow — recognize-or-not, not "recognize, and also fetch
/// metadata" — so a second forge (GitLab, Bitbucket) is just another
/// implementation a new forge crate supplies, and this crate never needs to
/// know a second forge exists. Only one implementation exists today
/// (`GitHubRemoteUrlRecognizer`, in the `github` crate); a future composite
/// trying several recognizers in turn, or preferring a configured forge, is
/// itself just another implementation of this same port, assembled by
/// whichever crate wires up more than one.
pub trait RemoteUrlRecognizer {
    /// `None` when `url` does not match this recognizer's forge/shape.
    fn parse(&self, url: &str) -> Option<OwnerRepo>;
}

/// The local repository's owner/repo, resolved from its `origin` remote.
///
/// `Err` when no `origin` remote is configured (the caller-facing "no
/// default remote repository" case), when the probe itself fails, or when
/// the configured URL is not recognized by `recognizer`.
///
/// # Errors
///
/// See above.
pub fn resolve_origin_owner_repo(
    root: &Path,
    origin_remote: &dyn vcs::origin_remote::OriginRemote,
    recognizer: &dyn RemoteUrlRecognizer,
) -> Result<OwnerRepo, kernel::Error> {
    let Some(url) = origin_remote.origin_url(root)? else {
        return Err(kernel::Error::Refusal(
            "no origin remote is configured for this repository".to_owned(),
        ));
    };
    recognizer.parse(&url).ok_or_else(|| {
        kernel::Error::Refusal(format!(
            "the origin remote '{url}' is not a recognized remote URL"
        ))
    })
}

/// A repository's metadata, as returned by a forge's repository-get
/// endpoint — specifically whether it has a `parent` (i.e. is a fork), and
/// if so, the parent's owner/repo.
pub trait RepositoryLookup {
    /// # Errors
    ///
    /// [`ForgeApiError`] describing why the lookup failed.
    fn repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<RepositoryDetails, ForgeApiError>;
}

/// Confirms a pull request exists at a given repository.
///
/// Used only to validate the resolved base repo actually holds the PR,
/// never to derive owner/repo from its response — a single PR-scoped GET
/// only ever returns PRs at the exact owner/repo queried, so it cannot
/// reveal a different repo than what was already asked for.
pub trait PullRequestExistence {
    /// # Errors
    ///
    /// [`ForgeApiError`] describing why the check failed, including a
    /// REST 404 when the PR is not at this repository.
    fn confirm_exists(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<(), ForgeApiError>;
}

pub trait PullRequestBodyUpdate {
    /// # Errors
    ///
    /// [`ForgeApiError`] describing why the update failed.
    fn update_body(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        body: &str,
    ) -> Result<(), ForgeApiError>;
}

/// The subset of a GET repository response this domain needs.
///
/// The raw parent owner/name, still unvalidated — a present-but-incomplete
/// parent (one field set, the other missing) is this crate's job to reject
/// as [`base_repo::BaseRepoFailure::MalformedParent`], not the adapter's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDetails {
    pub parent_owner: Option<String>,
    pub parent_repo: Option<String>,
}

/// Why a forge REST call did not succeed, carrying enough to report a
/// non-zero exit code with the REST error's status code and message on
/// stderr.
///
/// `Malformed` is a distinct variant from `Transport` — not merged into it —
/// specifically so [`base_repo::BaseRepoFailure::MalformedRepositoryResponse`]
/// is constructible: a response that deserializes into the wrong shape is a
/// materially different failure from a connection dropping or timing out,
/// and collapsing the two would make that branch dead code no test could
/// ever exercise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeApiError {
    Transport(String),
    Malformed(String),
    Status { code: u16, message: String },
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use vcs::origin_remote::OriginRemote;

    use super::{resolve_origin_owner_repo, OwnerRepo, RemoteUrlRecognizer};

    struct StubOriginRemote(Result<Option<&'static str>, &'static str>);

    impl OriginRemote for StubOriginRemote {
        fn origin_url(
            &self,
            _root: &Path,
        ) -> Result<Option<String>, kernel::Error> {
            match &self.0 {
                Ok(url) => Ok(url.map(str::to_owned)),
                Err(message) => {
                    Err(kernel::Error::Failed((*message).to_owned()))
                }
            }
        }
    }

    struct StubRecognizer(Option<OwnerRepo>);

    impl RemoteUrlRecognizer for StubRecognizer {
        fn parse(&self, _url: &str) -> Option<OwnerRepo> {
            self.0.clone()
        }
    }

    #[test]
    fn resolves_the_owner_repo_a_recognizer_parses() -> Result<(), kernel::Error>
    {
        let origin =
            StubOriginRemote(Ok(Some("https://example.test/owner/repo")));
        let recognizer = StubRecognizer(Some(OwnerRepo {
            owner: "owner".to_owned(),
            repo: "repo".to_owned(),
        }));
        let resolved = resolve_origin_owner_repo(
            &PathBuf::from("/repo"),
            &origin,
            &recognizer,
        )?;
        assert_eq!(
            resolved,
            OwnerRepo {
                owner: "owner".to_owned(),
                repo: "repo".to_owned(),
            }
        );
        Ok(())
    }

    #[test]
    fn no_origin_remote_configured_is_a_refusal() {
        let origin = StubOriginRemote(Ok(None));
        let recognizer = StubRecognizer(None);
        let result = resolve_origin_owner_repo(
            &PathBuf::from("/repo"),
            &origin,
            &recognizer,
        );
        assert!(matches!(result, Err(kernel::Error::Refusal(_))));
    }

    #[test]
    fn an_unrecognized_remote_shape_is_a_refusal() {
        let origin =
            StubOriginRemote(Ok(Some("https://example.test/owner/repo")));
        let recognizer = StubRecognizer(None);
        let result = resolve_origin_owner_repo(
            &PathBuf::from("/repo"),
            &origin,
            &recognizer,
        );
        assert!(matches!(result, Err(kernel::Error::Refusal(_))));
    }

    #[test]
    fn a_probe_failure_propagates() {
        let origin = StubOriginRemote(Err("could not read git config"));
        let recognizer = StubRecognizer(None);
        let result = resolve_origin_owner_repo(
            &PathBuf::from("/repo"),
            &origin,
            &recognizer,
        );
        assert!(matches!(result, Err(kernel::Error::Failed(_))));
    }
}
