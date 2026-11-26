//! `accelerator-research` — the `research fetch|topic|guard` sub-binary,
//! dispatched by the `accelerator` launcher.

mod cli;
mod concurrency_wording;
mod consultable_command;
mod context;
mod counts_command;
mod counts_wording;
mod crossref_contact;
mod fetch_command;
mod guard;
mod harness_setting;
mod installed_profiles;
mod ledger_wording;
#[cfg(feature = "test-loopback")]
mod loopback;
mod plan_wording;
mod render;
mod report_path_command;
mod search_cap;
mod search_wording;
mod subagent_cap;
mod topic_command;
mod topic_set;
mod web_command;
mod write_target;

use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Duration;

use clap::Parser as _;
use config_adapters::credentials::SystemEnvironment;
use corpus::Clock as _;
use corpus::FilenameTimestampFormat;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::RealFs;
use corpus_adapters::SystemClock as CorpusClock;
use research::conduct::ledger::RunId;
use research::sources::metadata::SourceMetadata;
use research::sources::ports::Clock;
use research::sources::queue::Ticket;
use research::sources::queue::LONGEST_INVOCATION;
use research::sources::request::parse_ticket;
use research::sources::request::Endpoint;
use research::sources::request::FetchRequest;
use research::sources::schedule::Deadline;
use research::sources::web::fetch::WebFailure;
use research_adapters::arxiv_xml::XmlArxivDecoder;
use research_adapters::clock::SystemClock;
use research_adapters::confirmations::FileConfirmationCache;
use research_adapters::crossref_json::JsonCrossrefDecoder;
use research_adapters::diagnostics::Diagnostics;
use research_adapters::diagnostics::Stderr;
use research_adapters::host_resolver::DnsHostResolver;
use research_adapters::openalex_json::JsonOpenAlexDecoder;
use research_adapters::pacing::NoPacing;
use research_adapters::page_transport::HttpPageTransport;
use research_adapters::queue::random_nonce;
use research_adapters::scratch::ScratchDir;
use research_adapters::services::AdapterContext;
use research_adapters::services::PreviousRelease;
use research_adapters::services::ServiceAdapters;
use research_adapters::transport::HttpTransport;

use crate::cli::Cli;
use crate::cli::Command;
use crate::cli::TopicAction;
use crate::context::ProjectContext;
use crate::crossref_contact::CrossrefContact;
use crate::fetch_command::ArxivAdapters;
use crate::fetch_command::ArxivCall;
use crate::fetch_command::DoiAdapters;
use crate::fetch_command::DoiCall;
use crate::fetch_command::Fetched;
use crate::fetch_command::FetchedDoi;
use crate::fetch_command::FetchedRecords;
use crate::fetch_command::OpenAlexAdapters;
use crate::fetch_command::OpenAlexCall;
use crate::fetch_command::SourceCall;
use crate::topic_command::OutstandingFlags;
use crate::topic_command::PlanRequest;
use crate::topic_command::RunFlag;
use crate::topic_command::RunMode;
use crate::topic_set::Printed;
use crate::web_command::WebAdapters;

// A release build that turns debug assertions back on slips past this guard, so
// `main` reads the marker below: stripping and LTO then keep its bytes in the
// binary, where a scan can find them.
#[cfg(all(feature = "test-loopback", not(debug_assertions)))]
compile_error!("test-loopback must never be enabled in a release build");

#[cfg(feature = "test-loopback")]
const TEST_LOOPBACK_MARKER: &str = "ACCELERATOR_RESEARCH_TEST_LOOPBACK_MARKER";

/// Below Claude Code's default Bash timeout, so a call that cannot finish in
/// time prints its outcome, unavailable or waiting, rather than being killed.
const CALL_BUDGET: Duration = Duration::from_secs(100);
const _: () =
    assert!(CALL_BUDGET.as_millis() <= LONGEST_INVOCATION.as_millis());
const REQUEST_BUDGET: Duration = Duration::from_secs(30);

const USAGE: u8 = 2;

fn main() -> ExitCode {
    #[cfg(feature = "test-loopback")]
    std::hint::black_box(TEST_LOOPBACK_MARKER);
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
                    ticket,
                },
        }) => fetch(
            &clock,
            &deadline,
            &family,
            &verb,
            &terms,
            limit.as_deref(),
            ticket.as_deref(),
        ),
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
    clock: &Rc<dyn Clock>,
    deadline: &Deadline,
    family: &str,
    verb: &str,
    terms: &[String],
    limit: Option<&str>,
    ticket: Option<&str>,
) -> ExitCode {
    let parsed =
        FetchRequest::parse(family, verb, terms, limit).and_then(|request| {
            let presented = parse_ticket(request.family(), ticket)?;
            Ok((request, presented))
        });
    let (request, presented) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => return usage(&error.to_string()),
    };
    let call = match source_call(request, presented, clock, deadline) {
        Ok(call) => call,
        Err(refused) => return refused,
    };
    match fetch_command::run(clock.as_ref(), deadline, &call) {
        Ok(Fetched::Records(records)) => report(&records),
        Ok(Fetched::Page(page)) => report_page(page),
        Ok(Fetched::Doi(doi)) => report_doi(&doi),
        Err(error) => failure(&error.to_string()),
    }
}

fn topic(action: TopicAction) -> ExitCode {
    let printed = match action {
        TopicAction::Outstanding {
            slug,
            depth,
            concurrency,
            start,
            run,
            spawned,
            withhold_profile,
        } => OutstandingFlags::parse(
            &depth,
            concurrency.as_deref(),
            start,
            run.as_deref(),
            spawned.as_deref(),
            &withhold_profile,
        )
        .and_then(|flags| outstanding(&slug, flags)),
        TopicAction::EndRun { slug, run } => {
            topic_command::parse_run(&run).and_then(|run| end_run(&slug, &run))
        }
        TopicAction::ReportPath { slug, question } => {
            report_path_command::parse_question(&question)
                .and_then(|question| report_path(&slug, &question))
        }
        TopicAction::Counts { slug } => counts(&slug),
        TopicAction::Consultable { slug } => consultable(&slug),
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

fn outstanding(slug: &str, flags: OutstandingFlags) -> Result<Printed, String> {
    let profiles_dir = installed_profiles_dir()?;
    let (cwd, project) = working_project()?;
    let set_root = topic_set::resolve_set(&project, &cwd, slug)?;
    let concurrency = topic_command::resolve_concurrency(
        flags.requested_concurrency,
        || {
            project
                .configured_concurrency()
                .map_err(|error| error.to_string())
        },
        subagent_cap::from_environment(|name| std::env::var(name).ok()),
    )?;
    let mode = match flags.run {
        RunFlag::Query => RunMode::Query,
        RunFlag::Start => RunMode::Start(minted_run()?),
        RunFlag::Continue {
            run,
            spawned,
            withheld,
        } => RunMode::Continue {
            run,
            spawned,
            withheld,
        },
    };
    let request = PlanRequest {
        depth: flags.depth,
        concurrency,
        search_cap: search_cap::from_environment(|name| {
            std::env::var(name).ok()
        }),
        mode,
    };
    topic_command::run_outstanding(
        &set_root,
        &profiles_dir,
        &request,
        &RealFs,
        &FileCorpusStore::new(&set_root),
    )
}

fn end_run(slug: &str, run: &RunId) -> Result<Printed, String> {
    let (cwd, project) = working_project()?;
    let set_root = topic_set::resolve_set(&project, &cwd, slug)?;
    topic_command::run_end_run(
        &set_root,
        run,
        &RealFs,
        &FileCorpusStore::new(&set_root),
    )
}

fn report_path(slug: &str, question: &str) -> Result<Printed, String> {
    let (cwd, project) = working_project()?;
    let set_root = topic_set::resolve_set(&project, &cwd, slug)?;
    report_path_command::run_report_path(&set_root, question, &RealFs)
}

fn counts(slug: &str) -> Result<Printed, String> {
    let (cwd, project) = working_project()?;
    let set_root = topic_set::resolve_set(&project, &cwd, slug)?;
    counts_command::run_counts(&set_root, &RealFs)
}

fn consultable(slug: &str) -> Result<Printed, String> {
    let profiles_dir = installed_profiles_dir()?;
    let (cwd, project) = working_project()?;
    let set_root = topic_set::resolve_set(&project, &cwd, slug)?;
    consultable_command::run_consultable(&set_root, &profiles_dir, &RealFs)
}

fn installed_profiles_dir() -> Result<PathBuf, String> {
    installed_profiles::profiles_dir(config_adapters::plugin_root_from_env())
}

fn minted_run() -> Result<RunId, String> {
    let clock = CorpusClock::try_new().map_err(|error| error.to_string())?;
    let timestamp =
        clock.filename_timestamp(FilenameTimestampFormat::DateTimeUnderscored);
    Ok(RunId::mint(&timestamp, rand::random()))
}

/// Builds only the asked source's adapters, so no call reads a project,
/// configuration or credential its source never needs.
fn source_call(
    request: FetchRequest,
    presented: Option<Ticket>,
    clock: &Rc<dyn Clock>,
    deadline: &Deadline,
) -> Result<SourceCall, ExitCode> {
    Ok(match request {
        FetchRequest::OpenAlex(request) => {
            let endpoints =
                selected_endpoints().map_err(|error| usage(&error))?;
            let (_, project) =
                working_project().map_err(|error| failure(&error))?;
            let transport = http_transport(deadline)?;
            let credentials = config_adapters::credential_ports(
                vcs_adapters::InProcessTracking,
                &project.root,
                &project.root,
            );
            SourceCall::OpenAlex(
                OpenAlexCall { request, project },
                OpenAlexAdapters {
                    api: endpoints.openalex_api,
                    transport: Box::new(transport),
                    decoder: Box::new(JsonOpenAlexDecoder),
                    gate: Box::new(NoPacing),
                    credentials,
                },
            )
        }
        FetchRequest::Arxiv(request) => {
            let endpoints =
                selected_endpoints().map_err(|error| usage(&error))?;
            let (_, project) =
                working_project().map_err(|error| failure(&error))?;
            let context = queued_context(clock, &project)?;
            let transport = http_transport(deadline)?;
            let research = context.research.clone();
            let diagnostics = context.diagnostics.clone();
            let arxiv = ServiceAdapters::for_arxiv(
                &context,
                Some(PreviousRelease::arxiv_layout(&research)),
            );
            SourceCall::Arxiv(
                ArxivCall { request, presented },
                ArxivAdapters {
                    api: endpoints.arxiv_api,
                    oai: endpoints.arxiv_oai,
                    transport: Box::new(transport),
                    decoder: Box::new(XmlArxivDecoder),
                    gate: Box::new(arxiv.gate),
                    confirmations: Box::new(FileConfirmationCache::new(
                        research,
                        diagnostics,
                    )),
                    queue: Box::new(arxiv.queue),
                    contention: Box::new(arxiv.contention),
                },
            )
        }
        FetchRequest::Web(request) => SourceCall::Web(
            request,
            selected_web_adapters().map_err(|error| usage(&error))?,
        ),
        FetchRequest::Doi(request) => {
            let endpoints =
                selected_endpoints().map_err(|error| usage(&error))?;
            let (_, project) =
                working_project().map_err(|error| failure(&error))?;
            let context = queued_context(clock, &project)?;
            let transport = http_transport(deadline)?;
            let contact = CrossrefContact::resolved(
                &SystemEnvironment,
                project.config.as_ref(),
            );
            if let Some(warning) = contact.warning() {
                eprintln!("{warning}");
            }
            let crossref =
                ServiceAdapters::for_crossref(contact.pool(), &context);
            SourceCall::Doi(
                DoiCall {
                    request,
                    presented,
                    contact: contact.address().cloned(),
                },
                DoiAdapters {
                    service: crossref.service,
                    api: endpoints.crossref_api,
                    transport: Box::new(transport),
                    decoder: Box::new(JsonCrossrefDecoder),
                    gate: Box::new(crossref.gate),
                    queue: Box::new(crossref.queue),
                    contention: Box::new(crossref.contention),
                    diagnostics: context.diagnostics,
                },
            )
        }
    })
}

/// What every queued service's adapters share, rooted in the enclosing
/// project's research scratch directory.
fn queued_context(
    clock: &Rc<dyn Clock>,
    project: &ProjectContext,
) -> Result<AdapterContext, ExitCode> {
    let scratch = project
        .research_scratch()
        .map_err(|error| failure(&error.to_string()))?;
    let diagnostics: Rc<dyn Diagnostics> = Rc::new(Stderr);
    Ok(AdapterContext {
        research: ScratchDir::new(&project.root, &scratch),
        clock: clock.clone(),
        diagnostics,
        nonces: Rc::new(random_nonce),
    })
}

fn http_transport(deadline: &Deadline) -> Result<HttpTransport, ExitCode> {
    HttpTransport::new(deadline.per_request())
        .map_err(|error| failure(&error.to_string()))
}

fn working_project() -> Result<(PathBuf, ProjectContext), String> {
    let cwd = std::env::current_dir().map_err(|error| {
        format!("could not read the working directory: {error}")
    })?;
    let project =
        ProjectContext::enclosing(&cwd).map_err(|error| error.to_string())?;
    Ok((cwd, project))
}

fn report(fetched: &FetchedRecords) -> ExitCode {
    if let Some(rejection) = render::rejection(fetched) {
        return usage(&rejection);
    }
    if let Some(failure) = render::failure(fetched) {
        eprintln!("{failure}");
    }
    if let Some(summary) = render::summary(fetched) {
        eprintln!("{summary}");
    }
    render::document(fetched).map_or(ExitCode::FAILURE, |document| {
        println!("{document}");
        ExitCode::SUCCESS
    })
}

fn report_doi(fetched: &FetchedDoi) -> ExitCode {
    if let Some(rejection) = render::doi_rejection(fetched) {
        return usage(&rejection);
    }
    if let Some(summary) = render::doi_summary(fetched) {
        eprintln!("{summary}");
    }
    render::doi_document(fetched).map_or(ExitCode::FAILURE, |document| {
        println!("{document}");
        ExitCode::SUCCESS
    })
}

fn report_page(page: Result<SourceMetadata, WebFailure>) -> ExitCode {
    match page {
        Ok(metadata) => render::web_document(&metadata).map_or(
            ExitCode::FAILURE,
            |document| {
                println!("{document}");
                ExitCode::SUCCESS
            },
        ),
        Err(failed) => failure(&failed.to_string()),
    }
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

fn production_web_adapters() -> WebAdapters {
    WebAdapters {
        resolver: Box::new(DnsHostResolver),
        transport: Box::new(HttpPageTransport),
    }
}

#[cfg(feature = "test-loopback")]
fn selected_web_adapters() -> Result<WebAdapters, String> {
    let production = production_web_adapters();
    Ok(match loopback::stand_in()? {
        Some(stand_in) => loopback::through_stand_in(&stand_in, production),
        None => production,
    })
}

#[cfg(not(feature = "test-loopback"))]
#[allow(clippy::unnecessary_wraps)]
fn selected_web_adapters() -> Result<WebAdapters, String> {
    Ok(production_web_adapters())
}

struct Endpoints {
    openalex_api: Endpoint,
    arxiv_api: Endpoint,
    arxiv_oai: Endpoint,
    crossref_api: Endpoint,
}

#[cfg(feature = "test-loopback")]
fn selected_endpoints() -> Result<Endpoints, String> {
    Ok(Endpoints {
        openalex_api: loopback::openalex_api()?,
        arxiv_api: loopback::arxiv_api()?,
        arxiv_oai: loopback::arxiv_oai()?,
        crossref_api: loopback::crossref_api()?,
    })
}

#[cfg(not(feature = "test-loopback"))]
#[allow(clippy::unnecessary_wraps)]
fn selected_endpoints() -> Result<Endpoints, String> {
    Ok(Endpoints {
        openalex_api: Endpoint::openalex(),
        arxiv_api: Endpoint::arxiv_api(),
        arxiv_oai: Endpoint::arxiv_oai(),
        crossref_api: Endpoint::crossref_api(),
    })
}
