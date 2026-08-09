//! The `octocrab`-backed implementation of `collaboration`'s three GitHub
//! ports.

use std::time::Duration;

use octocrab::Octocrab;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Wraps an `octocrab::Octocrab` instance.
///
/// Exposes `async fn`-returning inherent methods that satisfy
/// `collaboration`'s three ports — the domain-port trait methods themselves
/// stay synchronous, so a small blocking shim in `collaboration-cli` bridges
/// each one to one of these via its own, non-nested `block_on` call.
pub struct OctocrabClient {
    client: Octocrab,
}

impl OctocrabClient {
    /// Production client, authenticated with `token`, talking to
    /// `https://api.github.com`.
    ///
    /// Bounded by connect/read/write timeouts so a stalled or maliciously
    /// slow connection cannot hang the CLI indefinitely — sized down from
    /// `cli/launcher`'s own `Fetcher` connect/total-timeout precedent, since
    /// these are small JSON request/response bodies, not multi-MB binary
    /// downloads. Redirect-following is excluded from this crate's
    /// `octocrab` feature set (`cli/Cargo.toml`), so the three REST paths
    /// this client calls — which don't legitimately redirect — surface an
    /// unexpected redirect as a REST-shaped error rather than silently
    /// following it, which also forecloses a redirect ever carrying the
    /// `Authorization` header to an unintended host.
    ///
    /// # Errors
    ///
    /// When the client cannot be built (e.g. an invalid token).
    pub fn new(token: String) -> Result<Self, String> {
        let client = Octocrab::builder()
            .personal_token(token)
            .set_connect_timeout(Some(CONNECT_TIMEOUT))
            .set_read_timeout(Some(READ_TIMEOUT))
            .set_write_timeout(Some(WRITE_TIMEOUT))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { client })
    }

    /// Test client pointed at a local mock server via `base_uri`,
    /// unauthenticated or with a fixed token.
    ///
    /// Deliberately **not** `#[cfg(test)]`-gated: the mock-server tests
    /// that call this live in `tests/`, a separate integration-test binary
    /// that links this crate as an ordinary external dependency and cannot
    /// see `#[cfg(test)]` items — mirroring `Fetcher::with_backoff`
    /// (`cli/launcher/src/launch/outbound/resolve/fetcher.rs`), the
    /// established precedent for this exact situation.
    ///
    /// # Errors
    ///
    /// When `base_uri` is not a valid URI, or the client cannot be built.
    pub fn with_base_uri(
        base_uri: http::Uri,
        token: Option<String>,
    ) -> Result<Self, String> {
        let mut builder = Octocrab::builder()
            .base_uri(base_uri)
            .map_err(|error| error.to_string())?;
        if let Some(token) = token {
            builder = builder.personal_token(token);
        }
        let client = builder
            .set_connect_timeout(Some(CONNECT_TIMEOUT))
            .set_read_timeout(Some(READ_TIMEOUT))
            .set_write_timeout(Some(WRITE_TIMEOUT))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { client })
    }

    /// Repository metadata, including its `parent` (if it is a fork).
    ///
    /// # Errors
    ///
    /// [`collaboration::ForgeApiError`] describing why the lookup failed.
    pub async fn repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<collaboration::RepositoryDetails, collaboration::ForgeApiError>
    {
        let repository = self
            .client
            .repos(owner, repo)
            .get()
            .await
            .map_err(map_error)?;
        let (parent_owner, parent_repo) =
            repository.parent.map_or((None, None), |parent| {
                (parent.owner.map(|author| author.login), Some(parent.name))
            });
        Ok(collaboration::RepositoryDetails {
            parent_owner,
            parent_repo,
        })
    }

    /// Confirms a pull request exists at `owner/repo` — the response body
    /// is not parsed for any field beyond a successful status; deriving
    /// owner/repo from this response would be circular, since a PR-scoped
    /// GET only ever returns PRs at the exact owner/repo already queried.
    ///
    /// # Errors
    ///
    /// [`collaboration::ForgeApiError`] describing why the check failed,
    /// including a REST 404 when the PR is not at this repository.
    pub async fn confirm_pull_request_exists(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<(), collaboration::ForgeApiError> {
        self.client
            .pulls(owner, repo)
            .get(pull_number)
            .await
            .map_err(map_error)?;
        Ok(())
    }

    /// # Errors
    ///
    /// [`collaboration::ForgeApiError`] describing why the update failed.
    pub async fn update_body(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        body: &str,
    ) -> Result<(), collaboration::ForgeApiError> {
        self.client
            .pulls(owner, repo)
            .update(pull_number)
            .body(body)
            .send()
            .await
            .map_err(map_error)?;
        Ok(())
    }
}

/// Maps every `octocrab::Error` variant to the domain's own, narrower
/// vocabulary: a REST-level status (`GitHubError`'s status code and
/// message) becomes `Status`; a response that deserialized into the wrong
/// shape (`Serde`/`Json`) becomes `Malformed` — the distinction that makes
/// `BaseRepoFailure::MalformedRepositoryResponse` constructible; every
/// other variant (network/transport failures below the HTTP layer) becomes
/// `Transport`.
fn map_error(error: octocrab::Error) -> collaboration::ForgeApiError {
    match error {
        octocrab::Error::GitHub { source, .. } => {
            collaboration::ForgeApiError::Status {
                code: source.status_code.as_u16(),
                message: source.message,
            }
        }
        octocrab::Error::Serde { source, .. } => {
            collaboration::ForgeApiError::Malformed(source.to_string())
        }
        octocrab::Error::Json { source, .. } => {
            collaboration::ForgeApiError::Malformed(source.to_string())
        }
        other => collaboration::ForgeApiError::Transport(other.to_string()),
    }
}
