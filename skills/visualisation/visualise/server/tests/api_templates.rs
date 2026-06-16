use std::collections::HashMap;
use std::sync::Arc;

use visualiser::activity::Activity;
use visualiser::config::TemplateTiers;
use visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sha2::Digest;
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn templates_list_returns_all_configured_templates() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["templates"].as_array().unwrap();
    assert!(arr.iter().any(|s| s["name"] == "adr"));
}

#[tokio::test]
async fn template_detail_returns_three_tiers_with_plugin_default_active() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/adr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["name"], "adr");
    let tiers = v["tiers"].as_array().unwrap();
    assert_eq!(tiers.len(), 3);
    let active: Vec<&serde_json::Value> =
        tiers.iter().filter(|t| t["active"] == true).collect();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0]["source"], "plugin-default");
}

#[tokio::test]
async fn template_detail_includes_sha256_of_winning_content() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/adr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let sha = v["sha256"].as_str().expect("sha256 must be present");
    assert!(sha.starts_with("sha256-"), "must be sha256-prefixed: {sha}");
    let hex_part = &sha["sha256-".len()..];
    assert_eq!(hex_part.len(), 64, "hex digest must be 64 chars: {sha}");
    assert!(
        hex_part
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "must be lowercase hex only: {sha}",
    );
    let tiers = v["tiers"].as_array().unwrap();
    let active = tiers.iter().find(|t| t["active"] == true).unwrap();
    let content = active["content"].as_str().unwrap();
    let expected = format!(
        "sha256-{}",
        hex::encode(sha2::Sha256::digest(content.as_bytes())),
    );
    assert_eq!(sha, expected);
}

#[tokio::test]
async fn template_detail_omits_sha256_when_winning_content_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    // Replace the adr template's plugin-default with an empty file so the
    // winning tier exists but resolves to empty content.
    let empty = tmp.path().join("plugin-templates/adr.md");
    std::fs::write(&empty, "").unwrap();
    let mut templates: HashMap<String, TemplateTiers> = HashMap::new();
    templates.insert(
        "adr".to_string(),
        TemplateTiers {
            config_override: None,
            user_override: tmp.path().join("meta/templates/adr.md"),
            plugin_default: empty,
            config_override_source: None,
        },
    );
    cfg.templates = templates;
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/adr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let raw = std::str::from_utf8(&bytes).unwrap();
    assert!(!raw.contains("\"sha256\":null"), "raw: {raw}");
    assert!(!raw.contains("\"sha256\":\"\""), "raw: {raw}");
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("sha256").is_none(), "expected sha256 absent: {v}");
}

#[tokio::test]
async fn template_detail_omits_sha256_when_winning_tier_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    // Point all tiers at non-existent files.
    let mut templates: HashMap<String, TemplateTiers> = HashMap::new();
    templates.insert(
        "adr".to_string(),
        TemplateTiers {
            config_override: None,
            user_override: tmp.path().join("missing-user.md"),
            plugin_default: tmp.path().join("missing-plugin.md"),
            config_override_source: None,
        },
    );
    cfg.templates = templates;
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/adr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("sha256").is_none());
}

#[tokio::test]
async fn template_detail_omits_sha256_when_winning_content_not_utf8() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    let non_utf8 = tmp.path().join("plugin-templates/adr.md");
    std::fs::write(&non_utf8, [0xFFu8, 0xFE, 0x00, 0xFF]).unwrap();
    let mut templates: HashMap<String, TemplateTiers> = HashMap::new();
    templates.insert(
        "adr".to_string(),
        TemplateTiers {
            config_override: None,
            user_override: tmp.path().join("meta/templates/adr.md"),
            plugin_default: non_utf8,
            config_override_source: None,
        },
    );
    cfg.templates = templates;
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/adr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v.get("sha256").is_none());
}

#[tokio::test]
async fn template_detail_for_unknown_name_returns_404() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = common::seeded_cfg(tmp.path());
    let activity = Arc::new(Activity::new());
    let state = AppState::build(cfg, activity).await.unwrap();
    let app = build_router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/templates/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
