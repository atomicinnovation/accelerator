//! End-to-end test: file mutation → SSE event via the full server.

use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn file_mutation_arrives_as_sse_event() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("meta/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(plans.join("2026-01-01-test.md"), "---\ntitle: T\n---\n").unwrap();

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
        if start.elapsed().as_secs() > 5 {
            panic!("server-info.json did not appear in 5s");
        }
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

    // Give the watcher time to register with the OS.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Mutate a watched file.
    std::fs::write(plans.join("2026-01-01-test.md"), "---\ntitle: Updated\n---\n").unwrap();

    // Read SSE frames until we see "doc-changed" or time out.
    // 2000ms deadline: 100ms debounce + OS notification latency + reqwest
    // round-trip, with generous headroom for slow CI runners.
    let deadline = tokio::time::Instant::now() + Duration::from_millis(2000);
    let mut found = false;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(300), sse_response.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                let text = std::str::from_utf8(&chunk).unwrap_or("");
                if text.contains("doc-changed") {
                    found = true;
                    break;
                }
            }
            Ok(Ok(None)) => break,
            Ok(Err(e)) => panic!("reqwest error reading SSE stream: {e}"),
            Err(_) => break,
        }
    }
    assert!(found, "expected doc-changed SSE event within 2000ms");
}
