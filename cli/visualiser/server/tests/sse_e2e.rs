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
        let response = reqwest::Client::new()
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
        templates: HashMap::new(),
        work_item: None,
        kanban_columns: None,
        idle_timeout: None,
        editor: None,
        editor_project: None,
    }
}
