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
use research_adapters::arxiv_xml::XmlArxivDecoder;
use research_adapters::clock::SystemClock;
use research_adapters::confirmations::FileConfirmationCache;
use research_adapters::diagnostics::Diagnostics;
use research_adapters::diagnostics::Stderr;
use research_adapters::openalex_json::JsonOpenAlexDecoder;
use research_adapters::pacing::FilePacingGate;
use research_adapters::pacing::NoPacing;
use research_adapters::scratch::ScratchDir;
use research_adapters::transport::HttpTransport;

use crate::cli::Cli;
use crate::cli::Command;
use crate::context::ProjectContext;
use crate::fetch_command::ArxivAdapters;
use crate::fetch_command::FetchPorts;
use crate::fetch_command::Fetched;
use crate::fetch_command::OpenAlexAdapters;
use crate::fetch_command::SourceCall;
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
    let endpoints = match selected_endpoints() {
        Ok(endpoints) => endpoints,
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
    let call =
        match source_call(request, endpoints, &project, &clock, &deadline) {
            Ok(call) => call,
            Err(message) => return failure(&message),
        };
    let ports = FetchPorts {
        clock,
        credentials: CredentialPorts::system(Box::new(
            VcsProvenance::discovered(project.root.clone()),
        )),
    };
    match fetch_command::run(&ports, &project, &deadline, &call) {
        Ok(fetched) => report(&fetched),
        Err(error) => failure(&error.to_string()),
    }
}

/// Builds only the asked source's adapters, so an OpenAlex call never reads
/// the configuration arXiv's shared state needs.
fn source_call(
    request: FetchRequest,
    endpoints: Endpoints,
    project: &ProjectContext,
    clock: &Rc<dyn Clock>,
    deadline: &Deadline,
) -> Result<SourceCall, String> {
    let transport = HttpTransport::new(deadline.per_request())
        .map_err(|error| error.to_string())?;
    Ok(match request {
        FetchRequest::OpenAlex(request) => SourceCall::OpenAlex(
            request,
            OpenAlexAdapters {
                api: endpoints.openalex_api,
                transport: Box::new(transport),
                decoder: Box::new(JsonOpenAlexDecoder),
                gate: Box::new(NoPacing),
            },
        ),
        FetchRequest::Arxiv(request) => {
            let scratch = project
                .research_scratch()
                .map_err(|error| error.to_string())?;
            let diagnostics: Rc<dyn Diagnostics> = Rc::new(Stderr);
            SourceCall::Arxiv(
                request,
                ArxivAdapters {
                    api: endpoints.arxiv_api,
                    oai: endpoints.arxiv_oai,
                    transport: Box::new(transport),
                    decoder: Box::new(XmlArxivDecoder),
                    gate: Box::new(FilePacingGate::new(
                        ScratchDir::new(&project.root, &scratch),
                        clock.clone(),
                        diagnostics.clone(),
                    )),
                    confirmations: Box::new(FileConfirmationCache::new(
                        ScratchDir::new(&project.root, &scratch),
                        diagnostics,
                    )),
                },
            )
        }
    })
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

struct Endpoints {
    openalex_api: Endpoint,
    arxiv_api: Endpoint,
    arxiv_oai: Endpoint,
}

#[cfg(feature = "test-loopback")]
fn selected_endpoints() -> Result<Endpoints, String> {
    Ok(Endpoints {
        openalex_api: loopback::openalex_api()?,
        arxiv_api: loopback::arxiv_api()?,
        arxiv_oai: loopback::arxiv_oai()?,
    })
}

#[cfg(not(feature = "test-loopback"))]
#[allow(clippy::unnecessary_wraps)]
fn selected_endpoints() -> Result<Endpoints, String> {
    Ok(Endpoints {
        openalex_api: Endpoint::openalex(),
        arxiv_api: Endpoint::arxiv_api(),
        arxiv_oai: Endpoint::arxiv_oai(),
    })
}
