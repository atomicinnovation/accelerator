//! Resolving a pull request's base (upstream) owner/repo.

use std::path::Path;

use vcs::origin_remote::OriginRemote;

use crate::{
    resolve_origin_owner_repo, ForgeApiError, OwnerRepo, PullRequestExistence,
    RemoteUrlRecognizer, RepositoryLookup,
};

/// The result of resolving a PR's base repository.
///
/// Deliberately excludes a "resolved" arm on [`BaseRepoFailure`] itself:
/// composing `Resolved` and `Failed(BaseRepoFailure)` here rather than
/// folding success into the failure enum keeps a failure from ever being
/// able to carry a successful resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseRepoOutcome {
    Resolved(OwnerRepo),
    Failed(BaseRepoFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseRepoFailure {
    RepositoryLookupFailed(ForgeApiError),
    MalformedRepositoryResponse(String),
    MalformedParent,
    PullRequestLookupFailed(ForgeApiError),
}

/// Resolves a PR's base (upstream) owner/repo.
///
/// Parses the local repository's `origin` remote for a candidate
/// owner/repo, looks up that candidate's metadata to follow the forge's
/// `parent` field when it is a fork, then confirms the PR actually exists
/// at the resolved base — replicating `gh`'s own default fork-to-parent
/// resolution rather than assuming `origin` is already the upstream
/// repository.
///
/// # Errors
///
/// [`kernel::Error`] when the local repository's own `origin` remote
/// cannot be resolved — a `vcs`-level failure, surfaced before any network
/// call. Every forge-API-level failure is instead reported as
/// `Ok(BaseRepoOutcome::Failed(_))`, not an `Err`, since it is not a
/// failure of this function's own logic.
pub fn resolve_base_repository(
    root: &Path,
    origin_remote: &dyn OriginRemote,
    recognizer: &dyn RemoteUrlRecognizer,
    repository_lookup: &dyn RepositoryLookup,
    pull_request_existence: &dyn PullRequestExistence,
    pull_number: u64,
) -> Result<BaseRepoOutcome, kernel::Error> {
    let candidate = resolve_origin_owner_repo(root, origin_remote, recognizer)?;

    let details =
        match repository_lookup.repository(&candidate.owner, &candidate.repo) {
            Ok(details) => details,
            Err(ForgeApiError::Malformed(message)) => {
                return Ok(BaseRepoOutcome::Failed(
                    BaseRepoFailure::MalformedRepositoryResponse(message),
                ));
            }
            Err(error) => {
                return Ok(BaseRepoOutcome::Failed(
                    BaseRepoFailure::RepositoryLookupFailed(error),
                ));
            }
        };

    let base = match (details.parent_owner, details.parent_repo) {
        (Some(owner), Some(repo)) => OwnerRepo { owner, repo },
        (None, None) => candidate,
        (Some(_), None) | (None, Some(_)) => {
            return Ok(BaseRepoOutcome::Failed(
                BaseRepoFailure::MalformedParent,
            ));
        }
    };

    if let Err(error) = pull_request_existence.confirm_exists(
        &base.owner,
        &base.repo,
        pull_number,
    ) {
        return Ok(BaseRepoOutcome::Failed(
            BaseRepoFailure::PullRequestLookupFailed(error),
        ));
    }

    Ok(BaseRepoOutcome::Resolved(base))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use vcs::origin_remote::OriginRemote;

    use super::{resolve_base_repository, BaseRepoFailure, BaseRepoOutcome};
    use crate::{
        ForgeApiError, OwnerRepo, PullRequestExistence, RemoteUrlRecognizer,
        RepositoryLookup,
    };

    struct FixedOriginRemote(Result<Option<&'static str>, &'static str>);

    impl OriginRemote for FixedOriginRemote {
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

    fn configured_origin() -> FixedOriginRemote {
        FixedOriginRemote(Ok(Some(
            "https://example.test/candidate-owner/candidate-repo",
        )))
    }

    struct FixedRecognizer(Option<OwnerRepo>);

    impl RemoteUrlRecognizer for FixedRecognizer {
        fn parse(&self, _url: &str) -> Option<OwnerRepo> {
            self.0.clone()
        }
    }

    fn recognizes_candidate() -> FixedRecognizer {
        FixedRecognizer(Some(OwnerRepo {
            owner: "candidate-owner".to_owned(),
            repo: "candidate-repo".to_owned(),
        }))
    }

    struct FixedRepositoryLookup(
        Result<crate::RepositoryDetails, ForgeApiError>,
    );

    impl RepositoryLookup for FixedRepositoryLookup {
        fn repository(
            &self,
            _owner: &str,
            _repo: &str,
        ) -> Result<crate::RepositoryDetails, ForgeApiError> {
            self.0.clone()
        }
    }

    fn no_parent() -> FixedRepositoryLookup {
        FixedRepositoryLookup(Ok(crate::RepositoryDetails {
            parent_owner: None,
            parent_repo: None,
        }))
    }

    struct FixedPullRequestExistence(Result<(), ForgeApiError>);

    impl PullRequestExistence for FixedPullRequestExistence {
        fn confirm_exists(
            &self,
            _owner: &str,
            _repo: &str,
            _pull_number: u64,
        ) -> Result<(), ForgeApiError> {
            self.0.clone()
        }
    }

    fn pr_exists() -> FixedPullRequestExistence {
        FixedPullRequestExistence(Ok(()))
    }

    fn resolve(
        origin_remote: &dyn OriginRemote,
        recognizer: &dyn RemoteUrlRecognizer,
        repository_lookup: &dyn RepositoryLookup,
        pull_request_existence: &dyn PullRequestExistence,
    ) -> Result<BaseRepoOutcome, kernel::Error> {
        resolve_base_repository(
            &PathBuf::from("/repo"),
            origin_remote,
            recognizer,
            repository_lookup,
            pull_request_existence,
            42,
        )
    }

    #[test]
    fn branch_2_a_missing_origin_remote_propagates_as_a_kernel_error() {
        let result = resolve(
            &FixedOriginRemote(Ok(None)),
            &recognizes_candidate(),
            &no_parent(),
            &pr_exists(),
        );
        assert!(matches!(result, Err(kernel::Error::Refusal(_))));
    }

    #[test]
    fn branch_3_a_repository_lookup_transport_failure_is_reported(
    ) -> Result<(), kernel::Error> {
        let lookup = FixedRepositoryLookup(Err(ForgeApiError::Transport(
            "connection reset".to_owned(),
        )));
        let outcome = resolve(
            &configured_origin(),
            &recognizes_candidate(),
            &lookup,
            &pr_exists(),
        )?;
        assert_eq!(
            outcome,
            BaseRepoOutcome::Failed(BaseRepoFailure::RepositoryLookupFailed(
                ForgeApiError::Transport("connection reset".to_owned())
            ))
        );
        Ok(())
    }

    #[test]
    fn branch_3_a_repository_lookup_status_failure_is_reported(
    ) -> Result<(), kernel::Error> {
        let lookup = FixedRepositoryLookup(Err(ForgeApiError::Status {
            code: 404,
            message: "Not Found".to_owned(),
        }));
        let outcome = resolve(
            &configured_origin(),
            &recognizes_candidate(),
            &lookup,
            &pr_exists(),
        )?;
        assert_eq!(
            outcome,
            BaseRepoOutcome::Failed(BaseRepoFailure::RepositoryLookupFailed(
                ForgeApiError::Status {
                    code: 404,
                    message: "Not Found".to_owned()
                }
            ))
        );
        Ok(())
    }

    #[test]
    fn branch_4_a_malformed_repository_response_is_reported(
    ) -> Result<(), kernel::Error> {
        let lookup = FixedRepositoryLookup(Err(ForgeApiError::Malformed(
            "unexpected field shape".to_owned(),
        )));
        let outcome = resolve(
            &configured_origin(),
            &recognizes_candidate(),
            &lookup,
            &pr_exists(),
        )?;
        assert_eq!(
            outcome,
            BaseRepoOutcome::Failed(
                BaseRepoFailure::MalformedRepositoryResponse(
                    "unexpected field shape".to_owned()
                )
            )
        );
        Ok(())
    }

    #[test]
    fn branch_5_an_incomplete_parent_is_malformed() -> Result<(), kernel::Error>
    {
        let lookup = FixedRepositoryLookup(Ok(crate::RepositoryDetails {
            parent_owner: Some("upstream-owner".to_owned()),
            parent_repo: None,
        }));
        let outcome = resolve(
            &configured_origin(),
            &recognizes_candidate(),
            &lookup,
            &pr_exists(),
        )?;
        assert_eq!(
            outcome,
            BaseRepoOutcome::Failed(BaseRepoFailure::MalformedParent)
        );
        Ok(())
    }

    #[test]
    fn branch_6_a_pull_request_existence_failure_is_reported(
    ) -> Result<(), kernel::Error> {
        let existence = FixedPullRequestExistence(Err(ForgeApiError::Status {
            code: 404,
            message: "Not Found".to_owned(),
        }));
        let outcome = resolve(
            &configured_origin(),
            &recognizes_candidate(),
            &no_parent(),
            &existence,
        )?;
        assert_eq!(
            outcome,
            BaseRepoOutcome::Failed(BaseRepoFailure::PullRequestLookupFailed(
                ForgeApiError::Status {
                    code: 404,
                    message: "Not Found".to_owned()
                }
            ))
        );
        Ok(())
    }

    #[test]
    fn branch_7_no_parent_resolves_to_the_candidate(
    ) -> Result<(), kernel::Error> {
        let outcome = resolve(
            &configured_origin(),
            &recognizes_candidate(),
            &no_parent(),
            &pr_exists(),
        )?;
        assert_eq!(
            outcome,
            BaseRepoOutcome::Resolved(OwnerRepo {
                owner: "candidate-owner".to_owned(),
                repo: "candidate-repo".to_owned(),
            })
        );
        Ok(())
    }

    #[test]
    fn branch_8_a_fork_resolves_to_the_parent() -> Result<(), kernel::Error> {
        let lookup = FixedRepositoryLookup(Ok(crate::RepositoryDetails {
            parent_owner: Some("upstream-owner".to_owned()),
            parent_repo: Some("upstream-repo".to_owned()),
        }));
        let outcome = resolve(
            &configured_origin(),
            &recognizes_candidate(),
            &lookup,
            &pr_exists(),
        )?;
        assert_eq!(
            outcome,
            BaseRepoOutcome::Resolved(OwnerRepo {
                owner: "upstream-owner".to_owned(),
                repo: "upstream-repo".to_owned(),
            })
        );
        Ok(())
    }
}
