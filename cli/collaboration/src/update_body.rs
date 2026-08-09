//! Updating a pull request's body, resolving its base repository first.

use std::path::Path;

use vcs::origin_remote::OriginRemote;

use crate::base_repo::{
    resolve_base_repository, BaseRepoFailure, BaseRepoOutcome,
};
use crate::{
    ForgeApiError, PullRequestBodyUpdate, PullRequestExistence,
    RemoteUrlRecognizer, RepositoryLookup,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateBodyOutcome {
    Updated,
    BaseRepoResolutionFailed(BaseRepoFailure),
    PatchFailed(ForgeApiError),
}

/// Updates a pull request's body, resolving its base (upstream) repository
/// first.
///
/// # Errors
///
/// [`kernel::Error`] when the local repository's own `origin` remote
/// cannot be resolved, propagated unchanged from
/// [`resolve_base_repository`]. Every other failure — including any of the
/// resolver's own forge-API-level failures — is instead reported as
/// `Ok(UpdateBodyOutcome::BaseRepoResolutionFailed(_))` or
/// `Ok(UpdateBodyOutcome::PatchFailed(_))`, not an `Err`.
#[allow(clippy::too_many_arguments)]
pub fn update_pull_request_body(
    root: &Path,
    origin_remote: &dyn OriginRemote,
    recognizer: &dyn RemoteUrlRecognizer,
    repository_lookup: &dyn RepositoryLookup,
    pull_request_existence: &dyn PullRequestExistence,
    body_update: &dyn PullRequestBodyUpdate,
    pull_number: u64,
    body: &str,
) -> Result<UpdateBodyOutcome, kernel::Error> {
    let base = match resolve_base_repository(
        root,
        origin_remote,
        recognizer,
        repository_lookup,
        pull_request_existence,
        pull_number,
    )? {
        BaseRepoOutcome::Resolved(base) => base,
        BaseRepoOutcome::Failed(failure) => {
            return Ok(UpdateBodyOutcome::BaseRepoResolutionFailed(failure));
        }
    };

    match body_update.update_body(&base.owner, &base.repo, pull_number, body) {
        Ok(()) => Ok(UpdateBodyOutcome::Updated),
        Err(error) => Ok(UpdateBodyOutcome::PatchFailed(error)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use vcs::origin_remote::OriginRemote;

    use super::{update_pull_request_body, UpdateBodyOutcome};
    use crate::base_repo::BaseRepoFailure;
    use crate::{
        ForgeApiError, OwnerRepo, PullRequestBodyUpdate, PullRequestExistence,
        RemoteUrlRecognizer, RepositoryLookup,
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

    struct FixedBodyUpdate(Result<(), ForgeApiError>);

    impl PullRequestBodyUpdate for FixedBodyUpdate {
        fn update_body(
            &self,
            _owner: &str,
            _repo: &str,
            _pull_number: u64,
            _body: &str,
        ) -> Result<(), ForgeApiError> {
            self.0.clone()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn update(
        origin_remote: &dyn OriginRemote,
        recognizer: &dyn RemoteUrlRecognizer,
        repository_lookup: &dyn RepositoryLookup,
        pull_request_existence: &dyn PullRequestExistence,
        body_update: &dyn PullRequestBodyUpdate,
    ) -> Result<UpdateBodyOutcome, kernel::Error> {
        update_pull_request_body(
            &PathBuf::from("/repo"),
            origin_remote,
            recognizer,
            repository_lookup,
            pull_request_existence,
            body_update,
            42,
            "new body",
        )
    }

    #[test]
    fn a_missing_origin_remote_propagates_as_a_kernel_error() {
        let result = update(
            &FixedOriginRemote(Ok(None)),
            &recognizes_candidate(),
            &no_parent(),
            &pr_exists(),
            &FixedBodyUpdate(Ok(())),
        );
        assert!(matches!(result, Err(kernel::Error::Refusal(_))));
    }

    #[test]
    fn branch_3_a_base_repo_resolution_failure_is_reported(
    ) -> Result<(), kernel::Error> {
        let lookup = FixedRepositoryLookup(Err(ForgeApiError::Transport(
            "connection reset".to_owned(),
        )));
        let outcome = update(
            &configured_origin(),
            &recognizes_candidate(),
            &lookup,
            &pr_exists(),
            &FixedBodyUpdate(Ok(())),
        )?;
        assert_eq!(
            outcome,
            UpdateBodyOutcome::BaseRepoResolutionFailed(
                BaseRepoFailure::RepositoryLookupFailed(
                    ForgeApiError::Transport("connection reset".to_owned())
                )
            )
        );
        Ok(())
    }

    #[test]
    fn branch_4_a_patch_failure_is_reported() -> Result<(), kernel::Error> {
        let body_update = FixedBodyUpdate(Err(ForgeApiError::Status {
            code: 422,
            message: "Validation Failed".to_owned(),
        }));
        let outcome = update(
            &configured_origin(),
            &recognizes_candidate(),
            &no_parent(),
            &pr_exists(),
            &body_update,
        )?;
        assert_eq!(
            outcome,
            UpdateBodyOutcome::PatchFailed(ForgeApiError::Status {
                code: 422,
                message: "Validation Failed".to_owned()
            })
        );
        Ok(())
    }

    #[test]
    fn a_successful_update_reports_updated() -> Result<(), kernel::Error> {
        let outcome = update(
            &configured_origin(),
            &recognizes_candidate(),
            &no_parent(),
            &pr_exists(),
            &FixedBodyUpdate(Ok(())),
        )?;
        assert_eq!(outcome, UpdateBodyOutcome::Updated);
        Ok(())
    }
}
