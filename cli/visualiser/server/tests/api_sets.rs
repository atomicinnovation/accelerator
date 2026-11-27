use std::path::Path;
use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::config::Config;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use corpus::doc_type::{DocType, Listing};
use corpus::set::{
    MemberKind, MemberSlot, Presentation, SetDeclaration, SetShape,
};
use corpus::DocTypeKey;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

mod common;

const TOPICS: &str = "meta/research/topics";
const INVENTORIES: &str = "meta/research/design-inventories";

fn write_files(root: &Path, files: &[(&str, &str)]) {
    for (file, content) in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
}

fn topic_cfg(tmp: &Path) -> Config {
    let mut cfg = common::seeded_cfg(tmp);
    cfg.doc_paths
        .insert("research_topics".into(), tmp.join(TOPICS));
    cfg
}

fn manifest(id: &str) -> String {
    format!("---\nid: \"{id}\"\ntitle: Topic {id}\n---\nThe manifest.\n")
}

fn titled(title: &str) -> String {
    format!("---\ntitle: {title}\n---\nBody of {title}.\n")
}

async fn state_for(cfg: Config) -> Arc<AppState> {
    AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .unwrap()
}

async fn get_json(state: &Arc<AppState>, uri: &str) -> (StatusCode, Value) {
    let res = build_router(state.clone())
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn fetch_set(state: &Arc<AppState>, uri: &str) -> Value {
    let (status, body) = get_json(state, uri).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    body
}

fn member_paths(set: &Value) -> Vec<String> {
    set["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["setRelativePath"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn set_returns_its_root_and_ten_members() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    let members = [
        "brief.md",
        "outline.md",
        "synthesis.md",
        "findings/01-a.md",
        "findings/02-b.md",
        "findings/03-c.md",
        "findings/04-d.md",
        "findings/05-e.md",
        "reports/r1.md",
        "reports/r2.md",
    ];
    let mut files = vec![(
        "meta/research/topics/ten/manifest.md".to_owned(),
        manifest("ten"),
    )];
    for member in members {
        files.push((
            format!("meta/research/topics/ten/{member}"),
            titled(member),
        ));
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, content)| (path.as_str(), content.as_str()))
        .collect();
    write_files(tmp.path(), &borrowed);
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/topic-research/ten").await;

    assert_eq!(
        set["set"],
        serde_json::json!({
            "type": "topic-research",
            "id": "ten",
            "directory": "meta/research/topics/ten",
            "presentation": "set-page",
        })
    );
    assert_eq!(set["root"]["kind"], "document");
    assert_eq!(set["root"]["setRelativePath"], "manifest.md");
    assert_eq!(set["root"]["title"], "Topic ten");
    assert_eq!(set["root"]["frontmatterState"], "parsed");
    assert_eq!(set["root"]["frontmatter"]["id"], "ten");
    assert!(set["root"]["etag"].as_str().unwrap().starts_with("sha256-"));
    assert!(set["root"]["mtimeMs"].as_i64().is_some());
    let mut expected: Vec<&str> = members.to_vec();
    expected.sort_unstable();
    assert_eq!(member_paths(&set), expected);
    for member in set["members"].as_array().unwrap() {
        assert_eq!(member["kind"], "document");
        assert!(member["etag"].as_str().unwrap().starts_with("sha256-"));
    }
}

#[tokio::test]
async fn set_members_at_depth_carry_set_relative_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            ("meta/research/topics/deep/manifest.md", &manifest("deep")),
            ("meta/research/topics/deep/brief.md", &titled("Brief")),
            ("meta/research/topics/deep/findings/01-a.md", &titled("A")),
            (
                "meta/research/topics/deep/findings/01-a.levels/2-1.md",
                &titled("Level"),
            ),
            ("meta/research/topics/deep/reports/r.md", &titled("Report")),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/topic-research/deep").await;

    assert_eq!(
        member_paths(&set),
        [
            "brief.md",
            "findings/01-a.levels/2-1.md",
            "findings/01-a.md",
            "reports/r.md",
        ]
    );
    let titles: Vec<&str> = set["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Brief", "Level", "A", "Report"]);
}

#[tokio::test]
async fn set_with_a_malformed_root_is_addressed_by_its_directory_name() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            (
                "meta/research/topics/broken/manifest.md",
                "---\nid: \"never-read\"\ntitle: Broken\n",
            ),
            ("meta/research/topics/broken/brief.md", &titled("Brief")),
            ("meta/research/topics/broken/findings/01-a.md", &titled("A")),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/topic-research/broken").await;

    assert_eq!(set["set"]["id"], "broken");
    assert_eq!(set["root"]["frontmatterState"], "malformed");
    assert_eq!(member_paths(&set), ["brief.md", "findings/01-a.md"]);
}

#[tokio::test]
async fn set_member_documents_carry_their_frontmatter_state() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            ("meta/research/topics/s/manifest.md", &manifest("s")),
            ("meta/research/topics/s/brief.md", "No frontmatter.\n"),
            (
                "meta/research/topics/s/outline.md",
                "---\ntitle: Unclosed\n",
            ),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/topic-research/s").await;

    let states: Vec<&str> = set["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["frontmatterState"].as_str().unwrap())
        .collect();
    assert_eq!(states, ["absent", "malformed"]);
}

#[tokio::test]
async fn set_ids_name_the_set_by_its_roots_id() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            (
                "meta/research/topics/2026-01-01-dir/manifest.md",
                &manifest("x"),
            ),
            (
                "meta/research/design-inventories/2026-01-01-inv/inventory.md",
                "---\nid: \"y\"\ntitle: Inv\n---\nBody.\n",
            ),
        ],
    );
    let state = state_for(cfg).await;

    let topic = fetch_set(&state, "/api/sets/topic-research/x").await;
    let inventory = fetch_set(&state, "/api/sets/design-inventories/y").await;

    assert_eq!(topic["set"]["id"], "x");
    assert_eq!(
        topic["set"]["directory"],
        "meta/research/topics/2026-01-01-dir"
    );
    assert_eq!(inventory["set"]["id"], "y");
    assert_eq!(inventory["set"]["presentation"], "entry-point");
    assert_eq!(inventory["root"]["setRelativePath"], "inventory.md");
}

#[tokio::test]
async fn set_assets_keep_unencoded_names_with_size_and_mtime() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let screenshot = "Some File (v2).png";
    write_files(
        tmp.path(),
        &[
            (
                "meta/research/design-inventories/inv/inventory.md",
                "---\nid: \"inv\"\ntitle: Inv\n---\nBody.\n",
            ),
            (
                &format!("{INVENTORIES}/inv/screenshots/{screenshot}"),
                "12345",
            ),
        ],
    );
    common::set_mtime_ms(
        &tmp.path()
            .join(INVENTORIES)
            .join("inv/screenshots")
            .join(screenshot),
        1_700_000_000_000,
    )
    .unwrap();
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/design-inventories/inv").await;

    assert_eq!(
        set["members"],
        serde_json::json!([{
            "kind": "asset",
            "setRelativePath": "screenshots/Some File (v2).png",
            "size": 5,
            "mtimeMs": 1_700_000_000_000_i64,
        }])
    );
}

#[tokio::test]
async fn set_lookup_misses_are_not_found_and_unknown_types_are_bad_requests() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[("meta/research/topics/s/manifest.md", &manifest("s"))],
    );
    let state = state_for(cfg).await;
    fetch_set(&state, "/api/sets/topic-research/s").await;

    let (unknown_set, unknown_set_body) =
        get_json(&state, "/api/sets/topic-research/missing").await;
    let (flat_type, flat_type_body) =
        get_json(&state, "/api/sets/plans/2026-04-18-foo").await;
    let (unknown_type, _) = get_json(&state, "/api/sets/no-such-type/s").await;

    assert_eq!(unknown_set, StatusCode::NOT_FOUND);
    assert_eq!(unknown_set_body["error"], "missing");
    assert_eq!(flat_type, StatusCode::NOT_FOUND);
    assert_eq!(flat_type_body["error"], "2026-04-18-foo");
    assert_eq!(unknown_type, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn set_documents_carry_no_body() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            ("meta/research/topics/s/manifest.md", &manifest("s")),
            ("meta/research/topics/s/brief.md", &titled("Brief")),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/topic-research/s").await;

    let documents =
        std::iter::once(&set["root"]).chain(set["members"].as_array().unwrap());
    for document in documents {
        let fields = document.as_object().unwrap();
        assert!(
            !fields.contains_key("body") && !fields.contains_key("bodyPreview"),
            "{document}"
        );
    }
}

#[tokio::test]
async fn set_whose_root_is_deleted_is_gone_with_its_members() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            ("meta/research/topics/s/manifest.md", &manifest("s")),
            ("meta/research/topics/s/findings/01-a.md", &titled("A")),
        ],
    );
    let state = state_for(cfg).await;
    fetch_set(&state, "/api/sets/topic-research/s").await;

    std::fs::remove_file(tmp.path().join(TOPICS).join("s/manifest.md"))
        .unwrap();
    state
        .indexer
        .rescan_with_diff(
            &accelerator_visualiser::change_broadcast::ChangeBroadcast::of(
                &state,
            ),
        )
        .await
        .unwrap();

    let (set, _) = get_json(&state, "/api/sets/topic-research/s").await;
    let (member, _) =
        get_json(&state, "/api/docs/meta/research/topics/s/findings/01-a.md")
            .await;
    assert_eq!(set, StatusCode::NOT_FOUND);
    assert_eq!(member, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn set_id_shared_by_two_roots_names_the_lexically_first_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            (
                "meta/research/topics/b-second/manifest.md",
                &manifest("dup"),
            ),
            ("meta/research/topics/a-first/manifest.md", &manifest("dup")),
            ("meta/research/topics/c-third/manifest.md", &manifest("dup")),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/topic-research/dup").await;

    assert_eq!(set["set"]["directory"], "meta/research/topics/a-first");
}

#[tokio::test]
async fn set_id_is_percent_decoded_exactly_once() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            (
                "meta/research/topics/odd name %41/manifest.md",
                "---\ntitle: Odd\n---\n",
            ),
            ("meta/research/topics/odd name A/manifest.md", &titled("A")),
        ],
    );
    let state = state_for(cfg).await;

    let set =
        fetch_set(&state, "/api/sets/topic-research/odd%20name%20%2541").await;

    assert_eq!(set["set"]["id"], "odd name %41");
    assert_eq!(set["set"]["directory"], "meta/research/topics/odd name %41");
}

#[tokio::test]
async fn set_members_are_in_set_relative_path_order() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            (
                "meta/research/design-inventories/inv/inventory.md",
                "---\nid: \"inv\"\ntitle: Inv\n---\nBody.\n",
            ),
            (
                "meta/research/design-inventories/inv/screenshots/c.png",
                "c",
            ),
            (
                "meta/research/design-inventories/inv/screenshots/a.png",
                "a",
            ),
            (
                "meta/research/design-inventories/inv/screenshots/b.png",
                "b",
            ),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/design-inventories/inv").await;

    assert_eq!(
        member_paths(&set),
        [
            "screenshots/a.png",
            "screenshots/b.png",
            "screenshots/c.png"
        ]
    );
}

#[tokio::test]
async fn set_documents_and_assets_interleave_in_set_relative_path_order() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = topic_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            ("meta/research/topics/s/manifest.md", &manifest("s")),
            ("meta/research/topics/s/synthesis.md", &titled("Synthesis")),
            ("meta/research/topics/s/reports/r.md", &titled("Report")),
            ("meta/research/topics/s/brief.md", &titled("Brief")),
            ("meta/research/topics/s/findings/02-b.md", &titled("B")),
            ("meta/research/topics/s/findings/01-a.md", &titled("A")),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/topic-research/s").await;

    assert_eq!(
        member_paths(&set),
        [
            "brief.md",
            "findings/01-a.md",
            "findings/02-b.md",
            "reports/r.md",
            "synthesis.md",
        ]
    );
}

#[tokio::test]
async fn set_of_an_inventory_holds_no_prototype_or_stray_file() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    write_files(
        tmp.path(),
        &[
            (
                "meta/research/design-inventories/inv/inventory.md",
                "---\nid: \"inv\"\ntitle: Inv\n---\nBody.\n",
            ),
            (
                "meta/research/design-inventories/inv/screenshots/a.png",
                "a",
            ),
            (
                "meta/research/design-inventories/inv/prototype-full/x.html",
                "<html></html>",
            ),
            (
                "meta/research/design-inventories/inv/assets/y.svg",
                "<svg/>",
            ),
            ("meta/research/design-inventories/inv/notes.txt", "notes"),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/design-inventories/inv").await;

    assert_eq!(member_paths(&set), ["screenshots/a.png"]);
}

static ALBUMS: DocType = DocType {
    wire_token: "albums",
    config_path_key: Some("albums"),
    label: "Albums",
    listing: Listing::Set,
    ..DocType::FLAT_DEFAULTS
};

static ALBUM: SetDeclaration = SetDeclaration {
    shape: SetShape {
        root: "album.md",
        member_slots: &[
            MemberSlot {
                pattern: "pages/*.md",
                kind: MemberKind::Document,
            },
            MemberSlot {
                pattern: "pages/*.png",
                kind: MemberKind::Asset,
            },
        ],
    },
    presentation: Presentation::SetPage,
};

#[tokio::test]
async fn set_documents_and_assets_are_ordered_together() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    cfg.doc_paths
        .insert("albums".into(), tmp.path().join("meta/albums"));
    cfg.doc_types = Some(common::catalogue_with(
        &[&ALBUMS],
        &[(DocTypeKey::declared(&ALBUMS), &ALBUM)],
    ));
    write_files(
        tmp.path(),
        &[
            ("meta/albums/trip/album.md", &manifest("trip")),
            ("meta/albums/trip/pages/c.png", "c"),
            ("meta/albums/trip/pages/b.md", &titled("B")),
            ("meta/albums/trip/pages/a.png", "a"),
        ],
    );
    let state = state_for(cfg).await;

    let set = fetch_set(&state, "/api/sets/albums/trip").await;

    assert_eq!(
        member_paths(&set),
        ["pages/a.png", "pages/b.md", "pages/c.png"]
    );
    let kinds: Vec<&str> = set["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["asset", "document", "asset"]);
}
