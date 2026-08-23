//! `accelerator-linear` — the linear create|update|show|search|comment|
//! transition|attach|init sub-binary, dispatched by the `accelerator` launcher.

mod cli;
mod context;
mod exit_codes;
mod keywords;
mod work_item;

use std::path::Path;
use std::process::ExitCode;

use clap::Parser as _;
use linear_client::cache::{LinearCache, SystemFilesystem};
use linear_client::Search;
use serde_json::{json, Value};
use tracker::ExternalId;

use crate::cli::{
    AttachArgs, Cli, Command, CommentAction, CreateArgs, InitAction,
    SearchArgs, ShowArgs, TransitionArgs, UpdateArgs,
};
use crate::context::ContextError;

// The test-only loopback feature must never reach a release binary: the compile
// guard rests on `[profile.release]` keeping debug-assertions off, and a
// build-system byte scan of the staged binary greps for the marker below.
#[cfg(all(feature = "test-loopback", not(debug_assertions)))]
compile_error!("test-loopback must never be enabled in a release build");

#[cfg(feature = "test-loopback")]
#[no_mangle]
pub static ACCELERATOR_LINEAR_TEST_LOOPBACK_MARKER: u8 = 0;

fn id_of(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

/// Builds the client, or prints the failure and returns the mapped exit code.
fn client_or_report() -> Result<context::Built, ExitCode> {
    context::build_client().map_err(|error| match error {
        ContextError::BadApiUrl(raw) => {
            eprintln!(
                "E_BAD_API_URL: ACCELERATOR_LINEAR_API_URL={raw:?} is not an \
                 admissible https *.linear.app destination"
            );
            ExitCode::from(exit_codes::USAGE)
        }
        ContextError::Config(message) => {
            eprintln!("{message}");
            ExitCode::from(exit_codes::ERROR)
        }
        ContextError::Client(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_client(&error))
        }
    })
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| value.to_string())
    );
}

fn run_show(args: &ShowArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.show_detailed(&id_of(&args.id), args.comments) {
        Ok(body) => {
            let issue = body.pointer("/data/issue");
            let keyword = if issue.is_none() || issue == Some(&Value::Null) {
                keywords::Show::NotFound
            } else {
                keywords::Show::Found
            };
            print_json(&keywords::with_outcome(body, keyword.keyword()));
            match keyword {
                keywords::Show::Found => ExitCode::SUCCESS,
                keywords::Show::NotFound => {
                    ExitCode::from(exit_codes::SHOW_NOT_FOUND)
                }
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        }
    }
}

fn run_search(args: SearchArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    let search = Search {
        team_id: None,
        state: args.state,
        assignee: args.assignee,
        label: args.label,
        text: args.text,
    };
    if !args.quiet {
        if let Ok(filter) =
            linear_client::filter::compose(&search, client.states())
        {
            eprintln!("INFO: composed IssueFilter: {filter}");
        }
    }
    match client.search_detailed(&search) {
        Ok(page) => {
            let keyword = if page.nodes.is_empty() {
                keywords::Search::Empty
            } else if page.truncated {
                keywords::Search::Truncated
            } else {
                keywords::Search::Results
            };
            let envelope = json!({
                "data": {"issues": {
                    "nodes": page.nodes,
                    "truncated": page.truncated,
                }},
            });
            print_json(&keywords::with_outcome(envelope, keyword.keyword()));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        }
    }
}

fn run_comment(action: CommentAction) -> ExitCode {
    let CommentAction::Add(args) = action;
    let body =
        match resolve_body(args.body.as_deref(), args.body_file.as_deref()) {
            Ok(body) => body,
            Err(code) => return code,
        };
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.add_comment(&args.id, &body) {
        Ok(response) => {
            print_json(&keywords::with_outcome(
                response,
                keywords::Comment::Added.keyword(),
            ));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        }
    }
}

fn resolve_body(
    body: Option<&str>,
    body_file: Option<&Path>,
) -> Result<String, ExitCode> {
    match (body, body_file) {
        (Some(body), _) => Ok(body.to_owned()),
        (None, Some(path)) => std::fs::read_to_string(path).map_err(|error| {
            eprintln!("E_COMMENT_NO_BODY: could not read --body-file: {error}");
            ExitCode::from(exit_codes::COMMENT_NO_BODY)
        }),
        (None, None) => {
            eprintln!("E_COMMENT_NO_BODY: a --body or --body-file is required");
            Err(ExitCode::from(exit_codes::COMMENT_NO_BODY))
        }
    }
}

fn run_transition(args: TransitionArgs) -> ExitCode {
    let Some(state) = args.state.or(args.state_flag) else {
        eprintln!("E_TRANSITION_NO_STATE: a target state is required");
        return ExitCode::from(exit_codes::TRANSITION_NO_STATE);
    };
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.transition(&args.id, &state) {
        Ok(response) => {
            print_json(&keywords::with_outcome(
                response,
                keywords::Transition::Transitioned.keyword(),
            ));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        }
    }
}

fn run_attach(args: &AttachArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    let result = match (args.url.as_deref(), args.file.as_deref()) {
        (Some(_), Some(_)) => {
            eprintln!("E_ATTACH_BOTH_TARGETS: pass one of --url or --file");
            return ExitCode::from(exit_codes::ATTACH_BOTH_TARGETS);
        }
        (None, None) => {
            eprintln!("E_ATTACH_NO_TARGET: pass one of --url or --file");
            return ExitCode::from(exit_codes::ATTACH_NO_TARGET);
        }
        (Some(url), None) => {
            client.attach_link(&args.id, url, args.title.as_deref())
        }
        (None, Some(path)) => {
            client.attach_file(&args.id, path, args.title.as_deref())
        }
    };
    match result {
        Ok(response) => {
            print_json(&keywords::with_outcome(
                response,
                keywords::Attach::Attached.keyword(),
            ));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        }
    }
}

fn run_update(args: UpdateArgs) -> ExitCode {
    if args.title.is_none()
        && args.description.is_none()
        && args.state.is_none()
        && args.assignee_id.is_none()
        && args.priority.is_none()
    {
        eprintln!("E_UPDATE_NO_OPS: at least one field to update is required");
        return ExitCode::from(exit_codes::UPDATE_NO_OPS);
    }
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    // A state change routes through the transition mutation; a plain field
    // update through the port's whole-item update.
    if let Some(state) = args.state.as_deref() {
        match client.transition(&args.id, state) {
            Ok(response) => {
                print_json(&keywords::with_outcome(
                    response,
                    keywords::Update::Updated.keyword(),
                ));
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(exit_codes::for_surface(&error));
            }
        }
    }
    let title = args.title.unwrap_or_default();
    let body = args.description.unwrap_or_default();
    match client.update_op(&id_of(&args.id), &title, &body) {
        Ok(()) => {
            keywords::text_line(keywords::Update::Updated.keyword(), &args.id);
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("linear update failed");
            ExitCode::from(exit_codes::for_failure(&failure))
        }
    }
}

fn run_create(args: &CreateArgs) -> ExitCode {
    let (title, body) = match resolve_create_inputs(args) {
        Ok(inputs) => inputs,
        Err(code) => return code,
    };
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.create_op(&title, &body, "story") {
        Ok(identifier) => {
            keywords::text_line(
                keywords::Create::Created.keyword(),
                identifier.as_str(),
            );
            ExitCode::SUCCESS
        }
        Err(linear_client::LinearFailure::UnwritableIdentifier {
            identifier,
            ..
        }) => {
            keywords::text_line(
                keywords::Create::WritebackFailed.keyword(),
                &identifier,
            );
            eprintln!(
                "E_CREATE_WRITEBACK_FAILED: issue created remotely as \
                 {identifier}; reconcile manually"
            );
            ExitCode::from(exit_codes::CREATE_WRITEBACK_FAILED)
        }
        Err(failure) => {
            eprintln!("linear create failed");
            ExitCode::from(exit_codes::for_failure(&failure))
        }
    }
}

fn resolve_create_inputs(
    args: &CreateArgs,
) -> Result<(String, String), ExitCode> {
    if let Some(title) = &args.title {
        let body = match &args.body_file {
            Some(path) => std::fs::read_to_string(path).map_err(|error| {
                eprintln!(
                    "E_CREATE_NO_FILE: could not read --body-file: {error}"
                );
                ExitCode::from(exit_codes::CREATE_NO_FILE)
            })?,
            None => String::new(),
        };
        return Ok((title.clone(), body));
    }
    let Some(file) = &args.file else {
        eprintln!("E_CREATE_NO_TITLE: pass a work-item file or --title");
        return Err(ExitCode::from(exit_codes::CREATE_NO_TITLE));
    };
    let parsed = work_item::read(file).map_err(|error| {
        eprintln!("E_CREATE_BAD_FRONTMATTER: {error}");
        ExitCode::from(exit_codes::CREATE_BAD_FRONTMATTER)
    })?;
    if parsed.already_synced {
        eprintln!(
            "E_CREATE_ALREADY_SYNCED: the work item already has an external_id"
        );
        return Err(ExitCode::from(exit_codes::CREATE_ALREADY_SYNCED));
    }
    Ok((parsed.title, parsed.body))
}

fn run_init(action: InitAction) -> ExitCode {
    let built = match client_or_report() {
        Ok(built) => built,
        Err(code) => return code,
    };
    let state_dir = built.integrations_root.join("linear");
    // The catalogue write takes an mkdir lock whose parent must already exist,
    // so scaffold the state directory before either cache write.
    if let Err(error) = std::fs::create_dir_all(&state_dir) {
        eprintln!("E_CACHE_IO: {}: {error}", state_dir.display());
        return ExitCode::from(exit_codes::ERROR);
    }
    let filesystem = SystemFilesystem::new(built.project_root.clone());
    let cache = LinearCache::new(&filesystem, state_dir);
    match action {
        InitAction::Verify => match built.client.discover_viewer() {
            Ok(viewer) => {
                if let Err(error) = cache.write_viewer(&viewer) {
                    eprintln!("{error}");
                    return ExitCode::from(exit_codes::for_cache(&error));
                }
                print_json(&keywords::with_outcome(
                    viewer,
                    keywords::Init::Verified.keyword(),
                ));
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(exit_codes::INIT_VERIFY_FAILED)
            }
        },
        InitAction::ListTeams => match built.client.list_teams() {
            Ok(teams) => {
                let envelope = json!({"teams": teams});
                print_json(&keywords::with_outcome(
                    envelope,
                    keywords::Init::Listed.keyword(),
                ));
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(exit_codes::for_surface(&error))
            }
        },
        InitAction::Discover { team_id } => {
            match built.client.discover_team(&team_id) {
                Ok(catalogue) => {
                    if let Err(error) = cache.write_catalogue(&catalogue) {
                        eprintln!("{error}");
                        return ExitCode::from(exit_codes::for_cache(&error));
                    }
                    print_json(&keywords::with_outcome(
                        catalogue,
                        keywords::Init::Discovered.keyword(),
                    ));
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::from(exit_codes::for_surface(&error))
                }
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Show(args) => run_show(&args),
        Command::Search(args) => run_search(args),
        Command::Comment { action } => run_comment(action),
        Command::Transition(args) => run_transition(args),
        Command::Attach(args) => run_attach(&args),
        Command::Update(args) => run_update(args),
        Command::Create(args) => run_create(&args),
        Command::Init { action } => run_init(action),
    }
}
