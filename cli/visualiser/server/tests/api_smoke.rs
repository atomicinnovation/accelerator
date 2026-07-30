use std::os::unix::fs::symlink;
use std::process::Stdio;
use std::time::Duration;

use tokio::process::Command;

#[tokio::test]
async fn api_surface_is_fully_reachable_against_fixture_meta() {
    // Model-1 fixture project: symlink meta/ + templates/ to the committed
    // fixtures and remap only the one path that differs from the catalogue
    // default (research_codebase → meta/research). The server discovers this
    // root from its cwd and reads config directly.
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/meta");
    let plugin_templates = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/templates");
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path();
    std::fs::create_dir_all(project.join(".accelerator")).unwrap();
    symlink(&fixtures, project.join("meta")).unwrap();
    symlink(&plugin_templates, project.join("templates")).unwrap();
    std::fs::write(
        project.join(".accelerator/config.md"),
        "---\npaths:\n  research_codebase: meta/research\n---\n",
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_accelerator-visualiser");
    let mut child = Command::new(bin)
        .args(["serve", "--owner-pid", "0"])
        .current_dir(project)
        .env("ACCELERATOR_PLUGIN_ROOT", project)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();

    let info_path =
        project.join(".accelerator/tmp/visualiser/server-info.json");
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

    // /api/templates -> one entry per fixture plugin template (Model-1 composes
    // the whole plugin templates/ set, not a hand-picked subset).
    let tpl: serde_json::Value = client
        .get(format!("{base}/api/templates"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let expected_templates = std::fs::read_dir(&plugin_templates)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .ok()
                .and_then(|e| e.path().extension().map(|x| x == "md"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        tpl["templates"].as_array().unwrap().len(),
        expected_templates
    );

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
