use std::process::Stdio;
use std::time::Duration;

use serde_json::json;
use tokio::process::Command;

#[tokio::test]
async fn api_surface_is_fully_reachable_against_fixture_meta() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/meta");
    let plugin_templates = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/templates");
    let tmp = tempfile::tempdir().unwrap();
    let cfg_path = tmp.path().join("config.json");
    let tmp_dir = tmp.path().join("visualiser");
    std::fs::create_dir_all(&tmp_dir).unwrap();

    let mut doc_paths = serde_json::Map::new();
    for (key, rel) in [
        ("decisions", "decisions"),
        ("work", "work"),
        ("plans", "plans"),
        ("research", "research"),
        ("review_plans", "reviews/plans"),
        ("review_prs", "reviews/prs"),
        ("validations", "validations"),
        ("notes", "notes"),
        ("prs", "prs"),
        ("research_issues", "research/issues"),
    ] {
        doc_paths.insert(key.into(), json!(fixtures.join(rel)));
    }
    let mut templates = serde_json::Map::new();
    for name in ["adr", "plan", "research", "validation", "pr-description"] {
        templates.insert(
            name.into(),
            json!({
                "config_override": null,
                "user_override": fixtures.join(format!("templates/{name}.md")),
                "plugin_default": plugin_templates.join(format!("{name}.md")),
            }),
        );
    }
    let cfg = json!({
        "plugin_root": fixtures.parent().unwrap(),
        "plugin_version": "0.0.0-smoke",
        "project_root": fixtures.parent().unwrap(),
        "tmp_path": tmp_dir,
        "host": "127.0.0.1",
        "owner_pid": 0,
        "owner_start_time": null,
        "log_path": tmp_dir.join("server.log"),
        "doc_paths": doc_paths,
        "templates": templates,
    });
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&cfg).unwrap())
        .unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = Command::new(bin)
        .arg("--config")
        .arg(&cfg_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();

    let info_path = tmp_dir.join("server-info.json");
    let start = std::time::Instant::now();
    loop {
        if info_path.exists() {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill().await;
            panic!("server-info.json did not appear in 30s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let info: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&info_path).unwrap()).unwrap();
    let base = info["url"]
        .as_str()
        .unwrap()
        .trim_end_matches('/')
        .to_string();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();

    // server-info.json is written once the listener is bound (run() in
    // server.rs) but a few awaited steps before `axum::serve` starts its
    // accept loop. The kernel queues the connection on the bound socket, so
    // under full-`mise run` parallel load the first request can outlast a tight
    // per-request timeout while the serve task waits to be scheduled. Probe
    // /api/types until the server is actually serving, with generous headroom
    // for slow CI runners, then reuse the result for the first assertion.
    let probe_deadline = std::time::Instant::now() + Duration::from_secs(30);
    let t: serde_json::Value = loop {
        match client.get(format!("{base}/api/types")).send().await {
            Ok(resp) => break resp.json().await.unwrap(),
            Err(e) => {
                if std::time::Instant::now() >= probe_deadline {
                    let _ = child.kill().await;
                    panic!("server never became reachable on {base}: {e}");
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    };
    assert_eq!(t["types"].as_array().unwrap().len(), 14);

    // /api/docs?type=decisions -> 3 entries.
    let d: serde_json::Value = client
        .get(format!("{base}/api/docs?type=decisions"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(d["docs"].as_array().unwrap().len(), 3);

    // /api/docs?type=plan-reviews -> 2 entries with expected slugs.
    let pr: serde_json::Value = client
        .get(format!("{base}/api/docs?type=plan-reviews"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let slugs: Vec<&str> = pr["docs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["slug"].as_str().unwrap())
        .collect();
    assert!(slugs.contains(&"first-plan"));
    assert!(slugs.contains(&"example-and-review-some-topic"));

    // /api/templates -> 5 entries.
    let tpl: serde_json::Value = client
        .get(format!("{base}/api/templates"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tpl["templates"].as_array().unwrap().len(), 5);

    // /api/docs/{*path} with If-None-Match round-trip.
    let r1 = client
        .get(format!(
            "{base}/api/docs/meta/decisions/ADR-0001-example-decision.md"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    let etag = r1
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let r2 = client
        .get(format!(
            "{base}/api/docs/meta/decisions/ADR-0001-example-decision.md"
        ))
        .header("if-none-match", &etag)
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 304);

    // /api/lifecycle returns a non-empty cluster list.
    let lc: serde_json::Value = client
        .get(format!("{base}/api/lifecycle"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!lc["clusters"].as_array().unwrap().is_empty());

    let _ = child.kill().await;
}
