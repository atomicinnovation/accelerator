//! End-to-end test: file mutation → SSE event via the full server.

use std::collections::HashMap;
use std::time::Duration;

use sha2::Digest;

#[tokio::test]
async fn file_mutation_arrives_as_sse_event() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(plans.join("2026-01-01-test.md"), "---\ntitle: T\n---\n")
        .unwrap();

    let mut doc_paths = HashMap::new();
    doc_paths.insert("plans".into(), plans.clone());

    let cfg = accelerator_visualiser::config::Config {
        plugin_root: tmp.path().to_path_buf(),
        plugin_version: "test".into(),
        project_root: tmp.path().to_path_buf(),
        tmp_path: tmp.path().to_path_buf(),
        host: "127.0.0.1".into(),
        owner_pid: 0,
        owner_start_time: None,
        log_path: tmp.path().join("server.log"),
        doc_paths,
        templates: HashMap::new(),
        work_item: None,
        kanban_columns: None,
        idle_timeout: None,
        editor: None,
        editor_project: None,
    };

    let info_path = tmp.path().join("server-info.json");
    let info_path_clone = info_path.clone();

    let _handle = tokio::spawn(async move {
        accelerator_visualiser::server::run(cfg, &info_path_clone)
            .await
            .unwrap();
    });

    // Wait for server-info.json.
    let start = std::time::Instant::now();
    let port = loop {
        if let Ok(bytes) = std::fs::read(&info_path) {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            break v["port"].as_u64().unwrap() as u16;
        }
        assert!(
            start.elapsed().as_secs() <= 5,
            "server-info.json did not appear in 5s"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    // Open SSE stream.
    // NOTE: this test requires kernel filesystem notifications (inotify on
    // Linux, FSEvents on macOS). In containerised CI environments using
    // overlayfs or with exhausted inotify watch quotas, the test may be
    // flaky. Ensure `fs.inotify.max_user_watches` is sufficient (≥ 8192
    // recommended) on Linux CI runners.
    let url = format!("http://127.0.0.1:{port}/api/events");
    let client = reqwest::Client::new();
    let mut sse_response = client
        .get(&url)
        .send()
        .await
        .expect("GET /api/events failed");
    assert_eq!(sse_response.status(), 200);

    let event = write_until_event(
        &mut sse_response,
        &plans.join("2026-01-01-test.md"),
        "---\ntitle: Updated\n---\n",
        "doc-changed",
    )
    .await;
    assert!(event.is_some(), "expected doc-changed SSE event");
}

async fn start_server_with_template(
    cfg: accelerator_visualiser::config::Config,
    info_path: std::path::PathBuf,
) -> u16 {
    let info_path_clone = info_path.clone();
    let _handle = tokio::spawn(async move {
        accelerator_visualiser::server::run(cfg, &info_path_clone)
            .await
            .unwrap();
    });
    let start = std::time::Instant::now();
    loop {
        if let Ok(bytes) = std::fs::read(&info_path) {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            break v["port"].as_u64().unwrap() as u16;
        }
        assert!(
            start.elapsed().as_secs() <= 5,
            "server-info.json did not appear in 5s"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Saturated `FSEvents` has been seen to take minutes to register watches.
const WATCH_REGISTRATION_BUDGET: Duration = Duration::from_secs(300);

/// Rewrites `path` with `contents` until an SSE frame containing `needle`
/// arrives. The server answers HTTP before its filesystem watches are
/// registered, so a single write can land before anything is listening;
/// identical rewrites after the first observed one are suppressed server-side.
async fn write_until_event(
    sse_response: &mut reqwest::Response,
    path: &std::path::Path,
    contents: &str,
    needle: &str,
) -> Option<String> {
    let deadline = tokio::time::Instant::now() + WATCH_REGISTRATION_BUDGET;
    let mut accumulated = String::new();
    while tokio::time::Instant::now() < deadline {
        std::fs::write(path, contents).unwrap();
        let rewrite_at = (tokio::time::Instant::now()
            + Duration::from_millis(250))
        .min(deadline);
        while let Ok(read) =
            tokio::time::timeout_at(rewrite_at, sse_response.chunk()).await
        {
            match read {
                Ok(Some(chunk)) => {
                    accumulated.push_str(&String::from_utf8_lossy(&chunk));
                    if accumulated.contains(needle) {
                        return Some(accumulated);
                    }
                }
                Ok(None) | Err(_) => return None,
            }
        }
    }
    None
}

fn make_template_cfg(
    tmp: &std::path::Path,
    tier_file: std::path::PathBuf,
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
    let doc_paths = HashMap::new();
    accelerator_visualiser::config::Config {
        plugin_root: tmp.to_path_buf(),
        plugin_version: "test".into(),
        project_root: tmp.to_path_buf(),
        tmp_path: tmp.to_path_buf(),
        host: "127.0.0.1".into(),
        owner_pid: 0,
        owner_start_time: None,
        log_path: tmp.join("server.log"),
        doc_paths,
        templates,
        work_item: None,
        kanban_columns: None,
        idle_timeout: None,
        editor: None,
        editor_project: None,
    }
}

#[tokio::test]
async fn template_file_mutation_arrives_as_template_changed_sse_event() {
    let tmp = tempfile::tempdir().unwrap();
    let tier_dir = tmp.path().join("templates");
    std::fs::create_dir_all(&tier_dir).unwrap();
    let tier_file = tier_dir.join("adr.md");
    std::fs::write(&tier_file, "v1").unwrap();

    let cfg = make_template_cfg(tmp.path(), tier_file.clone());
    let info_path = tmp.path().join("server-info.json");
    let port = start_server_with_template(cfg, info_path).await;

    let url = format!("http://127.0.0.1:{port}/api/events");
    let client = reqwest::Client::new();
    let mut sse_response =
        client.get(&url).send().await.expect("GET /api/events");
    assert_eq!(sse_response.status(), 200);

    let chunk = write_until_event(
        &mut sse_response,
        &tier_file,
        "v2",
        "\"type\":\"template-changed\"",
    )
    .await
    .expect("expected template-changed SSE event");

    // The chunk is an SSE frame: "data: {json}\n\n". Find and parse.
    let json_str = chunk
        .lines()
        .find(|l| l.contains("\"type\":\"template-changed\""))
        .and_then(|l| l.strip_prefix("data: "))
        .expect("expected data: line with template-changed payload");
    let payload: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(payload["template"], "adr");
    let expected =
        format!("sha256-{}", hex::encode(sha2::Sha256::digest(b"v2")));
    assert_eq!(payload["sha256"], expected);
}

#[tokio::test]
async fn template_file_emptied_arrives_as_template_changed_with_no_sha256() {
    let tmp = tempfile::tempdir().unwrap();
    let tier_dir = tmp.path().join("templates");
    std::fs::create_dir_all(&tier_dir).unwrap();
    let tier_file = tier_dir.join("adr.md");
    std::fs::write(&tier_file, "initial").unwrap();

    let cfg = make_template_cfg(tmp.path(), tier_file.clone());
    let info_path = tmp.path().join("server-info.json");
    let port = start_server_with_template(cfg, info_path).await;

    let url = format!("http://127.0.0.1:{port}/api/events");
    let client = reqwest::Client::new();
    let mut sse_response =
        client.get(&url).send().await.expect("GET /api/events");
    assert_eq!(sse_response.status(), 200);

    let chunk = write_until_event(
        &mut sse_response,
        &tier_file,
        "",
        "\"type\":\"template-changed\"",
    )
    .await
    .expect("expected template-changed SSE event");

    let json_str = chunk
        .lines()
        .find(|l| l.contains("\"type\":\"template-changed\""))
        .and_then(|l| l.strip_prefix("data: "))
        .expect("expected data: line with template-changed payload");
    let payload: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(payload["template"], "adr");
    assert!(
        payload.get("sha256").is_none(),
        "expected no sha256 key on empty-content transition, got: {payload}",
    );
}
