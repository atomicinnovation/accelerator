//! The five write subcommands. Each refuses what it can before it sends
//! anything, and reports what a retry would repeat when a send fails.

use std::path::Path;
use std::process::ExitCode;

use serde_json::json;
use shortcut_client::reference::ShortcutReference;
use shortcut_client::writes::StoryUpdate;
use shortcut_client::writes::WriteError;
use shortcut_client::ShortcutClient;
use shortcut_client::ShortcutKey;
use shortcut_client::StoryId;
use tracker::RemoteTracker as _;

use crate::cli::AttachArgs;
use crate::cli::CommentAction;
use crate::cli::CreateArgs;
use crate::cli::TransitionArgs;
use crate::cli::UpdateArgs;
use crate::context::Intent;
use crate::exit_codes;
use crate::keywords;
use crate::work_item;

fn writer() -> Result<ShortcutClient, ExitCode> {
    crate::client_or_report(Intent::Write)
}

fn key_in(
    client: &ShortcutClient,
    reference: &str,
) -> Result<ShortcutKey, ExitCode> {
    ShortcutReference::parse(reference)
        .and_then(|parsed| parsed.key_in(client.catalogue().slug()))
        .map_err(|refusal| crate::refused_reference(&refusal))
}

fn story_in(
    client: &ShortcutClient,
    reference: &str,
) -> Result<StoryId, ExitCode> {
    key_in(client, reference)?
        .into_story()
        .map_err(|epic| failed(&WriteError::NotAStory(epic.0)))
}

fn failed(error: &WriteError) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(exit_codes::for_write(error))
}

fn report(
    result: Result<serde_json::Value, WriteError>,
    key: ShortcutKey,
    keyword: &str,
) -> ExitCode {
    match result {
        Ok(_) => {
            crate::print_json(&keywords::with_outcome(
                json!({"key": key.to_string()}),
                keyword,
            ));
            ExitCode::SUCCESS
        }
        Err(error) => failed(&error),
    }
}

fn read_text(path: &Path, code: u8, name: &str) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|error| {
        eprintln!("{name}: could not read {}: {error}", path.display());
        ExitCode::from(code)
    })
}

struct NewEntity {
    title: String,
    body: String,
    kind: String,
}

fn create_inputs(args: &CreateArgs) -> Result<NewEntity, ExitCode> {
    if let Some(title) = &args.title {
        let body = args
            .body_file
            .as_deref()
            .map(|path| {
                read_text(
                    path,
                    exit_codes::CREATE_NO_BODY_FILE,
                    "E_CREATE_NO_BODY_FILE",
                )
            })
            .transpose()?
            .unwrap_or_default();
        return Ok(NewEntity {
            title: title.clone(),
            body,
            kind: "story".to_owned(),
        });
    }
    let Some(file) = &args.file else {
        eprintln!("E_CREATE_NO_TITLE: pass a work-item file or --title");
        return Err(ExitCode::from(exit_codes::CREATE_NO_TITLE));
    };
    let item = work_item::read(file).map_err(|error| {
        eprintln!("E_CREATE_BAD_FRONTMATTER: {error}");
        ExitCode::from(exit_codes::CREATE_BAD_FRONTMATTER)
    })?;
    if item.already_synced {
        eprintln!(
            "E_CREATE_ALREADY_SYNCED: the work item already has an external_id"
        );
        return Err(ExitCode::from(exit_codes::CREATE_ALREADY_SYNCED));
    }
    Ok(NewEntity {
        title: item.title,
        body: item.body,
        kind: item.kind,
    })
}

pub fn create(args: &CreateArgs) -> ExitCode {
    let entity = match create_inputs(args) {
        Ok(entity) => entity,
        Err(code) => return code,
    };
    let client = match writer() {
        Ok(client) => client,
        Err(code) => return code,
    };
    let key = match client.create(&entity.title, &entity.body, &entity.kind) {
        Ok(key) => key,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(exit_codes::for_tracker_write(&error));
        }
    };
    if let Some(file) = &args.file {
        if let Err(error) = work_item::link(file, key.as_str()) {
            crate::print_json(&keywords::with_outcome(
                json!({"key": key.as_str()}),
                keywords::Create::WritebackFailed.keyword(),
            ));
            eprintln!(
                "E_CREATE_WRITEBACK_FAILED: created {} in Shortcut, but {error}; \
                 set external_id by hand rather than creating it again",
                key.as_str()
            );
            return ExitCode::from(exit_codes::CREATE_WRITEBACK_FAILED);
        }
    }
    crate::print_json(&keywords::with_outcome(
        json!({"key": key.as_str()}),
        keywords::Create::Created.keyword(),
    ));
    ExitCode::SUCCESS
}

pub fn update(args: &UpdateArgs) -> ExitCode {
    let update = StoryUpdate {
        title: args.title.clone(),
        description: args.description.clone(),
        state: args.state.clone(),
        owner: args.owner.clone(),
        estimate: args.estimate,
    };
    if update == StoryUpdate::default() {
        eprintln!("E_UPDATE_NO_FIELDS: name at least one field to update");
        return ExitCode::from(exit_codes::UPDATE_NO_FIELDS);
    }
    let client = match writer() {
        Ok(client) => client,
        Err(code) => return code,
    };
    let key = match key_in(&client, &args.reference) {
        Ok(key) => key,
        Err(code) => return code,
    };
    report(
        client.update_entity(key, &update),
        key,
        keywords::Update::Updated.keyword(),
    )
}

pub fn comment(action: CommentAction) -> ExitCode {
    let CommentAction::Add(args) = action;
    let body = match (args.body, args.body_file) {
        (Some(body), _) => body,
        (None, Some(path)) => {
            match read_text(
                &path,
                exit_codes::COMMENT_NO_BODY,
                "E_COMMENT_NO_BODY",
            ) {
                Ok(body) => body,
                Err(code) => return code,
            }
        }
        (None, None) => {
            eprintln!("E_COMMENT_NO_BODY: pass --body or --body-file");
            return ExitCode::from(exit_codes::COMMENT_NO_BODY);
        }
    };
    let client = match writer() {
        Ok(client) => client,
        Err(code) => return code,
    };
    let key = match key_in(&client, &args.reference) {
        Ok(key) => key,
        Err(code) => return code,
    };
    report(
        client.add_comment(key, &body),
        key,
        keywords::Comment::Added.keyword(),
    )
}

pub fn transition(args: TransitionArgs) -> ExitCode {
    let Some(state) = args.state.or(args.state_flag) else {
        eprintln!("E_TRANSITION_NO_STATE: name the target state");
        return ExitCode::from(exit_codes::TRANSITION_NO_STATE);
    };
    let client = match writer() {
        Ok(client) => client,
        Err(code) => return code,
    };
    let story = match story_in(&client, &args.reference) {
        Ok(story) => story,
        Err(code) => return code,
    };
    report(
        client.transition(story, &state),
        ShortcutKey::Story(story),
        keywords::Transition::Transitioned.keyword(),
    )
}

pub fn attach(args: &AttachArgs) -> ExitCode {
    if args.file.is_some() {
        eprintln!(
            "E_ATTACH_NO_UPLOAD: attaching a file is not supported yet; \
             upload it elsewhere and attach its https URL with --url"
        );
        return ExitCode::from(exit_codes::USAGE);
    }
    let Some(url) = &args.url else {
        eprintln!("E_ATTACH_NO_URL: pass --url");
        return ExitCode::from(exit_codes::ATTACH_NO_URL);
    };
    let client = match writer() {
        Ok(client) => client,
        Err(code) => return code,
    };
    let story = match story_in(&client, &args.reference) {
        Ok(story) => story,
        Err(code) => return code,
    };
    report(
        client.attach_link(story, url),
        ShortcutKey::Story(story),
        keywords::Attach::Attached.keyword(),
    )
}
