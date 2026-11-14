//! `accelerator-shortcut`: the Shortcut init, show and search sub-binary,
//! dispatched by the `accelerator` launcher.

mod cli;
mod context;
mod exit_codes;
mod keywords;
mod team_key_writeback;

use std::process::ExitCode;

use clap::Parser as _;
use config::consent;
use config::consent::Refusal;
use serde_json::json;
use serde_json::Value;
use shortcut_client::cache::ShortcutCache;
use shortcut_client::cache::SystemFilesystem;
use shortcut_client::catalogue::Catalogue;
use shortcut_client::client::ReadFailure;
use shortcut_client::discovery::discover;
use shortcut_client::discovery::DiscoveryError;
use shortcut_client::paging::collect;
use shortcut_client::paging::PageError;
use shortcut_client::paths::Paths;
use shortcut_client::reference::ReferenceRefusal;
use shortcut_client::reference::ShortcutReference;
use shortcut_client::team_key::TeamIdentity;
use shortcut_client::team_key::TeamKeys;
use shortcut_client::workspace::whoami;
use shortcut_client::ShortcutClient;
use shortcut_client::ShortcutKey;
use tracker::Completeness;
use tracker::EntityScope;
use tracker::SearchScope;

use crate::cli::Cli;
use crate::cli::Command;
use crate::cli::InitAction;
use crate::cli::SearchArgs;
use crate::cli::ShowArgs;
use crate::context::ContextError;
use crate::context::Intent;
use crate::context::Session;
use crate::team_key_writeback::Writeback;

// A release build that turns debug assertions back on slips past this guard,
// so `main` reads the marker below: stripping and LTO then keep its bytes in
// the binary, where a scan can find them.
#[cfg(all(feature = "test-loopback", not(debug_assertions)))]
compile_error!("test-loopback must never be enabled in a release build");

#[cfg(feature = "test-loopback")]
const TEST_LOOPBACK_MARKER: &str = "ACCELERATOR_SHORTCUT_TEST_LOOPBACK_MARKER";

fn reported(error: ContextError) -> ExitCode {
    match error {
        ContextError::BadApiUrl(raw) => {
            eprintln!(
                "E_BAD_API_URL: ACCELERATOR_SHORTCUT_API_URL={raw:?} is not an \
                 admissible https://api.app.shortcut.com origin"
            );
            ExitCode::from(exit_codes::USAGE)
        }
        ContextError::Config(message) => {
            eprintln!("{message}");
            ExitCode::from(exit_codes::ERROR)
        }
        ContextError::Client(error) => {
            report_warnings(error.warnings());
            eprintln!("{error}");
            ExitCode::from(exit_codes::for_client(&error))
        }
    }
}

fn report_warnings(warnings: &[Refusal]) {
    for warning in consent::reportable(warnings) {
        eprintln!("warning: {warning}");
    }
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| value.to_string())
    );
}

fn session_or_report(intent: Intent) -> Result<Session, ExitCode> {
    let session = context::open_session(intent).map_err(reported)?;
    if let Some(notice) = &session.notice {
        eprintln!("{notice}");
    }
    report_warnings(&session.refusals);
    Ok(session)
}

fn run_init(action: InitAction) -> ExitCode {
    let intent = match action {
        InitAction::Discover { .. } => Intent::Write,
        InitAction::Verify(_) | InitAction::ListTeams(_) => Intent::Read,
    };
    let session = match session_or_report(intent) {
        Ok(session) => session,
        Err(code) => return code,
    };
    match action {
        InitAction::Verify(quiet) => verify(&session, quiet.quiet),
        InitAction::ListTeams(_) => list_teams(&session),
        InitAction::Discover { team, force, quiet } => {
            discover_workspace(&session, &team, force, quiet.quiet)
        }
    }
}

fn verify(session: &Session, quiet: bool) -> ExitCode {
    let me = match whoami(
        &session.transport,
        &session.base,
        &session.transport.deadline(),
    ) {
        Ok(me) => me,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::for_discovery(&error));
        }
    };
    if me.is_read_only() && !quiet {
        eprintln!(
            "note: this token has the read scope only. Show and search work; \
             the create, update, comment, transition and attach skills will \
             be refused"
        );
    }
    print_json(&keywords::with_outcome(
        json!({
            "workspace": {"slug": me.slug.as_str(), "name": me.workspace_name},
            "member": {"id": me.member_id, "mention_name": me.member_mention_name},
            "scopes": me.scopes,
            "read_only": me.is_read_only(),
        }),
        keywords::Init::Verified.keyword(),
    ));
    ExitCode::SUCCESS
}

fn discovery_failed(error: &DiscoveryError) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(exit_codes::for_discovery(error))
}

fn list_teams(session: &Session) -> ExitCode {
    let deadline = session.transport.deadline();
    let me = match whoami(&session.transport, &session.base, &deadline) {
        Ok(me) => me,
        Err(error) => return discovery_failed(&error),
    };
    let paths = Paths::new(session.base.clone(), me.slug);
    let pages = match collect(
        &session.transport,
        &paths,
        &paths.teams(),
        None,
        &deadline,
    ) {
        Ok(pages) => pages,
        Err(error) => return discovery_failed(&page_failure(error)),
    };
    let identities: Vec<TeamIdentity> = pages
        .entities
        .iter()
        .filter_map(|team| {
            Some(TeamIdentity {
                uuid: team.get("id")?.as_str()?.to_owned(),
                mention_name: team
                    .get("mention_name")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                name: team.get("name")?.as_str()?.to_owned(),
            })
        })
        .collect();
    let keys = TeamKeys::over(&identities);
    let teams: Vec<Value> = identities
        .iter()
        .map(|team| {
            json!({
                "key": keys.visible_key(&team.uuid),
                "name": team.name,
                "id": team.uuid,
            })
        })
        .collect();
    if let Some(cut) = pages.cut {
        eprintln!(
            "warning: the team listing stopped before its last page ({}), so \
             some teams may be missing; retry",
            cut.cause()
        );
    }
    print_json(&keywords::with_outcome(
        json!({"teams": teams, "complete": pages.cut.is_none()}),
        keywords::Init::Listed.keyword(),
    ));
    ExitCode::SUCCESS
}

fn page_failure(error: PageError) -> DiscoveryError {
    match error {
        PageError::Refused(received) => DiscoveryError::refused(&received, &[]),
        PageError::Unusable(what) => DiscoveryError::Unusable { what },
        PageError::Client(error) => DiscoveryError::Client(error),
        PageError::Repeated(url) => DiscoveryError::RepeatedPage { url },
    }
}

fn discover_workspace(
    session: &Session,
    team_key: &str,
    force: bool,
    quiet: bool,
) -> ExitCode {
    let (_, discovered) = match discover(
        &session.transport,
        &session.base,
        &session.transport.deadline(),
    ) {
        Ok(found) => found,
        Err(error) => return discovery_failed(&error),
    };
    let base = match discovered.catalogue.team_for_key(team_key) {
        Ok(team) => json!({"key": team_key, "name": team.name, "id": team.id}),
        Err(error) => {
            eprintln!(
                "E_INIT_NO_TEAM: {error}; the team keys are {}",
                discovered.catalogue.team_keys().known().join(", ")
            );
            return ExitCode::from(exit_codes::INIT_NO_TEAM);
        }
    };
    let summary = discovered_summary(&discovered.catalogue, &base);
    let filesystem = SystemFilesystem::new(session.project_root.clone());
    let cache = ShortcutCache::new(
        &filesystem,
        session.integrations_root.join("shortcut"),
    );
    if let Err(error) = cache.write(discovered.catalogue, discovered.listing) {
        eprintln!("{error}");
        return ExitCode::from(exit_codes::for_cache(&error));
    }
    report_team_key(&session.config.service, team_key, force, quiet);
    print_json(&keywords::with_outcome(
        summary,
        keywords::Init::Discovered.keyword(),
    ));
    ExitCode::SUCCESS
}

fn discovered_summary(catalogue: &Catalogue, base: &Value) -> Value {
    json!({
        "workspace": {
            "slug": catalogue.slug().as_str(),
            "name": catalogue.workspace().name,
        },
        "team": base,
        "teams": catalogue.teams().len(),
        "workflows": catalogue.workflows().len(),
    })
}

fn report_team_key(
    config: &dyn config::ConfigAccess,
    discovered: &str,
    force: bool,
    quiet: bool,
) {
    match team_key_writeback::write_team_key(config, discovered, force) {
        Ok(Writeback::Written | Writeback::Overwritten) if !quiet => {
            eprintln!("shortcut.team_key set to {discovered:?}");
        }
        Ok(Writeback::LeftIntact { existing }) => {
            eprintln!(
                "shortcut.team_key left intact as {existing:?}; re-run with \
                 --force to set it to {discovered:?}"
            );
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("warning: could not write shortcut.team_key: {error}");
        }
    }
}

fn client_or_report() -> Result<ShortcutClient, ExitCode> {
    let client = context::build_client(Intent::Read).map_err(reported)?;
    if let Some(notice) = client.notice() {
        eprintln!("{notice}");
    }
    report_warnings(client.refusals());
    Ok(client)
}

fn refused_reference(refusal: &ReferenceRefusal) -> ExitCode {
    eprintln!("E_SHOW_BAD_REFERENCE: {refusal}");
    ExitCode::from(match refusal {
        ReferenceRefusal::Unrecognised(_) => exit_codes::SHOW_BAD_REFERENCE,
        ReferenceRefusal::ForeignWorkspace { .. } => {
            exit_codes::SHOW_FOREIGN_WORKSPACE
        }
    })
}

fn run_show(args: &ShowArgs) -> ExitCode {
    let reference = match ShortcutReference::parse(&args.reference) {
        Ok(reference) => reference,
        Err(refusal) => return refused_reference(&refusal),
    };
    let client = match client_or_report() {
        Ok(client) => client,
        Err(code) => return code,
    };
    let key = match reference.key_in(client.catalogue().slug()) {
        Ok(key) => key,
        Err(refusal) => return refused_reference(&refusal),
    };
    match client.read(&client.paths().entity(key)) {
        Ok(body) => show_found(&client, key, body, args.comments),
        Err(ReadFailure::Status { status: 404, .. }) => {
            print_json(&keywords::with_outcome(
                json!({"key": key.to_string()}),
                keywords::Show::NotFound.keyword(),
            ));
            ExitCode::from(exit_codes::SHOW_NOT_FOUND)
        }
        Err(failure) => {
            eprintln!("{failure}");
            ExitCode::from(exit_codes::for_read(&failure))
        }
    }
}

fn show_found(
    client: &ShortcutClient,
    key: ShortcutKey,
    body: Value,
    comments: Option<usize>,
) -> ExitCode {
    let mut entity = body.get("entity").cloned().unwrap_or(body);
    if let Value::Object(fields) = &mut entity {
        fields.remove("comments");
    }
    let mut envelope = json!({"key": key.to_string(), "entity": entity});
    if let Some(count) = comments {
        let paths = client.paths();
        let pages = match collect(
            client.transport(),
            paths,
            &paths.comments(key),
            None,
            &client.transport().deadline(),
        ) {
            Ok(pages) => pages,
            Err(error) => return discovery_failed(&page_failure(error)),
        };
        let skipped = pages.entities.len().saturating_sub(count);
        envelope["comments"] = Value::from(
            pages.entities.into_iter().skip(skipped).collect::<Vec<_>>(),
        );
    }
    print_json(&keywords::with_outcome(
        envelope,
        keywords::Show::Found.keyword(),
    ));
    ExitCode::SUCCESS
}

fn run_search(args: &SearchArgs) -> ExitCode {
    let client = match client_or_report() {
        Ok(client) => client,
        Err(code) => return code,
    };
    let team = match client.base_team() {
        Ok(team) => team.id.clone(),
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::NO_TEAM_KEY);
        }
    };
    let scope = SearchScope {
        entities: EntityScope::Keyed {
            base: Some(team),
            additional: Vec::new(),
        },
        filters: args.filter_pairs(),
    };
    let (stories, completeness) = match client.search_stories(&scope) {
        Ok(found) => found,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::for_search(&error));
        }
    };
    let limited = args.limit.is_some_and(|limit| stories.len() > limit);
    let stories: Vec<Value> = stories
        .iter()
        .take(args.limit.unwrap_or(usize::MAX))
        .map(story_summary)
        .collect();
    let truncated = limited || !completeness.is_complete();
    let keyword = if completeness == Completeness::CapHit {
        keywords::Search::CapHit
    } else if truncated {
        keywords::Search::Truncated
    } else if stories.is_empty() {
        keywords::Search::Empty
    } else {
        keywords::Search::Results
    };
    print_json(&keywords::with_outcome(
        json!({"stories": stories, "truncated": truncated}),
        keyword.keyword(),
    ));
    match completeness {
        Completeness::CapHit => {
            eprintln!(
                "E_SEARCH_CAP_HIT: the page cap was reached before the last \
                 result, so the results are a lower bound. Raise \
                 shortcut.pull.max_pages, or set it to `unlimited`, then re-run"
            );
            ExitCode::from(exit_codes::SEARCH_CAP_HIT)
        }
        Completeness::Transient if !args.quiet.quiet => {
            eprintln!(
                "warning: the search was cut short by its deadline, so the \
                 results are a lower bound; retry"
            );
            ExitCode::SUCCESS
        }
        Completeness::Transient | Completeness::Complete => ExitCode::SUCCESS,
    }
}

fn names(list: Option<&Value>) -> Vec<Value> {
    list.and_then(|list| list.get("entities").or(Some(list)))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.get("name").cloned())
                .collect()
        })
        .unwrap_or_default()
}

fn story_summary(story: &Value) -> Value {
    let field = |name: &str| story.get(name).cloned().unwrap_or(Value::Null);
    json!({
        "key": story
            .get("id")
            .and_then(Value::as_u64)
            .map(|number| format!("sc-{number}")),
        "name": field("name"),
        "story_type": field("story_type"),
        "state": story.pointer("/workflow_state/name").cloned(),
        "owners": names(story.get("owners")),
        "labels": names(story.get("labels")),
        "estimate": field("estimate"),
        "app_url": field("app_url"),
        "updated_at": field("updated_at"),
    })
}

fn main() -> ExitCode {
    #[cfg(feature = "test-loopback")]
    std::hint::black_box(TEST_LOOPBACK_MARKER);
    let cli = Cli::parse();
    match cli.command {
        Command::Init { action } => run_init(action),
        Command::Show(args) => run_show(&args),
        Command::Search(args) => run_search(&args),
    }
}
