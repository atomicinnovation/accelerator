//! `accelerator-collaboration` — the `collaboration pr base-repo|update-body`
//! sub-binary, dispatched by the `accelerator` launcher.

mod auth;
mod cli;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser as _;
use collaboration::base_repo::resolve_base_repository;
use collaboration::base_repo::BaseRepoFailure;
use collaboration::base_repo::BaseRepoOutcome;
use collaboration::update_body::update_pull_request_body;
use collaboration::update_body::UpdateBodyOutcome;
use collaboration::ForgeApiError;
use collaboration::PullRequestBodyUpdate;
use collaboration::PullRequestExistence;
use collaboration::RepositoryDetails;
use collaboration::RepositoryLookup;
use config::ConfigAccess;
use config_adapters::compose;
use config_adapters::LegacyPolicy;
use github::GitHubRemoteUrlRecognizer;
use github::OctocrabClient;
use vcs_adapters::library::InProcessProbe;

use crate::cli::Cli;
use crate::cli::Command;
use crate::cli::PrAction;

fn current_dir() -> Result<PathBuf, kernel::Error> {
    std::env::current_dir().map_err(|error| {
        kernel::Error::Failed(format!(
            "could not read the current directory: {error}"
        ))
    })
}

/// The GitHub REST API's base URI, pinned to `https://api.github.com` and
/// overridable by `ACCELERATOR_COLLABORATION_GITHUB_API_URL` — a test-only
/// seam (mirroring the launcher's own `ACCELERATOR_RELEASE_BASE_URL`) that
/// lets the end-to-end binary tests point at a local mock server.
fn github_api_base_uri() -> Option<http::Uri> {
    let raw = std::env::var_os("ACCELERATOR_COLLABORATION_GITHUB_API_URL")?;
    raw.to_string_lossy().parse().ok()
}

/// Bridges `collaboration`'s three synchronous ports to `OctocrabClient`'s
/// `async fn`s. Each trait method does its own, non-nested `block_on` call;
/// `main()` never enters the runtime itself, since a `block_on`-driven async
/// block that then called back into a synchronous port method calling
/// `block_on` again would be a nested runtime-enter on the same thread,
/// which Tokio panics on.
struct BlockingGitHubClient {
    runtime: tokio::runtime::Runtime,
    client: OctocrabClient,
}

impl RepositoryLookup for BlockingGitHubClient {
    fn repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<RepositoryDetails, ForgeApiError> {
        self.runtime.block_on(self.client.repository(owner, repo))
    }
}

impl PullRequestExistence for BlockingGitHubClient {
    fn confirm_exists(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
    ) -> Result<(), ForgeApiError> {
        self.runtime
            .block_on(self.client.confirm_pull_request_exists(
                owner,
                repo,
                pull_number,
            ))
    }
}

impl PullRequestBodyUpdate for BlockingGitHubClient {
    fn update_body(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        body: &str,
    ) -> Result<(), ForgeApiError> {
        self.runtime.block_on(self.client.update_body(
            owner,
            repo,
            pull_number,
            body,
        ))
    }
}

fn build_blocking_client(
    config: &dyn ConfigAccess,
) -> Result<BlockingGitHubClient, kernel::Error> {
    let token = auth::resolve_github_token(config)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            kernel::Error::Failed(format!(
                "could not start the async runtime: {error}"
            ))
        })?;
    // `Octocrab::builder()...build()` unconditionally wraps its service in a
    // `tower::Buffer`, which spawns its worker task via `tokio::spawn` at
    // construction time — this needs an *entered* runtime, not merely a
    // `Runtime` value to `block_on` with later, so construction happens
    // under this guard rather than inside `block_on`'s own future.
    let client = {
        let _guard = runtime.enter();
        match github_api_base_uri() {
            Some(base_uri) => {
                OctocrabClient::with_base_uri(base_uri, Some(token.value))
            }
            None => OctocrabClient::new(token.value),
        }
    }
    .map_err(kernel::Error::Failed)?;
    Ok(BlockingGitHubClient { runtime, client })
}

/// `Transport`/`Malformed` carry an already-formatted, human-readable
/// string; only `Status` has a separate numeric field to interpolate.
fn render_forge_api_error(error: &ForgeApiError) -> String {
    match error {
        ForgeApiError::Status { code, message } => {
            format!("{code}: {message}")
        }
        ForgeApiError::Transport(message)
        | ForgeApiError::Malformed(message) => message.clone(),
    }
}

fn render_base_repo_failure(failure: &BaseRepoFailure) -> String {
    match failure {
        BaseRepoFailure::RepositoryLookupFailed(error)
        | BaseRepoFailure::PullRequestLookupFailed(error) => {
            render_forge_api_error(error)
        }
        BaseRepoFailure::MalformedRepositoryResponse(message) => {
            message.clone()
        }
        BaseRepoFailure::MalformedParent => {
            "the repository's parent field was inconsistent: only one of \
             owner/name was present"
                .to_owned()
        }
    }
}

fn run_base_repo(pull_number: u64) -> Result<(), kernel::Error> {
    let start = current_dir()?;
    let composed = compose(&start, LegacyPolicy::Reject)?;
    let service: &dyn ConfigAccess = &composed.service;
    let client = build_blocking_client(service)?;
    let origin_remote = InProcessProbe;
    let recognizer = GitHubRemoteUrlRecognizer;

    match resolve_base_repository(
        &start,
        &origin_remote,
        &recognizer,
        &client,
        &client,
        pull_number,
    )? {
        BaseRepoOutcome::Resolved(owner_repo) => {
            println!("{}/{}", owner_repo.owner, owner_repo.repo);
            Ok(())
        }
        BaseRepoOutcome::Failed(failure) => {
            Err(kernel::Error::Failed(render_base_repo_failure(&failure)))
        }
    }
}

fn run_update_body(
    pull_number: u64,
    body_file: &Path,
) -> Result<(), kernel::Error> {
    let body = std::fs::read_to_string(body_file).map_err(|error| {
        kernel::Error::Refusal(format!(
            "could not read --body-file {}: {error}",
            body_file.display()
        ))
    })?;
    let start = current_dir()?;
    let composed = compose(&start, LegacyPolicy::Reject)?;
    let service: &dyn ConfigAccess = &composed.service;
    let client = build_blocking_client(service)?;
    let origin_remote = InProcessProbe;
    let recognizer = GitHubRemoteUrlRecognizer;

    match update_pull_request_body(
        &start,
        &origin_remote,
        &recognizer,
        &client,
        &client,
        &client,
        pull_number,
        &body,
    )? {
        UpdateBodyOutcome::Updated => Ok(()),
        UpdateBodyOutcome::BaseRepoResolutionFailed(failure) => {
            Err(kernel::Error::Failed(render_base_repo_failure(&failure)))
        }
        UpdateBodyOutcome::PatchFailed(error) => {
            Err(kernel::Error::Failed(render_forge_api_error(&error)))
        }
    }
}

fn report(error: &kernel::Error) -> ExitCode {
    let message = error.to_string();
    if !message.is_empty() {
        eprintln!("{message}");
    }
    match error {
        kernel::Error::Refusal(_) => ExitCode::from(2),
        _ => ExitCode::FAILURE,
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Pr {
            action: PrAction::BaseRepo { pull_number },
        } => run_base_repo(pull_number),
        Command::Pr {
            action:
                PrAction::UpdateBody {
                    pull_number,
                    body_file,
                },
        } => run_update_body(pull_number, &body_file),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(&error),
    }
}
