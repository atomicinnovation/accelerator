use std::sync::Arc;

use accelerator_visualiser::activity::Activity;
use accelerator_visualiser::server::{build_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use corpus::doc_type::{DocType, Listing};
use corpus::set::{
    MemberKind, MemberSlot, Presentation, SetDeclaration, SetShape,
};
use http_body_util::BodyExt;
use tower::ServiceExt;

mod common;

static NOTEBOOKS: DocType = DocType {
    wire_token: "notebooks",
    config_path_key: Some("notebooks"),
    label: "Notebooks",
    ..DocType::FLAT_DEFAULTS
};

static SCRAPBOOKS: DocType = DocType {
    wire_token: "scrapbooks",
    config_path_key: Some("scrapbooks"),
    label: "Scrapbooks",
    listing: Listing::Set,
    ..DocType::FLAT_DEFAULTS
};

static SCRAPBOOK: SetDeclaration = SetDeclaration {
    shape: SetShape {
        root: "cover.md",
        member_slots: &[MemberSlot {
            pattern: "clippings/*.jpg",
            kind: MemberKind::Asset,
        }],
    },
    presentation: Presentation::SetPage,
};

async fn served_with_notebooks() -> (tempfile::TempDir, Arc<AppState>) {
    let tmp = tempfile::tempdir().unwrap();
    let notebooks = tmp.path().join("meta/notebooks");
    std::fs::create_dir_all(&notebooks).unwrap();
    std::fs::write(
        notebooks.join("field-observations.md"),
        "---\ntitle: \"Field observations\"\n---\n# body\n",
    )
    .unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    cfg.doc_paths.insert("notebooks".into(), notebooks);
    cfg.doc_types = Some(common::catalogue_with(&[&NOTEBOOKS], &[]));
    let state = AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .unwrap();
    (tmp, state)
}

async fn get_json(state: Arc<AppState>, uri: &str) -> serde_json::Value {
    let res = build_router(state)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "expected 200 for {uri}");
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn a_declared_doc_type_is_listed_among_the_types() {
    let (_tmp, state) = served_with_notebooks().await;

    let v = get_json(state, "/api/types").await;

    let notebooks = v["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["key"] == "notebooks")
        .expect("notebooks listed");
    assert_eq!(notebooks["label"], "Notebooks");
    assert_eq!(notebooks["count"], 1);
}

#[tokio::test]
async fn a_declared_doc_type_lists_its_documents() {
    let (_tmp, state) = served_with_notebooks().await;

    let v = get_json(state, "/api/docs?type=notebooks").await;

    let docs = v["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["title"], "Field observations");
}

#[tokio::test]
async fn a_declared_doc_type_s_documents_are_found_by_title() {
    let (_tmp, state) = served_with_notebooks().await;

    let v = get_json(state, "/api/search?q=field%20observations").await;

    let results = v["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "got {results:?}");
    assert_eq!(results[0]["docType"], "notebooks");
    assert_eq!(results[0]["slug"], "field-observations");
}

#[tokio::test]
async fn a_config_carrying_no_catalogue_serves_the_assembled_one() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    cfg.doc_types = None;

    let state = AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .unwrap();

    assert_eq!(
        state.doc_types.doc_types(),
        declarations::catalogue().unwrap().doc_types()
    );
}

#[tokio::test]
async fn a_declared_set_type_lists_its_sets_and_describes_its_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let scrapbooks = tmp.path().join("meta/scrapbooks");
    let holiday = scrapbooks.join("holiday");
    std::fs::create_dir_all(&holiday).unwrap();
    std::fs::write(
        holiday.join("cover.md"),
        "---\ntitle: \"Holiday\"\n---\n# body\n",
    )
    .unwrap();
    let mut cfg = common::seeded_cfg(tmp.path());
    cfg.doc_paths.insert("scrapbooks".into(), scrapbooks);
    cfg.doc_types = Some(common::catalogue_with(
        &[&SCRAPBOOKS],
        &[(corpus::DocTypeKey::declared(&SCRAPBOOKS), &SCRAPBOOK)],
    ));
    let state = AppState::build(cfg, Arc::new(Activity::new()))
        .await
        .unwrap();

    let docs = get_json(state.clone(), "/api/docs?type=scrapbooks").await;
    let types = get_json(state, "/api/types").await;

    let docs = docs["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["title"], "Holiday");
    let scrapbooks = types["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["key"] == "scrapbooks")
        .expect("scrapbooks listed");
    assert_eq!(
        scrapbooks["set"],
        serde_json::json!({
            "root": "cover.md",
            "documentSlots": [],
            "assetSlots": ["clippings/*.jpg"],
            "presentation": "set-page"
        })
    );
}
