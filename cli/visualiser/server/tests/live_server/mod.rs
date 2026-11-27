//! A server started on a tempdir whose watcher reports only the changes a
//! test injects, so a test covers everything from a file change to the
//! event and the API's answer without waiting on the OS.
// Shared by several integration test binaries, each using only part of it.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use accelerator_visualiser::watcher::{DirectoryWatcher, EventSink};
use notify::EventHandler as _;

use crate::common;

/// A watcher that records the directories the server asks it to watch and
/// reports only the changes a test injects.
pub struct InjectedWatcher {
    pub watched: Arc<Mutex<Vec<PathBuf>>>,
}

impl DirectoryWatcher for InjectedWatcher {
    fn watch_recursively(&mut self, dir: &Path) -> notify::Result<()> {
        self.watched.lock().unwrap().push(dir.to_path_buf());
        Ok(())
    }
}

pub struct StartedServer {
    pub port: u16,
    watching: Option<tokio::sync::oneshot::Receiver<()>>,
    sink: Option<tokio::sync::oneshot::Receiver<EventSink>>,
    changes: Option<EventSink>,
    watched: Arc<Mutex<Vec<PathBuf>>>,
}

impl StartedServer {
    /// The SSE stream, opened once the server has registered its watches,
    /// so every change injected from here on is observed.
    pub async fn events(&mut self) -> reqwest::Response {
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

    pub fn watched(&self) -> Vec<PathBuf> {
        self.watched.lock().unwrap().clone()
    }

    pub fn changed(&mut self, path: &Path) {
        self.changes
            .as_mut()
            .expect("events() opens the watcher first")
            .handle_event(Ok(notify::Event::new(notify::EventKind::Modify(
                notify::event::ModifyKind::Any,
            ))
            .add_path(path.to_path_buf())));
    }
}

pub async fn start_server(
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
pub const BUDGET: Duration = Duration::from_secs(10);

pub async fn within_budget<T>(step: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(BUDGET, step)
        .await
        .expect("a server step exceeded its budget")
}

/// Every SSE frame read until one contains `needle`.
pub async fn read_until(
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

impl StartedServer {
    pub fn changed_all(&mut self, paths: &[PathBuf]) {
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

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    pub async fn get(&self, path: &str) -> (u16, serde_json::Value) {
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

pub async fn doc_changes_within(
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

pub fn base_cfg(tmp: &Path) -> accelerator_visualiser::config::Config {
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

pub fn write_files(root: &Path, files: &[(&str, &str)]) {
    for (file, content) in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
}
