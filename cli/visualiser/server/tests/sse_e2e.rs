//! End-to-end test: file mutation → SSE event via the full server.
//!
//! The server learns of each change through an injected watcher rather than
//! the OS's file notifications, so these tests cover everything from the
//! directories the server watches to the SSE frame it sends, without
//! waiting on `FSEvents` or inotify. The OS watcher itself is covered by
//! `watcher::tests::the_os_watcher_reports_a_write_under_a_registered_directory`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use accelerator_visualiser::watcher::{DirectoryWatcher, EventSink};
use notify::EventHandler as _;

mod common;
use sha2::Digest;

#[tokio::test]
async fn file_mutation_arrives_as_sse_event() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    std::fs::create_dir_all(&plans).unwrap();
    let plan = plans.join("2026-01-01-test.md");
    std::fs::write(&plan, "---\ntitle: T\n---\n").unwrap();

    let mut doc_paths = HashMap::new();
    doc_paths.insert("plans".into(), plans.clone());
    let cfg = accelerator_visualiser::config::Config {
        doc_paths,
        ..base_cfg(tmp.path())
    };

    let mut server = start_server(cfg, tmp.path()).await;
    let mut sse_response = server.events().await;
    assert!(
        server.watched().contains(&plans.canonicalize().unwrap()),
        "the server did not watch its doc path: {:?}",
        server.watched(),
    );

    std::fs::write(&plan, "---\ntitle: Updated\n---\n").unwrap();
    server.changed(&plan);

    read_until(&mut sse_response, "doc-changed").await;
}

#[tokio::test]
async fn template_file_mutation_arrives_as_template_changed_sse_event() {
    let tmp = tempfile::tempdir().unwrap();
    let tier_file = template_tier(tmp.path(), "v1");

    let mut server =
        start_server(template_cfg(tmp.path(), tier_file.clone()), tmp.path())
            .await;
    let mut sse_response = server.events().await;
    assert!(
        server
            .watched()
            .contains(&tier_file.parent().unwrap().canonicalize().unwrap()),
        "the server did not watch its template tier: {:?}",
        server.watched(),
    );

    std::fs::write(&tier_file, "v2").unwrap();
    server.changed(&tier_file);

    let payload = template_changed(
        &read_until(&mut sse_response, "\"type\":\"template-changed\"").await,
    );
    assert_eq!(payload["template"], "adr");
    let expected =
        format!("sha256-{}", hex::encode(sha2::Sha256::digest(b"v2")));
    assert_eq!(payload["sha256"], expected);
}

#[tokio::test]
async fn template_file_emptied_arrives_as_template_changed_with_no_sha256() {
    let tmp = tempfile::tempdir().unwrap();
    let tier_file = template_tier(tmp.path(), "initial");

    let mut server =
        start_server(template_cfg(tmp.path(), tier_file.clone()), tmp.path())
            .await;
    let mut sse_response = server.events().await;

    std::fs::write(&tier_file, "").unwrap();
    server.changed(&tier_file);

    let payload = template_changed(
        &read_until(&mut sse_response, "\"type\":\"template-changed\"").await,
    );
    assert_eq!(payload["template"], "adr");
    assert!(
        payload.get("sha256").is_none(),
        "expected no sha256 key on empty-content transition, got: {payload}",
    );
}

const TOPICS: &str = "meta/research/topics";
const INVENTORIES: &str = "meta/research/design-inventories";
const WORK_ITEM: &str = "meta/work/0001-todo.md";

fn write_files(root: &Path, files: &[(&str, &str)]) {
    for (file, content) in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
}

fn set_cfg(tmp: &Path) -> accelerator_visualiser::config::Config {
    let mut doc_paths = HashMap::new();
    for (key, dir) in [
        ("research_topics", TOPICS),
        ("research_design_inventories", INVENTORIES),
        ("work", "meta/work"),
    ] {
        std::fs::create_dir_all(tmp.join(dir)).unwrap();
        doc_paths.insert(key.into(), tmp.join(dir));
    }
    accelerator_visualiser::config::Config {
        doc_paths,
        ..base_cfg(tmp)
    }
}

impl StartedServer {
    fn changed_all(&mut self, paths: &[PathBuf]) {
        let event = paths
            .iter()
            .fold(notify::Event::new(notify::EventKind::Any), |event, path| {
                event.add_path(path.clone())
            });
        self.changes
            .as_mut()
            .expect("events() opens the watcher first")
            .handle_event(Ok(event));
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    async fn get(&self, path: &str) -> (u16, serde_json::Value) {
        let response = common::http_client()
            .build()
            .unwrap()
            .get(self.url(path))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body = response.json().await.unwrap_or(serde_json::Value::Null);
        (status, body)
    }
}

fn set_paths(set: &serde_json::Value) -> (String, Vec<String>) {
    let members = set["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["setRelativePath"].as_str().unwrap().to_owned())
        .collect();
    (
        set["root"]["setRelativePath"].as_str().unwrap().to_owned(),
        members,
    )
}

/// Every `doc-changed` payload on the stream within `window`.
async fn doc_changes_within(
    sse_response: &mut reqwest::Response,
    window: Duration,
) -> Vec<serde_json::Value> {
    let mut accumulated = String::new();
    let _ = tokio::time::timeout(window, async {
        while let Ok(Some(chunk)) = sse_response.chunk().await {
            accumulated.push_str(&String::from_utf8_lossy(&chunk));
        }
    })
    .await;
    accumulated
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|json| serde_json::from_str(json).ok())
        .filter(|v: &serde_json::Value| v["type"] == "doc-changed")
        .collect()
}

#[tokio::test]
async fn an_asset_renamed_only_in_case_is_listed_once() {
    let tmp = tempfile::tempdir().unwrap();
    let inventory = tmp.path().join(INVENTORIES).join("i");
    write_files(
        &inventory,
        &[
            ("inventory.md", "---\nid: \"i\"\n---\n"),
            ("screenshots/a.png", "a"),
        ],
    );
    let mut server = start_server(set_cfg(tmp.path()), tmp.path()).await;
    let mut sse_response = server.events().await;
    let screenshots = inventory.join("screenshots");
    std::fs::rename(screenshots.join("a.png"), screenshots.join("a.PNG"))
        .unwrap();

    server.changed_all(&[screenshots.join("a.png"), screenshots.join("a.PNG")]);

    read_until(&mut sse_response, "a.PNG").await;
    let (status, set) = server.get("/api/sets/design-inventories/i").await;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set_paths(&set).1, ["screenshots/a.PNG"]);
}

#[tokio::test]
async fn a_set_renamed_into_place_is_served_with_its_members() {
    let tmp = tempfile::tempdir().unwrap();
    let topics = tmp.path().join(TOPICS);
    let mut server = start_server(set_cfg(tmp.path()), tmp.path()).await;
    let mut sse_response = server.events().await;
    write_files(
        &topics.join(".x.tmp"),
        &[
            ("manifest.md", "---\nid: \"x\"\n---\n"),
            ("brief.md", "---\ntitle: Brief\n---\n"),
            ("outline.md", "---\ntitle: Outline\n---\n"),
            ("notes.txt", "notes"),
        ],
    );
    let set = topics.join("x");
    std::fs::rename(topics.join(".x.tmp"), &set).unwrap();

    server.changed_all(&[
        set.clone(),
        set.join("manifest.md"),
        set.join("brief.md"),
        set.join("outline.md"),
        set.join("notes.txt"),
    ]);

    read_until(&mut sse_response, "doc-changed").await;
    let (status, served) = server.get("/api/sets/topic-research/x").await;
    assert_eq!(status, 200, "{served}");
    assert_eq!(
        set_paths(&served),
        (
            "manifest.md".to_owned(),
            vec!["brief.md".to_owned(), "outline.md".to_owned()]
        )
    );
}

#[tokio::test]
async fn a_deleted_root_takes_its_set_and_members_with_it() {
    let tmp = tempfile::tempdir().unwrap();
    let set = tmp.path().join(TOPICS).join("s");
    write_files(
        &set,
        &[
            ("manifest.md", "---\nid: \"s\"\n---\n"),
            ("brief.md", "---\ntitle: Brief\n---\n"),
            ("outline.md", "---\ntitle: Outline\n---\n"),
        ],
    );
    let mut server = start_server(set_cfg(tmp.path()), tmp.path()).await;
    let mut sse_response = server.events().await;
    assert_eq!(server.get("/api/sets/topic-research/s").await.0, 200);
    std::fs::remove_file(set.join("manifest.md")).unwrap();

    server.changed(&set.join("manifest.md"));

    read_until(&mut sse_response, "deleted").await;
    let (_, listing) = server.get("/api/docs?type=topic-research").await;
    assert_eq!(
        listing["docs"].as_array().map(Vec::len),
        Some(0),
        "{listing}"
    );
    assert_eq!(server.get("/api/sets/topic-research/s").await.0, 404);
    for member in ["brief.md", "outline.md"] {
        let (status, _) =
            server.get(&format!("/api/docs/{TOPICS}/s/{member}")).await;
        assert_eq!(status, 404, "{member}");
    }
}

#[tokio::test]
async fn a_patch_during_a_member_debounce_emits_each_change_once() {
    let tmp = tempfile::tempdir().unwrap();
    let set = tmp.path().join(TOPICS).join("s");
    write_files(
        &set,
        &[
            ("manifest.md", "---\nid: \"s\"\n---\n"),
            ("brief.md", "---\ntitle: Brief\n---\n"),
        ],
    );
    write_files(
        tmp.path(),
        &[(
            WORK_ITEM,
            "---\ntitle: Todo\ntype: task\nstatus: todo\n---\n",
        )],
    );
    let mut server = start_server(set_cfg(tmp.path()), tmp.path()).await;
    let mut sse_response = server.events().await;
    let client = common::http_client().build().unwrap();
    let etag = client
        .get(server.url(&format!("/api/docs/{WORK_ITEM}")))
        .send()
        .await
        .unwrap()
        .headers()[reqwest::header::ETAG]
        .to_str()
        .unwrap()
        .to_owned();

    std::fs::write(set.join("brief.md"), "---\ntitle: Changed\n---\n").unwrap();
    server.changed(&set.join("brief.md"));
    let patched = client
        .patch(server.url(&format!("/api/docs/{WORK_ITEM}/frontmatter")))
        .header(reqwest::header::IF_MATCH, etag)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(r#"{"patch":{"status":"done"}}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(patched.status().as_u16(), 204);
    let written = patched.headers()[reqwest::header::ETAG]
        .to_str()
        .unwrap()
        .trim_matches('"')
        .to_owned();

    let changes = doc_changes_within(&mut sse_response, BUDGET / 5).await;

    let paths: Vec<&str> = changes
        .iter()
        .map(|c| c["path"].as_str().unwrap())
        .collect();
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        [format!("{TOPICS}/s/brief.md"), WORK_ITEM.to_owned()],
        "{changes:?}"
    );
    let patch_event = changes.iter().find(|c| c["path"] == WORK_ITEM).unwrap();
    assert_eq!(patch_event["etag"], written);
}

/// A watcher that records the directories the server asks it to watch and
/// reports only the changes a test injects.
struct InjectedWatcher {
    watched: Arc<Mutex<Vec<PathBuf>>>,
}

impl DirectoryWatcher for InjectedWatcher {
    fn watch_recursively(&mut self, dir: &Path) -> notify::Result<()> {
        self.watched.lock().unwrap().push(dir.to_path_buf());
        Ok(())
    }
}

struct StartedServer {
    port: u16,
    watching: Option<tokio::sync::oneshot::Receiver<()>>,
    sink: Option<tokio::sync::oneshot::Receiver<EventSink>>,
    changes: Option<EventSink>,
    watched: Arc<Mutex<Vec<PathBuf>>>,
}

impl StartedServer {
    /// The SSE stream, opened once the server has registered its watches,
    /// so every change injected from here on is observed.
    async fn events(&mut self) -> reqwest::Response {
        let url = format!("http://127.0.0.1:{}/api/events", self.port);
        let response = common::http_client()
            .build()
            .expect("an http client")
            .get(&url)
            .send()
            .await
            .expect("GET /api/events failed");
        assert_eq!(response.status(), 200);
        within_budget(self.watching.take().unwrap())
            .await
            .expect("the server stopped before registering its watches");
        self.changes = Some(
            within_budget(self.sink.take().unwrap())
                .await
                .expect("the server never opened its watcher"),
        );
        response
    }

    fn watched(&self) -> Vec<PathBuf> {
        self.watched.lock().unwrap().clone()
    }

    fn changed(&mut self, path: &Path) {
        self.changes
            .as_mut()
            .expect("events() opens the watcher first")
            .handle_event(Ok(notify::Event::new(notify::EventKind::Modify(
                notify::event::ModifyKind::Any,
            ))
            .add_path(path.to_path_buf())));
    }
}

async fn start_server(
    cfg: accelerator_visualiser::config::Config,
    tmp: &Path,
) -> StartedServer {
    let info_path = tmp.join("server-info.json");
    let (watching_tx, watching) = tokio::sync::oneshot::channel();
    let (sink_tx, sink) = tokio::sync::oneshot::channel();
    let watched = Arc::new(Mutex::new(Vec::new()));
    let recorded = watched.clone();
    let info_path_clone = info_path.clone();
    tokio::spawn(async move {
        accelerator_visualiser::server::run_with(
            cfg,
            &info_path_clone,
            accelerator_visualiser::server::StartupSignals {
                watching: Some(watching_tx),
            },
            move |sink| {
                let _ = sink_tx.send(sink);
                Ok(InjectedWatcher { watched: recorded })
            },
        )
        .await
        .unwrap();
    });
    let port = within_budget(async {
        loop {
            if let Ok(bytes) = std::fs::read(&info_path) {
                let v: serde_json::Value =
                    serde_json::from_slice(&bytes).unwrap();
                break v["port"].as_u64().unwrap() as u16;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    StartedServer {
        port,
        watching: Some(watching),
        sink: Some(sink),
        changes: None,
        watched,
    }
}

/// A hang detector, not a latency budget: nothing here waits on the OS, so
/// a step that takes this long is stuck.
const BUDGET: Duration = Duration::from_secs(10);

async fn within_budget<T>(step: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(BUDGET, step)
        .await
        .expect("a server step exceeded its budget")
}

/// Every SSE frame read until one contains `needle`.
async fn read_until(
    sse_response: &mut reqwest::Response,
    needle: &str,
) -> String {
    within_budget(async {
        let mut accumulated = String::new();
        while !accumulated.contains(needle) {
            let chunk = sse_response
                .chunk()
                .await
                .expect("the SSE stream failed")
                .expect("the SSE stream ended");
            accumulated.push_str(&String::from_utf8_lossy(&chunk));
        }
        accumulated
    })
    .await
}

fn template_changed(frames: &str) -> serde_json::Value {
    let json = frames
        .lines()
        .find(|l| l.contains("\"type\":\"template-changed\""))
        .and_then(|l| l.strip_prefix("data: "))
        .expect("expected data: line with template-changed payload");
    serde_json::from_str(json).unwrap()
}

fn template_tier(tmp: &Path, content: &str) -> PathBuf {
    let tier_dir = tmp.join("templates");
    std::fs::create_dir_all(&tier_dir).unwrap();
    let tier_file = tier_dir.join("adr.md");
    std::fs::write(&tier_file, content).unwrap();
    tier_file
}

fn template_cfg(
    tmp: &Path,
    tier_file: PathBuf,
) -> accelerator_visualiser::config::Config {
    let mut templates = HashMap::new();
    templates.insert(
        "adr".to_string(),
        accelerator_visualiser::config::TemplateTiers {
            config_override: None,
            user_override: tier_file.parent().unwrap().join("missing-user.md"),
            plugin_default: tier_file,
            config_override_source: None,
        },
    );
    accelerator_visualiser::config::Config {
        templates,
        ..base_cfg(tmp)
    }
}

fn base_cfg(tmp: &Path) -> accelerator_visualiser::config::Config {
    accelerator_visualiser::config::Config {
        plugin_root: tmp.to_path_buf(),
        plugin_version: "test".into(),
        project_root: tmp.to_path_buf(),
        tmp_path: tmp.to_path_buf(),
        host: "127.0.0.1".into(),
        owner_pid: 0,
        owner_start_time: None,
        log_path: tmp.join("server.log"),
        doc_paths: HashMap::new(),
        doc_types: Some(declarations::catalogue().unwrap()),
        templates: HashMap::new(),
        work_item: accelerator_visualiser::config::WorkItemScheme::Default,
        kanban_columns: None,
        idle_timeout: None,
        editor: None,
        editor_project: None,
    }
}
