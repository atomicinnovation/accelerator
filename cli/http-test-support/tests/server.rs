//! The behaviours these tests pin down: `(method, path)` keying, request
//! recording, and the stall response every read-timeout test depends on.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::time::Duration;

use http_test_support::{MockServer, RequestKey, Route, UNMATCHED_STATUS};

struct Response {
    status: u16,
    body: Vec<u8>,
}

fn connect(server: &MockServer) -> TcpStream {
    let address = server.base_url().replace("http://", "");
    TcpStream::connect(address).expect("connect")
}

fn send(
    stream: &mut TcpStream,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    let mut bytes = format!(
        "{method} {target} HTTP/1.1\r\n\
         Host: mock\r\n\
         Content-Length: {length}\r\n",
        length = body.len()
    )
    .into_bytes();
    for (name, value) in headers {
        bytes.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(body);
    stream.write_all(&bytes).expect("write request");
    stream.flush().expect("flush request");
}

fn read_head(reader: &mut BufReader<TcpStream>) -> (u16, usize) {
    let mut status_line = String::new();
    reader.read_line(&mut status_line).expect("status line");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .expect("status code")
        .parse()
        .expect("numeric status");
    let mut content_length = 0;
    let mut header = String::new();
    loop {
        header.clear();
        let read = reader.read_line(&mut header).expect("header");
        if read == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    (status, content_length)
}

fn request(
    server: &MockServer,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Response {
    let mut stream = connect(server);
    send(&mut stream, method, target, headers, body);
    let mut reader = BufReader::new(stream);
    let (status, content_length) = read_head(&mut reader);
    let mut received = vec![0_u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut received).expect("body");
    }
    Response {
        status,
        body: received,
    }
}

#[test]
fn a_request_matching_no_route_returns_the_unmatched_status() {
    let server = MockServer::start();
    server.route(RequestKey::get("/present"), Route::Status(204));

    let response = request(&server, "GET", "/absent", &[], &[]);

    assert_eq!(response.status, UNMATCHED_STATUS);
}

#[test]
fn a_route_registered_for_one_method_does_not_answer_another() {
    let server = MockServer::start();
    server.route(RequestKey::get("/issue"), Route::Status(200));

    let response = request(&server, "POST", "/issue", &[], b"{}");

    assert_eq!(response.status, UNMATCHED_STATUS);
    assert_eq!(server.hits(&RequestKey::get("/issue")), 0);
    assert_eq!(server.hits(&RequestKey::post("/issue")), 1);
}

#[test]
fn hits_count_exact_key_matches_only() {
    let server = MockServer::start();
    server.route(RequestKey::get("/a"), Route::Status(200));
    server.route(RequestKey::get("/b"), Route::Status(200));

    request(&server, "GET", "/a", &[], &[]);
    request(&server, "GET", "/a", &[], &[]);
    request(&server, "GET", "/b", &[], &[]);

    assert_eq!(server.hits(&RequestKey::get("/a")), 2);
    assert_eq!(server.hits(&RequestKey::get("/b")), 1);
}

#[test]
fn the_accessors_report_nothing_for_a_key_never_requested() {
    let server = MockServer::start();
    server.route(RequestKey::post("/issue"), Route::Status(200));
    request(&server, "POST", "/other", &[], b"body");

    let never = RequestKey::post("/issue");

    assert_eq!(server.last_body(&never), None);
    assert_eq!(server.last_query(&never), None);
    assert_eq!(server.last_header(&never, "content-type"), None);
}

#[test]
fn the_recorded_request_is_visible_as_soon_as_the_call_returns() {
    let server = MockServer::start();
    let key = RequestKey::post("/rest/api/3/search/jql");
    server.route(
        key.clone(),
        Route::Json {
            status: 200,
            body: "{}".to_owned(),
        },
    );

    request(
        &server,
        "POST",
        "/rest/api/3/search/jql?expand=names",
        &[("Content-Type", "application/json")],
        b"{\"jql\":\"key in (ABC-1)\"}",
    );

    assert_eq!(
        server.last_body(&key),
        Some(b"{\"jql\":\"key in (ABC-1)\"}".to_vec())
    );
    assert_eq!(server.last_query(&key), Some("expand=names".to_owned()));
    assert_eq!(
        server.last_header(&key, "Content-Type"),
        Some("application/json".to_owned())
    );
}

#[test]
fn headers_are_recorded_per_route_rather_than_globally() {
    let server = MockServer::start();
    let authorised = RequestKey::post("/graphql");
    let anonymous = RequestKey::put("/upload");
    server.route(
        authorised.clone(),
        Route::Json {
            status: 200,
            body: "{}".to_owned(),
        },
    );
    server.route(anonymous.clone(), Route::Status(200));

    request(
        &server,
        "POST",
        "/graphql",
        &[("Authorization", "Bearer secret")],
        b"{}",
    );
    request(&server, "PUT", "/upload", &[], b"bytes");

    assert_eq!(
        server.last_header(&authorised, "authorization"),
        Some("Bearer secret".to_owned())
    );
    assert_eq!(server.last_header(&anonymous, "authorization"), None);
}

#[test]
fn a_json_route_returns_its_body_verbatim() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/repos/owner/repo"),
        Route::Json {
            status: 404,
            body: "{\"message\":\"Not Found\"}".to_owned(),
        },
    );

    let response = request(&server, "GET", "/repos/owner/repo", &[], &[]);

    assert_eq!(response.status, 404);
    assert_eq!(response.body, b"{\"message\":\"Not Found\"}");
}

#[test]
fn a_flaky_route_fails_the_stated_number_of_times_then_succeeds() {
    let server = MockServer::start();
    let key = RequestKey::get("/asset");
    server.route(
        key.clone(),
        Route::FlakyThenOk {
            fail_times: 2,
            body: b"payload".to_vec(),
        },
    );

    assert_eq!(request(&server, "GET", "/asset", &[], &[]).status, 500);
    assert_eq!(request(&server, "GET", "/asset", &[], &[]).status, 500);
    let succeeded = request(&server, "GET", "/asset", &[], &[]);

    assert_eq!(succeeded.status, 200);
    assert_eq!(succeeded.body, b"payload");
    assert_eq!(server.hits(&key), 3);
}

#[test]
fn a_stalled_route_sends_its_headers_then_withholds_the_body() {
    let server = MockServer::start();
    server.route(
        RequestKey::get("/slow"),
        Route::Stall(Duration::from_secs(30)),
    );

    let mut stream = connect(&server);
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .expect("read timeout");
    send(&mut stream, "GET", "/slow", &[], &[]);
    let mut reader = BufReader::new(stream);
    let (status, content_length) = read_head(&mut reader);

    assert_eq!(status, 200);
    assert_eq!(content_length, 16);

    let mut body = vec![0_u8; content_length];
    let outcome = reader.read_exact(&mut body);

    assert!(outcome.is_err(), "the promised body must never arrive");
}

#[test]
fn a_sequenced_route_answers_each_hit_in_turn() {
    let server = MockServer::start();
    let key = RequestKey::post("/graphql");
    server.route(
        key.clone(),
        Route::Sequence(vec![
            Route::Json {
                status: 200,
                body: "{\"first\":true}".to_owned(),
            },
            Route::Json {
                status: 200,
                body: "{\"second\":true}".to_owned(),
            },
            Route::Status(503),
        ]),
    );

    let first = request(&server, "POST", "/graphql", &[], b"{}");
    let second = request(&server, "POST", "/graphql", &[], b"{}");
    let third = request(&server, "POST", "/graphql", &[], b"{}");
    let fourth = request(&server, "POST", "/graphql", &[], b"{}");

    assert_eq!(first.body, b"{\"first\":true}");
    assert_eq!(second.body, b"{\"second\":true}");
    assert_eq!(third.status, 503);
    assert_eq!(
        fourth.status, 503,
        "the last entry repeats rather than becoming an unmatched key"
    );
    assert_eq!(server.hits(&key), 4);
}
