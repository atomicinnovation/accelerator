//! The live-workspace harness.
//!
//! Named exactly `contract.rs`, so `cli/.config/nextest.toml`'s
//! `not binary(=contract)` keeps it out of every default run; only
//! `mise run test:integration:tracker-contract` selects it. An unconfigured
//! run fails, naming the variables it wants.
//!
//! `records_the_v4_facts` writes Shortcut's raw answers to the gitignored
//! `tests/captures/`. A person sanitises those into `tests/fixtures/v4/`,
//! which `evidence_hygiene.rs` guards. Everything the run creates is archived
//! before it ends.

#![allow(clippy::expect_used, clippy::panic, clippy::too_many_lines)]

mod support;

use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use config::credentials::Secret;
use config::credentials::TokenSource;
use serde_json::json;
use serde_json::Value;
use shortcut_client::catalogue::StateType;
use shortcut_client::discovery::discover;
use shortcut_client::paths::Paths;
use shortcut_client::paths::PRODUCTION_BASE;
use shortcut_client::transport::Deadline;
use shortcut_client::transport::Delivery;
use shortcut_client::transport::Resend;
use shortcut_client::transport::Transport;
use shortcut_client::Credentials;
use shortcut_client::Method;
use shortcut_client::Url;
use tracker_support::ClockJitter;
use tracker_support::SystemSleeper;
use tracker_support::TransportConfig;

const TOKEN: &str = "ACCELERATOR_SHORTCUT_TOKEN";
const TEAM_KEY: &str = "ACCELERATOR_SHORTCUT_TEAM_KEY";
const READ_ONLY_TOKEN: &str = "ACCELERATOR_SHORTCUT_READ_ONLY_TOKEN";

const MARKDOWN: &str = "Contract body\n\n\
    A paragraph that wraps across\n\
    1. a line that starts with a digit\n\n\
    - a list item\n\
    - another, with `code` and a.dotted.token\n\n\
    ```\n    indented code\n```\n\n\
    | a | b |\n|---|---|\n| 1 | 2 |\n";

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!(
            "{name} is not set. A live Shortcut run needs \
             ACCELERATOR_TRACKER_CONTRACT=1, {TOKEN} (an sct_rw_ token for a \
             scratch workspace) and {TEAM_KEY}; {READ_ONLY_TOKEN} is optional"
        )
    })
}

fn opted_in() {
    assert_eq!(
        std::env::var("ACCELERATOR_TRACKER_CONTRACT").as_deref(),
        Ok("1"),
        "ACCELERATOR_TRACKER_CONTRACT=1 is not set — run this harness through \
         `mise run test:integration:tracker-contract`"
    );
}

fn transport(token: &str) -> Transport {
    Transport::new(
        &Credentials {
            token: Secret::new(token.to_owned()),
            source: TokenSource::Env,
            refusals: Vec::new(),
            notice: None,
        },
        TransportConfig::default(),
        Box::new(SystemSleeper),
        Box::new(ClockJitter),
    )
    .expect("the transport builds")
}

fn captures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/captures")
}

struct Recorder {
    dir: PathBuf,
}

impl Recorder {
    fn new() -> Self {
        let dir = captures_dir();
        std::fs::create_dir_all(&dir).expect("the captures directory");
        Self { dir }
    }

    fn record(
        &self,
        name: &str,
        request: &str,
        delivery: &Delivery,
    ) -> Option<Value> {
        let (status, body) = match &delivery.result {
            Ok(received) => (
                json!(received.status),
                received.json().unwrap_or_else(|| json!(received.body)),
            ),
            Err(error) => (json!(null), json!(error.to_string())),
        };
        let capture = json!({
            "request": request,
            "status": status,
            "may_have_applied": delivery.may_have_applied,
            "body": body,
        });
        std::fs::write(
            self.dir.join(format!("{name}.json")),
            serde_json::to_string_pretty(&capture).expect("json"),
        )
        .expect("write the capture");
        delivery
            .result
            .as_ref()
            .ok()
            .filter(|received| received.succeeded())
            .and_then(shortcut_client::transport::Received::json)
    }
}

fn entity_id(body: Option<&Value>) -> Option<i64> {
    let body = body?;
    body.pointer("/entity/id")
        .or_else(|| body.get("id"))
        .and_then(Value::as_i64)
}

fn with_query(mut url: Url, pairs: &[(&str, &str)]) -> Url {
    {
        let mut query = url.query_pairs_mut();
        for (name, value) in pairs {
            query.append_pair(name, value);
        }
    }
    url
}

#[test]
fn records_the_v4_facts() {
    opted_in();
    let token = required(TOKEN);
    let team_key = required(TEAM_KEY);
    let base = Url::parse(PRODUCTION_BASE).expect("parses");
    let live = transport(&token);
    let deadline = Deadline::starting_now(Duration::from_secs(600));
    let recorder = Recorder::new();
    let send = |method: &Method, url: &Url, body: &Value, name: &str| {
        let delivery =
            live.send(method, url, body, Resend::OnlyWhenThrottled, &deadline);
        recorder.record(name, &format!("{method} {url} {body}"), &delivery)
    };
    let get = |url: &Url, name: &str| {
        let delivery = live.get(url, &deadline);
        recorder.record(name, &format!("GET {url}"), &delivery)
    };

    let whoami = get(&Paths::whoami(&base), "whoami");
    let unauthorised = transport("sct_rw_not_a_real_token");
    recorder.record(
        "error-401",
        "GET whoami with a bad token",
        &unauthorised.get(&Paths::whoami(&base), &deadline),
    );
    assert!(
        whoami.is_some(),
        "whoami must answer with a read-write token"
    );

    let (me, discovered) =
        discover(&live, &base, &deadline).expect("discovers");
    let paths = Paths::new(base.clone(), me.slug.clone());
    let catalogue = discovered.catalogue;
    let team = catalogue
        .team_for_key(&team_key)
        .expect("the team key resolves");
    get(&paths.team(&team.id), "team-by-uuid");
    get(
        &paths.team("00000000-0000-4000-8000-000000000000"),
        "team-unknown-404",
    );
    get(&paths.members(), "members");
    let workflow = team
        .default_workflow_id
        .and_then(|id| catalogue.workflow(id))
        .expect("the team has a default workflow");
    let unstarted = workflow
        .states
        .iter()
        .filter(|state| state.kind == StateType::Unstarted)
        .min_by_key(|state| state.position)
        .expect("the default workflow has an unstarted state");

    let story_body = json!({
        "name": "Contract story",
        "description": MARKDOWN,
        "story_type": "feature",
        "team_id": team.id,
        "workflow_state_id": unstarted.id,
    });
    let story = send(
        &Method::POST,
        &paths.collection(shortcut_client::Collection::Stories),
        &story_body,
        "story-create",
    );
    let story_id = entity_id(story.as_ref()).expect("the story was created");
    let story_key =
        shortcut_client::ShortcutKey::parse(&format!("sc-{story_id}"))
            .expect("a story key");
    get(
        &with_query(
            paths.entity(story_key),
            &[("fields", "id,name,description,updated_at,app_url")],
        ),
        "story-show-fields",
    );
    get(&paths.entity(story_key), "story-show-full");
    get(&paths.comments(story_key), "story-comments-list");
    send(
        &Method::POST,
        &paths.comments(story_key),
        &json!({"text": "Contract comment"}),
        "story-comment-create",
    );
    send(
        &Method::PATCH,
        &paths.entity(story_key),
        &json!({"external_links_add": ["https://example.com/contract"]}),
        "story-links-add",
    );
    send(
        &Method::PATCH,
        &paths.entity(story_key),
        &json!({"external_links_add": ["https://example.com/contract"]}),
        "story-links-add-repeat",
    );
    send(
        &Method::PATCH,
        &paths.entity(story_key),
        &json!({"estimate": 2, "owner_ids": [me.member_id]}),
        "story-update-estimate-owner",
    );
    if let Some(other) = workflow
        .states
        .iter()
        .find(|state| state.id != unstarted.id)
    {
        send(
            &Method::PATCH,
            &paths.entity(story_key),
            &json!({"workflow_state_id": other.id}),
            "story-transition",
        );
    }
    send(
        &Method::PATCH,
        &paths.entity(story_key),
        &json!({"name": ""}),
        "error-422-empty-name",
    );
    get(
        &paths.entity(
            shortcut_client::ShortcutKey::parse("sc-999999999").expect("key"),
        ),
        "error-404-story",
    );

    let archived_body = json!({
        "name": "Contract archived story",
        "story_type": "chore",
        "team_id": team.id,
        "workflow_state_id": unstarted.id,
    });
    let archived = send(
        &Method::POST,
        &paths.collection(shortcut_client::Collection::Stories),
        &archived_body,
        "story-create-archived",
    );
    let archived_id = entity_id(archived.as_ref()).expect("created");
    let archived_key =
        shortcut_client::ShortcutKey::parse(&format!("sc-{archived_id}"))
            .expect("key");
    send(
        &Method::PATCH,
        &paths.entity(archived_key),
        &json!({"archived": true}),
        "story-archive",
    );
    let ids = format!("{story_id},{archived_id},999999999");
    get(
        &with_query(
            paths.collection(shortcut_client::Collection::Stories),
            &[
                ("ids", &ids),
                ("filter", "include_archived"),
                ("limit", "100"),
                ("fields", "id,updated_at,team"),
            ],
        ),
        "stories-by-ids-include-archived",
    );
    get(
        &with_query(
            paths.collection(shortcut_client::Collection::Stories),
            &[("ids", &ids), ("limit", "100")],
        ),
        "stories-by-ids-default-filter",
    );

    let team_name = &team.name;
    for (name, query) in [
        ("search-team", format!("team:\"{team_name}\"")),
        ("search-team-limit-250", format!("team:\"{team_name}\"")),
        ("search-escaped-quote", "title:\"a \\\" b\"".to_owned()),
        (
            "search-label-twice",
            "label:\"alpha\" label:\"beta\"".to_owned(),
        ),
        (
            "search-label-negated",
            format!("team:\"{team_name}\" -label:\"alpha\""),
        ),
        ("search-type", format!("team:\"{team_name}\" type:feature")),
        (
            "search-state",
            format!("team:\"{team_name}\" state:\"{}\"", unstarted.name),
        ),
        ("search-owner", format!("owner:{}", me.member_mention_name)),
    ] {
        let limit = if name == "search-team-limit-250" {
            "250"
        } else {
            "1"
        };
        get(
            &with_query(
                paths.search_stories(),
                &[("query", &query), ("limit", limit)],
            ),
            name,
        );
    }

    let epic = send(
        &Method::POST,
        &paths.collection(shortcut_client::Collection::Epics),
        &json!({"name": "Contract epic", "description": MARKDOWN}),
        "epic-create",
    );
    if let Some(epic_id) = entity_id(epic.as_ref()) {
        let epic_key =
            shortcut_client::ShortcutKey::parse(&format!("epic-{epic_id}"))
                .expect("key");
        get(&paths.entity(epic_key), "epic-show-full");
        get(
            &with_query(
                paths.collection(shortcut_client::Collection::Epics),
                &[("ids", &epic_id.to_string()), ("limit", "100")],
            ),
            "epics-by-ids",
        );
        send(
            &Method::POST,
            &paths.comments(epic_key),
            &json!({"text": "Contract epic comment"}),
            "epic-comment-create",
        );
        get(
            &paths.entity(
                shortcut_client::ShortcutKey::parse(&format!("sc-{epic_id}"))
                    .expect("key"),
            ),
            "story-with-epic-id",
        );
        send(
            &Method::PATCH,
            &paths.entity(epic_key),
            &json!({"archived": true}),
            "epic-archive",
        );
    }

    if let Ok(read_only) = std::env::var(READ_ONLY_TOKEN) {
        let reader = transport(&read_only);
        recorder.record(
            "whoami-read-only",
            "GET whoami with a read-only token",
            &reader.get(&Paths::whoami(&base), &deadline),
        );
        recorder.record(
            "error-403-read-only-create",
            "POST stories with a read-only token",
            &reader.send(
                &Method::POST,
                &paths.collection(shortcut_client::Collection::Stories),
                &story_body,
                Resend::OnlyWhenThrottled,
                &deadline,
            ),
        );
    }

    send(
        &Method::PATCH,
        &paths.entity(story_key),
        &json!({"archived": true}),
        "story-archive-final",
    );

    for entry in std::fs::read_dir(captures_dir())
        .expect("captures")
        .flatten()
    {
        let contents = std::fs::read_to_string(entry.path()).expect("read");
        assert!(
            !contents.contains(&token)
                && !contents.contains("sct_rw_not_a_real_token"),
            "{} holds a token",
            entry.path().display()
        );
    }
}
