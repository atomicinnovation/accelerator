//! Attachment upload: the multipart content type and `X-Atlassian-Token`
//! header, one part per file, and the file-safety refusals.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use std::fs;
use std::path::Path;

use http_test_support::{MockServer, RequestKey, Route};
use jira_client::SurfaceError;
use support::client::client_for;
use tempfile::TempDir;
use tracker_support::TransportConfig;

const KEY: &str = "ENG-1";
const ATTACH: &str = "/rest/api/3/issue/ENG-1/attachments";

fn write(root: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = root.join(name);
    fs::write(&path, bytes).expect("the fixture file is written");
    path
}

#[test]
fn attach_sends_multipart_with_the_no_check_token_and_one_part_per_file() {
    let root = TempDir::new().expect("a temp root");
    let one = write(root.path(), "a.txt", b"alpha");
    let two = write(root.path(), "b.txt", b"beta");

    let server = MockServer::start();
    server.route(
        RequestKey::post(ATTACH),
        Route::Json {
            status: 200,
            body: "[]".to_owned(),
        },
    );
    let client = client_for(&server, TransportConfig::default());

    client
        .attach(KEY, &[one.as_path(), two.as_path()], root.path())
        .expect("the upload succeeds");

    let key = RequestKey::post(ATTACH);
    assert_eq!(
        server.last_header(&key, "x-atlassian-token").as_deref(),
        Some("no-check")
    );
    let content_type = server
        .last_header(&key, "content-type")
        .expect("a content type");
    assert!(
        content_type.starts_with("multipart/form-data; boundary="),
        "{content_type}"
    );
    let body = server.last_body(&key).expect("a body");
    let text = String::from_utf8_lossy(&body);
    assert_eq!(text.matches("Content-Disposition").count(), 2);
}

#[test]
fn a_missing_file_is_refused() {
    let root = TempDir::new().expect("a temp root");
    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let missing = root.path().join("absent.txt");
    let error = client
        .attach(KEY, &[missing.as_path()], root.path())
        .expect_err("refused");

    assert!(matches!(error, SurfaceError::FileRefused { .. }));
    assert_eq!(server.hits(&RequestKey::post(ATTACH)), 0);
}

#[test]
fn a_path_outside_the_root_is_refused() {
    let root = TempDir::new().expect("a temp root");
    let outside_dir = TempDir::new().expect("a second temp dir");
    let outside = write(outside_dir.path(), "secret.txt", b"leak");

    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach(KEY, &[outside.as_path()], root.path())
        .expect_err("refused");

    assert!(matches!(error, SurfaceError::PathEscapesRoot { .. }));
    assert_eq!(server.hits(&RequestKey::post(ATTACH)), 0);
}

#[test]
fn a_directory_is_refused_by_the_handle_check() {
    let root = TempDir::new().expect("a temp root");
    let dir = root.path().join("subdir");
    fs::create_dir(&dir).expect("the directory is created");

    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach(KEY, &[dir.as_path()], root.path())
        .expect_err("refused");

    assert!(matches!(error, SurfaceError::FileRefused { .. }));
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_device_is_refused() {
    let root = TempDir::new().expect("a temp root");
    let link = root.path().join("dev-link");
    std::os::unix::fs::symlink("/dev/null", &link)
        .expect("the symlink is created");

    let server = MockServer::start();
    let client = client_for(&server, TransportConfig::default());

    let error = client
        .attach(KEY, &[link.as_path()], root.path())
        .expect_err("refused");

    // /dev/null resolves outside the root, so confinement refuses it before
    // the handle check even sees the character device.
    assert!(matches!(
        error,
        SurfaceError::PathEscapesRoot { .. } | SurfaceError::FileRefused { .. }
    ));
    assert_eq!(server.hits(&RequestKey::post(ATTACH)), 0);
}
