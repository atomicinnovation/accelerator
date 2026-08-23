//! `accelerator-jira` — the jira create|update|show|search|comment|transition|
//! attach|init|fields|resolve-fields sub-binary, dispatched by the
//! `accelerator` launcher.

mod cli;
mod context;
mod exit_codes;
mod frontmatter;
mod keywords;
mod render;
mod resolve_fields;

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser as _;
use jira_client::cache::JiraCache;
use jira_client::cache::SystemFilesystem;
use jira_client::comment::Visibility;
use jira_client::transition::Target;
use serde_json::json;
use serde_json::Value;
use tracker::ExternalId;

use crate::cli::{
    AttachArgs, Cli, Command, CommentAction, CommentAddArgs, CommentDeleteArgs,
    CommentEditArgs, CommentListArgs, CreateArgs, CreateEmit, FieldsAction,
    InitAction, SearchArgs, ShowArgs, TransitionArgs, UpdateArgs,
};
use crate::context::ContextError;
use crate::resolve_fields::ResolveError;

// The test-only loopback feature must never reach a release binary: the compile
// guard rests on `[profile.release]` keeping debug-assertions off, and a
// build-system byte scan of the staged binary greps for the marker below.
#[cfg(all(feature = "test-loopback", not(debug_assertions)))]
compile_error!("test-loopback must never be enabled in a release build");

#[cfg(feature = "test-loopback")]
#[no_mangle]
pub static ACCELERATOR_JIRA_TEST_LOOPBACK_MARKER: u8 = 0;

fn id_of(value: &str) -> ExternalId {
    ExternalId::new(value.to_owned())
}

/// Builds the client, or prints the failure and returns the mapped exit code.
fn client_or_report() -> Result<context::Built, ExitCode> {
    context::build_client().map_err(|error| match error {
        ContextError::BadApiUrl(raw) => {
            eprintln!(
                "E_BAD_API_URL: ACCELERATOR_JIRA_API_URL={raw:?} is not an \
                 admissible https *.atlassian.net destination"
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

fn read_body(
    body: Option<&str>,
    body_file: Option<&Path>,
    missing: u8,
) -> Result<String, ExitCode> {
    match (body, body_file) {
        (Some(body), _) => Ok(body.to_owned()),
        (None, Some(path)) => std::fs::read_to_string(path).map_err(|error| {
            eprintln!("E_COMMENT_NO_BODY: could not read --body-file: {error}");
            ExitCode::from(missing)
        }),
        (None, None) => {
            eprintln!("E_COMMENT_NO_BODY: a --body or --body-file is required");
            Err(ExitCode::from(missing))
        }
    }
}

fn parse_visibility(raw: &str) -> Result<Visibility, ExitCode> {
    match raw.split_once(':') {
        Some(("role", name)) => Ok(Visibility::Role(name.to_owned())),
        Some(("group", name)) => Ok(Visibility::Group(name.to_owned())),
        _ => {
            eprintln!(
                "E_COMMENT_BAD_VISIBILITY: --visibility must be role:NAME or \
                 group:NAME"
            );
            Err(ExitCode::from(exit_codes::COMMENT_BAD_VISIBILITY))
        }
    }
}

fn run_create(args: &CreateArgs) -> ExitCode {
    let Some(summary) = args.summary.as_deref() else {
        eprintln!("E_CREATE_NO_SUMMARY: a --summary is required");
        return ExitCode::from(exit_codes::CREATE_NO_SUMMARY);
    };
    let body = match args.body_file.as_deref() {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(body) => body,
            Err(error) => {
                eprintln!(
                    "E_CREATE_NO_BODY: could not read --body-file: {error}"
                );
                return ExitCode::from(exit_codes::CREATE_NO_BODY);
            }
        },
        None => String::new(),
    };
    let kind = args.r#type.clone().unwrap_or_default();
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.create_op(summary, &body, &kind) {
        Ok(key) => emit_created(key.as_str(), args.emit),
        Err(jira_client::JiraFailure::UnwritableIdentifier {
            identifier,
            ..
        }) => {
            eprintln!(
                "E_REQ_BAD_RESPONSE: an issue may have been created as \
                 {identifier}; do NOT blindly retry — reconcile manually"
            );
            ExitCode::from(exit_codes::REQ_BAD_RESPONSE)
        }
        Err(failure) => {
            eprintln!("jira create failed");
            ExitCode::from(exit_codes::for_failure(&failure))
        }
    }
}

fn emit_created(key: &str, emit: CreateEmit) -> ExitCode {
    match emit {
        CreateEmit::Key => println!("{key}"),
        CreateEmit::Json => {
            let envelope = json!({"key": key});
            print_json(&keywords::with_outcome(
                envelope,
                keywords::Create::Created.keyword(),
            ));
        }
    }
    ExitCode::SUCCESS
}

fn run_update(args: &UpdateArgs) -> ExitCode {
    if args.summary.is_none() && args.body_file.is_none() {
        eprintln!("E_UPDATE_NO_OPS: at least one field to update is required");
        return ExitCode::from(exit_codes::UPDATE_NO_OPS);
    }
    let body = match args.body_file.as_deref() {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(body) => body,
            Err(error) => {
                eprintln!(
                    "E_UPDATE_NO_BODY: could not read --body-file: {error}"
                );
                return ExitCode::from(exit_codes::UPDATE_NO_BODY);
            }
        },
        None => String::new(),
    };
    let summary = args.summary.clone().unwrap_or_default();
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.update_op(&id_of(&args.key), &summary, &body) {
        Ok(()) => {
            keywords::text_line(keywords::Update::Updated.keyword(), &args.key);
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("jira update failed");
            ExitCode::from(exit_codes::for_failure(&failure))
        }
    }
}

fn run_show(args: &ShowArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    let expand = if args.comments.is_some() {
        format!("{},comments", args.expand)
    } else {
        args.expand.clone()
    };
    match client.show_detailed(&args.key, &args.fields, &expand) {
        Ok(mut issue) => {
            if let Some(limit) = args.comments {
                slice_comments(&mut issue, limit as usize);
            }
            if !args.no_render_adf {
                render::render_issue(&mut issue);
            }
            let found = issue.pointer("/key").is_some();
            let keyword = if found {
                keywords::Show::Found
            } else {
                keywords::Show::NotFound
            };
            print_json(&keywords::with_outcome(issue, keyword.keyword()));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        }
    }
}

fn slice_comments(issue: &mut Value, limit: usize) {
    if let Some(comments) = issue
        .pointer_mut("/fields/comment/comments")
        .and_then(Value::as_array_mut)
    {
        if comments.len() > limit {
            let start = comments.len() - limit;
            comments.drain(0..start);
        }
    }
}

fn run_search(args: &SearchArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    let mut search = search_from(args);
    // Default the project from config when neither --project nor --all-projects
    // is given, reproducing the bash search flow.
    if search.project.is_none() && !search.all_projects {
        search.project = resolve_fields::configured_default_project();
    }
    if !args.quiet {
        if let Ok(jql) = client.compose_search_jql(&search) {
            eprintln!("INFO: composed JQL: {jql}");
        }
    }
    match client.search_detailed(
        &search,
        &args.fields,
        args.limit,
        args.page_token.as_deref(),
    ) {
        Ok(mut envelope) => {
            if args.render_adf {
                render::render_search(&mut envelope);
            }
            let empty = envelope
                .pointer("/issues")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty);
            let keyword = if empty {
                keywords::Search::Empty
            } else {
                keywords::Search::Results
            };
            print_json(&keywords::with_outcome(envelope, keyword.keyword()));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_surface(&error))
        }
    }
}

fn search_from(args: &SearchArgs) -> jira_client::jql::Search {
    let family = |field: &str, values: &[String]| {
        (!values.is_empty()).then(|| jira_client::jql::Family {
            field: field.to_owned(),
            values: values.to_vec(),
        })
    };
    let families = [
        family("status", &args.statuses),
        family("labels", &args.labels),
        family("assignee", &args.assignees),
        family("issuetype", &args.types),
        family("component", &args.components),
        family("reporter", &args.reporters),
        family("parent", &args.parents),
    ]
    .into_iter()
    .flatten()
    .collect();
    jira_client::jql::Search {
        project: args.project.clone(),
        all_projects: args.all_projects,
        families,
        watching: args.watching,
        text: args.texts.clone(),
        raw: args.jql.clone(),
        ..jira_client::jql::Search::default()
    }
}

fn run_comment(action: CommentAction) -> ExitCode {
    match action {
        CommentAction::Add(args) => run_comment_add(&args),
        CommentAction::List(args) => run_comment_list(&args),
        CommentAction::Edit(args) => run_comment_edit(&args),
        CommentAction::Delete(args) => run_comment_delete(&args),
    }
}

fn run_comment_add(args: &CommentAddArgs) -> ExitCode {
    let body = match read_body(
        args.body.as_deref(),
        args.body_file.as_deref(),
        exit_codes::COMMENT_NO_BODY,
    ) {
        Ok(body) => body,
        Err(code) => return code,
    };
    let visibility = match args.visibility.as_deref().map(parse_visibility) {
        Some(Ok(visibility)) => Some(visibility),
        Some(Err(code)) => return code,
        None => None,
    };
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.add_comment(
        &args.key,
        &body,
        visibility.as_ref(),
        !args.no_notify,
    ) {
        Ok(response) => {
            print_json(&keywords::with_outcome(
                response,
                keywords::Comment::Added.keyword(),
            ));
            ExitCode::SUCCESS
        }
        Err(error) => surface_failure(&error),
    }
}

fn run_comment_list(args: &CommentListArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.list_comments(&args.key, args.page_size, args.first_page_only)
    {
        Ok(page) => {
            let envelope = json!({
                "startAt": 0,
                "maxResults": page.comments.len(),
                "total": page.total,
                "truncated": page.truncated,
                "comments": page.comments,
            });
            print_json(&keywords::with_outcome(
                envelope,
                keywords::Comment::Listed.keyword(),
            ));
            ExitCode::SUCCESS
        }
        Err(error) => surface_failure(&error),
    }
}

fn run_comment_edit(args: &CommentEditArgs) -> ExitCode {
    let body = match read_body(
        args.body.as_deref(),
        args.body_file.as_deref(),
        exit_codes::COMMENT_NO_BODY,
    ) {
        Ok(body) => body,
        Err(code) => return code,
    };
    let visibility = match args.visibility.as_deref().map(parse_visibility) {
        Some(Ok(visibility)) => Some(visibility),
        Some(Err(code)) => return code,
        None => None,
    };
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.edit_comment(
        &args.key,
        &args.comment_id,
        &body,
        visibility.as_ref(),
        !args.no_notify,
    ) {
        Ok(response) => {
            print_json(&keywords::with_outcome(
                response,
                keywords::Comment::Edited.keyword(),
            ));
            ExitCode::SUCCESS
        }
        Err(error) => surface_failure(&error),
    }
}

fn run_comment_delete(args: &CommentDeleteArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.delete_comment(&args.key, &args.comment_id, !args.no_notify) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => surface_failure(&error),
    }
}

fn run_transition(args: &TransitionArgs) -> ExitCode {
    let state = args.state.as_deref().or(args.state_flag.as_deref());
    let target = match (args.transition_id.as_deref(), state) {
        (Some(id), _) => Target::Id(id.to_owned()),
        (None, Some(state)) => Target::State(state.to_owned()),
        (None, None) => {
            eprintln!("E_TRANSITION_NO_STATE: a target state is required");
            return ExitCode::from(exit_codes::TRANSITION_NO_STATE);
        }
    };
    let client = match client_or_report() {
        Ok(built) => built.client,
        Err(code) => return code,
    };
    match client.transition(
        &args.key,
        &target,
        args.resolution.as_deref(),
        args.comment.as_deref(),
        !args.no_notify,
    ) {
        Ok(()) => {
            keywords::text_line(
                keywords::Transition::Transitioned.keyword(),
                &args.key,
            );
            ExitCode::SUCCESS
        }
        Err(error) => surface_failure(&error),
    }
}

fn run_attach(args: &AttachArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(built) => built,
        Err(code) => return code,
    };
    let files: Vec<&Path> = args.files.iter().map(PathBuf::as_path).collect();
    match client
        .client
        .attach(&args.key, &files, &client.project_root)
    {
        // The attachments response is a bare JSON array, which cannot carry a
        // top-level `outcome` field; attach is exit-code driven (a write flow),
        // so the array is emitted verbatim as the bash flow did.
        Ok(response) => {
            print_json(&response);
            ExitCode::SUCCESS
        }
        Err(error) => surface_failure(&error),
    }
}

fn surface_failure(error: &jira_client::SurfaceError) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(exit_codes::for_surface(error))
}

fn run_init(action: &InitAction) -> ExitCode {
    let built = match client_or_report() {
        Ok(built) => built,
        Err(code) => return code,
    };
    let state_dir = built.integrations_root.join("jira");
    if let Err(error) = std::fs::create_dir_all(&state_dir) {
        eprintln!("E_CACHE_IO: {}: {error}", state_dir.display());
        return ExitCode::from(exit_codes::ERROR);
    }
    let filesystem = SystemFilesystem::new(built.project_root.clone());
    let cache = JiraCache::new(&filesystem, state_dir.clone());
    match action {
        InitAction::Verify => init_verify(&built, &cache),
        InitAction::Discover | InitAction::RefreshFields => {
            init_discover(&built, &cache)
        }
        InitAction::PromptDefault => init_prompt_default(),
        InitAction::ListProjects => {
            list_cached(&state_dir, "projects.json", "projects")
        }
        InitAction::ListFields => {
            list_cached(&state_dir, "fields.json", "fields")
        }
    }
}

fn init_verify(built: &context::Built, cache: &JiraCache<'_>) -> ExitCode {
    match built.client.discover_site() {
        Ok(site) => {
            if let Err(error) = cache.write_site(&site) {
                eprintln!("{error}");
                return ExitCode::from(exit_codes::for_cache(&error));
            }
            print_json(&keywords::with_outcome(
                site,
                keywords::Init::Verified.keyword(),
            ));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(exit_codes::INIT_VERIFY_FAILED)
        }
    }
}

fn init_discover(built: &context::Built, cache: &JiraCache<'_>) -> ExitCode {
    let projects = match built.client.discover_projects() {
        Ok(projects) => projects,
        Err(error) => return surface_failure(&error),
    };
    let fields = match built.client.discover_fields() {
        Ok(fields) => fields,
        Err(error) => return surface_failure(&error),
    };
    if let Err(error) = cache.write_discovery(&projects, &fields) {
        eprintln!("{error}");
        return ExitCode::from(exit_codes::for_cache(&error));
    }
    let envelope = json!({"projects": projects, "fields": fields});
    print_json(&keywords::with_outcome(
        envelope,
        keywords::Init::Discovered.keyword(),
    ));
    ExitCode::SUCCESS
}

fn init_prompt_default() -> ExitCode {
    let resolved = resolve_fields::resolve(&cli::ResolveFieldsArgs {
        file: None,
        kind: Some("task".to_owned()),
        project: None,
        id: None,
    });
    if let Ok(resolution) = resolved {
        keywords::text_line(
            keywords::Init::DefaultPrompted.keyword(),
            &resolution.project,
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "E_INIT_NEEDS_CONFIG: no default project — set \
             work.default_project_code"
        );
        ExitCode::from(exit_codes::INIT_NEEDS_CONFIG)
    }
}

fn list_cached(state_dir: &Path, file: &str, field: &str) -> ExitCode {
    let path = state_dir.join(file);
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!(
            "E_FIELD_CACHE_MISSING: {}; run init discover",
            path.display()
        );
        return ExitCode::from(exit_codes::FIELD_CACHE_MISSING);
    };
    let Ok(cached) = serde_json::from_str::<Value>(&text) else {
        eprintln!(
            "E_FIELD_CACHE_CORRUPT: {} is not valid JSON",
            path.display()
        );
        return ExitCode::from(exit_codes::FIELD_CACHE_CORRUPT);
    };
    let list = cached.get(field).cloned().unwrap_or(Value::Array(vec![]));
    print_json(&list);
    ExitCode::SUCCESS
}

fn run_fields(action: FieldsAction) -> ExitCode {
    let built = match client_or_report() {
        Ok(built) => built,
        Err(code) => return code,
    };
    let state_dir = built.integrations_root.join("jira");
    match action {
        FieldsAction::Refresh => {
            if let Err(error) = std::fs::create_dir_all(&state_dir) {
                eprintln!("E_CACHE_IO: {}: {error}", state_dir.display());
                return ExitCode::from(exit_codes::ERROR);
            }
            let filesystem = SystemFilesystem::new(built.project_root.clone());
            let cache = JiraCache::new(&filesystem, state_dir);
            init_discover(&built, &cache)
        }
        FieldsAction::List => list_cached(&state_dir, "fields.json", "fields"),
        FieldsAction::Resolve { query } => resolve_field(&state_dir, &query),
    }
}

fn resolve_field(state_dir: &Path, query: &str) -> ExitCode {
    let path = state_dir.join("fields.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!(
            "E_FIELD_CACHE_MISSING: {}; run init discover",
            path.display()
        );
        return ExitCode::from(exit_codes::FIELD_CACHE_MISSING);
    };
    let Ok(cached) = serde_json::from_str::<Value>(&text) else {
        eprintln!(
            "E_FIELD_CACHE_CORRUPT: {} is not valid JSON",
            path.display()
        );
        return ExitCode::from(exit_codes::FIELD_CACHE_CORRUPT);
    };
    let fields = cached.get("fields").and_then(Value::as_array);
    let resolved = fields.and_then(|fields| {
        fields.iter().find_map(|field| match_field(field, query))
    });
    resolved.map_or_else(
        || {
            eprintln!("E_FIELD_NOT_FOUND: no field matches {query:?}");
            ExitCode::from(exit_codes::FIELD_NOT_FOUND)
        },
        |id| {
            println!("{id}");
            ExitCode::SUCCESS
        },
    )
}

/// A field matches by name, slug, id or key; the id it resolves to is returned.
fn match_field(field: &Value, query: &str) -> Option<String> {
    let matches = ["name", "slug", "id", "key"].iter().any(|attribute| {
        field.get(attribute).and_then(Value::as_str) == Some(query)
    });
    matches
        .then(|| field.get("id").and_then(Value::as_str))
        .flatten()
        .map(str::to_owned)
}

fn run_resolve_fields(args: &cli::ResolveFieldsArgs) -> ExitCode {
    match resolve_fields::resolve(args) {
        Ok(resolution) => {
            print!("{}", resolution.line());
            ExitCode::SUCCESS
        }
        Err(ResolveError::AlreadySynced { file, external_id }) => {
            eprintln!(
                "E_RESOLVE_ALREADY_SYNCED: {file} already carries external_id \
                 {external_id}; nothing to create"
            );
            ExitCode::from(exit_codes::RESOLVE_ALREADY_SYNCED)
        }
        Err(ResolveError::NoProject) => {
            eprintln!(
                "E_RESOLVE_NO_PROJECT: cannot resolve a Jira project — pass \
                 --project, set work.default_project_code, or use a \
                 project-coded id"
            );
            ExitCode::from(exit_codes::RESOLVE_NO_PROJECT)
        }
        Err(ResolveError::Usage(message)) => {
            eprintln!("jira resolve-fields: {message}");
            ExitCode::from(exit_codes::USAGE)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Create(args) => run_create(&args),
        Command::Update(args) => run_update(&args),
        Command::Show(args) => run_show(&args),
        Command::Search(args) => run_search(&args),
        Command::Comment { action } => run_comment(action),
        Command::Transition(args) => run_transition(&args),
        Command::Attach(args) => run_attach(&args),
        Command::Init { action } => run_init(&action),
        Command::Fields { action } => run_fields(action),
        Command::ResolveFields(args) => run_resolve_fields(&args),
    }
}
