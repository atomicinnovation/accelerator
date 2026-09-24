//! `accelerator-research` — the `research fetch` sub-binary, dispatched by
//! the `accelerator` launcher.

mod cli;
mod context;
mod fetch_command;
#[cfg(feature = "test-loopback")]
mod loopback;
mod provenance;
mod render;

use std::process::ExitCode;
use std::rc::Rc;
use std::time::Duration;

use clap::Parser as _;
use config_adapters::credentials::CredentialPorts;
use research::fetch::Clock;
use research::fetch::FetchOutcome;
use research::request::Endpoint;
use research::request::FetchRequest;
use research::schedule::Deadline;
use research_adapters::clock::SystemClock;
use research_adapters::openalex_json::JsonOpenAlexDecoder;
use research_adapters::pacing::NoPacing;
use research_adapters::transport::HttpTransport;

use crate::cli::Cli;
use crate::cli::Command;
use crate::context::ProjectContext;
use crate::fetch_command::FetchPorts;
use crate::fetch_command::Fetched;
use crate::fetch_command::OpenAlexAdapters;
use crate::fetch_command::Refusal;
use crate::provenance::VcsProvenance;

// The test-only loopback feature must never reach a release binary: the compile
// guard rests on `[profile.release]` keeping debug-assertions off, and a
// build-system byte scan of the staged binary greps for the marker below.
#[cfg(all(feature = "test-loopback", not(debug_assertions)))]
compile_error!("test-loopback must never be enabled in a release build");

#[cfg(feature = "test-loopback")]
#[no_mangle]
pub static ACCELERATOR_RESEARCH_TEST_LOOPBACK_MARKER: u8 = 0;

/// Below Claude Code's default Bash timeout, so a throttled source reports
/// itself unavailable rather than the call being killed.
const CALL_BUDGET: Duration = Duration::from_secs(100);
const REQUEST_BUDGET: Duration = Duration::from_secs(30);

const USAGE: u8 = 2;

fn main() -> ExitCode {
    let clock = selected_clock();
    let deadline = Deadline::starting(clock.now(), CALL_BUDGET, REQUEST_BUDGET);
    let Command::Fetch {
        family,
        verb,
        terms,
        limit,
    } = Cli::parse().command;
    let request =
        match FetchRequest::parse(&family, &verb, &terms, limit.as_deref()) {
            Ok(request) => request,
            Err(error) => return usage(&error.to_string()),
        };
    let openalex_api = match selected_openalex_api() {
        Ok(api) => api,
        Err(message) => return usage(&message),
    };
    let project = match std::env::current_dir()
        .map_err(|error| {
            format!("could not read the working directory: {error}")
        })
        .and_then(|cwd| {
            ProjectContext::enclosing(&cwd).map_err(|error| error.to_string())
        }) {
        Ok(project) => project,
        Err(message) => return failure(&message),
    };
    let transport = match HttpTransport::new(deadline.per_request()) {
        Ok(transport) => transport,
        Err(error) => return failure(&error.to_string()),
    };
    let ports = FetchPorts {
        clock,
        credentials: CredentialPorts::system(Box::new(
            VcsProvenance::discovered(project.root.clone()),
        )),
        openalex: OpenAlexAdapters {
            api: openalex_api,
            transport: Box::new(transport),
            decoder: Box::new(JsonOpenAlexDecoder),
            gate: Box::new(NoPacing),
        },
    };
    match fetch_command::run(&ports, &project, &deadline, &request) {
        Ok(fetched) => report(&fetched),
        Err(Refusal::Credential(error)) => failure(&error.to_string()),
        Err(Refusal::FamilyUnavailable(family)) => failure(&format!(
            "E_RESEARCH_UNSUPPORTED: fetching from {family} is not available \
             in this release"
        )),
    }
}

fn report(fetched: &Fetched) -> ExitCode {
    if let FetchOutcome::Failed(error) = &fetched.outcome {
        eprintln!("{error}");
    }
    if let Some(summary) = render::summary(fetched) {
        eprintln!("{summary}");
    }
    render::document(fetched).map_or(ExitCode::FAILURE, |document| {
        println!("{document}");
        ExitCode::SUCCESS
    })
}

fn usage(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(USAGE)
}

fn failure(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

#[cfg(feature = "test-loopback")]
fn selected_clock() -> Rc<dyn Clock> {
    loopback::logging_clock().map_or_else(
        || Rc::new(SystemClock) as Rc<dyn Clock>,
        |clock| Rc::new(clock),
    )
}

#[cfg(not(feature = "test-loopback"))]
fn selected_clock() -> Rc<dyn Clock> {
    Rc::new(SystemClock)
}

#[cfg(feature = "test-loopback")]
fn selected_openalex_api() -> Result<Endpoint, String> {
    Ok(loopback::openalex_api()?.unwrap_or_else(Endpoint::openalex))
}

#[cfg(not(feature = "test-loopback"))]
#[allow(clippy::unnecessary_wraps)]
fn selected_openalex_api() -> Result<Endpoint, String> {
    Ok(Endpoint::openalex())
}
