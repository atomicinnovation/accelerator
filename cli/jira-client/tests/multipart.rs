//! The hand-rolled multipart encoder's contract: the exact byte layout, the
//! filename refusal that stops header injection, and the boundary that is
//! always free of every part body.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use jira_client::multipart;
use jira_client::multipart::Part;
use jira_client::SurfaceError;

fn part(filename: &str, content_type: &'static str, bytes: &[u8]) -> Part {
    Part {
        filename: filename.to_owned(),
        content_type,
        bytes: bytes.to_vec(),
    }
}

#[test]
fn one_part_encodes_to_the_exact_expected_bytes() {
    let body =
        multipart::encode("BND", &[part("log.txt", "text/plain", b"hello")])
            .expect("the body encodes");

    let expected = concat!(
        "--BND\r\n",
        "Content-Disposition: form-data; name=\"file\"; ",
        "filename=\"log.txt\"\r\n",
        "Content-Type: text/plain\r\n",
        "\r\n",
        "hello\r\n",
        "--BND--\r\n",
    );
    assert_eq!(body, expected.as_bytes());
}

#[test]
fn two_files_produce_two_parts() {
    let body = multipart::encode(
        "BND",
        &[
            part("a.txt", "text/plain", b"a"),
            part("b.bin", "application/octet-stream", b"b"),
        ],
    )
    .expect("the body encodes");

    let text = String::from_utf8_lossy(&body);
    assert_eq!(text.matches("Content-Disposition").count(), 2);
    assert_eq!(text.matches("--BND\r\n").count(), 2);
    assert!(text.ends_with("--BND--\r\n"));
}

#[test]
fn a_filename_with_a_quote_is_refused_not_escaped() {
    let injection = "a\"\r\nContent-Type: text/html\r\n\r\n<script>";
    let error =
        multipart::encode("BND", &[part(injection, "text/plain", b"x")])
            .expect_err("refused");

    assert!(matches!(error, SurfaceError::BadFilename { .. }));
}

#[test]
fn a_filename_with_a_control_byte_is_refused() {
    for bad in ["with\ttab", "with\u{7f}del", ""] {
        let error = multipart::encode("BND", &[part(bad, "text/plain", b"x")])
            .expect_err("refused");
        assert!(matches!(error, SurfaceError::BadFilename { .. }), "{bad:?}");
    }
}

#[test]
fn the_generated_boundary_is_absent_from_every_part_body() {
    // A body that happens to contain a boundary-shaped run must not be picked.
    let bytes = b"----accelerator-embedded-boundary-here".to_vec();
    let parts = vec![Part {
        filename: "x".to_owned(),
        content_type: "text/plain",
        bytes,
    }];

    let boundary =
        multipart::boundary_free_of(&parts).expect("a free boundary");

    assert!(!parts[0]
        .bytes
        .windows(boundary.len())
        .any(|window| window == boundary.as_bytes()));
}
