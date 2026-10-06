//! `accelerator-research` — the `research fetch|topic|guard` sub-binary,
//! dispatched by the `accelerator` launcher.

mod cli;
mod context;
mod fetch_command;
mod guard;
#[cfg(feature = "test-loopback")]
mod loopback;
mod plan_wording;
mod render;
mod topic_command;
mod write_target;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Duration;

use clap::Parser as _;
use corpus::Clock as _;
use corpus::FilenameTimestampFormat;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RealFs;
use corpus_adapters::SystemClock as CorpusClock;
use research::conduct::ledger::RunId;
use research::sources::fetch::Clock;
use research::sources::fetch::FetchOutcome;
use research::sources::request::Endpoint;
use research::sources::request::FetchRequest;
use research::sources::schedule::Deadline;
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
use crate::cli::TopicAction;
use crate::context::ProjectContext;
use crate::fetch_command::ArxivAdapters;
use crate::fetch_command::FetchPorts;
use crate::fetch_command::Fetched;
use crate::fetch_command::OpenAlexAdapters;
use crate::fetch_command::SourceCall;
use crate::topic_command::OutstandingFlags;
use crate::topic_command::Printed;
use crate::topic_command::RunFlag;
use crate::topic_command::RunMode;

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
    let deadline =
        Deadline::starting(clock.now(), selected_call_budget(), REQUEST_BUDGET);
    match Cli::try_parse() {
        Ok(Cli {
            command:
                Command::Fetch {
                    family,
                    verb,
                    terms,
                    limit,
                },
        }) => fetch(clock, &deadline, &family, &verb, &terms, limit.as_deref()),
        Ok(Cli {
            command: Command::Topic { action },
        }) => topic(action),
        Ok(Cli {
            command: Command::Guard { .. },
        }) => guard::run(),
        Err(error) if invoked_as_guard() => {
            let _ = error.print();
            ExitCode::SUCCESS
        }
        Err(error) => error.exit(),
    }
}

/// A hook must never block by accident, and clap exits `2` on a usage error,
/// which Claude Code reads as a block.
fn invoked_as_guard() -> bool {
    std::env::args_os()
        .nth(1)
        .is_some_and(|verb| verb == "guard")
}

fn fetch(
    clock: Rc<dyn Clock>,
    deadline: &Deadline,
    family: &str,
    verb: &str,
    terms: &[String],
    limit: Option<&str>,
) -> ExitCode {
    let request = match FetchRequest::parse(family, verb, terms, limit) {
        Ok(request) => request,
        Err(error) => return usage(&error.to_string()),
    };
    let endpoints = match selected_endpoints() {
        Ok(endpoints) => endpoints,
        Err(message) => return usage(&message),
    };
    let project = match working_project() {
        Ok((_, project)) => project,
        Err(message) => return failure(&message),
    };
    let call = match source_call(request, endpoints, &project, &clock, deadline)
    {
        Ok(call) => call,
        Err(message) => return failure(&message),
    };
    let ports = FetchPorts {
        clock,
        credentials: config_adapters::credential_ports(
            vcs_adapters::InProcessTracking,
            &project.root,
            &project.root,
        ),
    };
    match fetch_command::run(&ports, &project, deadline, &call) {
        Ok(fetched) => report(&fetched),
        Err(error) => failure(&error.to_string()),
    }
}

fn topic(action: TopicAction) -> ExitCode {
    let printed = match action {
        TopicAction::Outstanding {
            slug,
            profiles_dir,
            depth,
            limit,
            start,
            run,
            spawned,
        } => OutstandingFlags::parse(
            &depth,
            limit.as_deref(),
            start,
            run.as_deref(),
            spawned.as_deref(),
        )
        .and_then(|flags| outstanding(&slug, &profiles_dir, flags)),
        TopicAction::EndRun { slug, run } => {
            topic_command::parse_run(&run).and_then(|run| end_run(&slug, &run))
        }
    };
    match printed {
        Ok(printed) => {
            eprint!("{}", printed.stderr);
            print!("{}", printed.stdout);
            ExitCode::SUCCESS
        }
        Err(message) => failure(&message),
    }
}

fn outstanding(
    slug: &str,
    profiles_dir: &Path,
    flags: OutstandingFlags,
) -> Result<Printed, String> {
    let (cwd, project) = working_project()?;
    let set_root = topic_command::resolve_set(&project, &cwd, slug)?;
    let mode = match flags.run {
        RunFlag::Query => RunMode::Query,
        RunFlag::Start => RunMode::Start(minted_run()?),
        RunFlag::Continue { run, spawned } => {
            RunMode::Continue { run, spawned }
        }
    };
    topic_command::run_outstanding(
        &set_root,
        profiles_dir,
        flags.depth,
        flags.limit,
        mode,
        &RealFs,
        &FileCorpusStore::new(&set_root),
    )
}

fn end_run(slug: &str, run: &RunId) -> Result<Printed, String> {
    let (cwd, project) = working_project()?;
    let set_root = topic_command::resolve_set(&project, &cwd, slug)?;
    topic_command::run_end_run(
        &set_root,
        run,
        &RealFs,
        &FileCorpusStore::new(&set_root),
    )
}

fn minted_run() -> Result<RunId, String> {
    let clock = CorpusClock::try_new().map_err(|error| error.to_string())?;
    let timestamp =
        clock.filename_timestamp(FilenameTimestampFormat::DateTimeUnderscored);
    Ok(RunId::mint(&timestamp, rand::random()))
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

fn working_project() -> Result<(PathBuf, ProjectContext), String> {
    let cwd = std::env::current_dir().map_err(|error| {
        format!("could not read the working directory: {error}")
    })?;
    let project =
        ProjectContext::enclosing(&cwd).map_err(|error| error.to_string())?;
    Ok((cwd, project))
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
fn selected_call_budget() -> Duration {
    loopback::call_budget().unwrap_or(CALL_BUDGET)
}

#[cfg(not(feature = "test-loopback"))]
const fn selected_call_budget() -> Duration {
    CALL_BUDGET
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
