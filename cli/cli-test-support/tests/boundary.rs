//! The shared machinery's own boundary tests: a parser or loader bug would
//! otherwise silently weaken every downstream parity, keyword and request
//! assertion in both provider binaries at once.

#![allow(clippy::expect_used)]

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;

use cli_test_support::{parse_u8_consts, Scenario};
use http_test_support::{MockServer, RequestKey};

/// A raw POST over TCP, returning `(status, body)` — no HTTP client, so the
/// test needs no TLS crypto provider for a plain-http mock.
fn post(server: &MockServer, path: &str, body: &[u8]) -> (u16, Vec<u8>) {
    let address = server.base_url().replace("http://", "");
    let mut stream = TcpStream::connect(address).expect("connect");
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: mock\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).expect("head");
    stream.write_all(body).expect("body");
    stream.flush().expect("flush");

    let mut reader = BufReader::new(stream);
    let mut status_line = String::new();
    reader.read_line(&mut status_line).expect("status");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .expect("status code")
        .parse()
        .expect("numeric status");
    let mut length = 0;
    let mut header = String::new();
    loop {
        header.clear();
        if reader.read_line(&mut header).expect("header") == 0
            || header == "\r\n"
            || header == "\n"
        {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut response = vec![0_u8; length];
    if length > 0 {
        reader.read_exact(&mut response).expect("response body");
    }
    (status, response)
}

#[test]
fn the_parser_reads_u8_consts_and_skips_everything_else() {
    let source = "\
//! a doc line, not a const
pub const CLEAN: u8 = 0;
    pub const SEARCH_BAD_FLAG: u8 = 75;
pub const NOT_A_U8: u16 = 9;
pub const MISSING_VALUE: u8 =;
const PRIVATE: u8 = 3;
";
    let parsed = parse_u8_consts(source);

    assert_eq!(
        parsed,
        vec![("CLEAN".to_owned(), 0), ("SEARCH_BAD_FLAG".to_owned(), 75)],
        "only well-formed public u8 consts are read, in file order"
    );
}

#[test]
fn the_loader_maps_a_lone_expectation_to_a_single_route() {
    let scenario = Scenario::from_json(
        r#"{"expectations":[{"method":"post","path":"/graphql",
            "response":{"status":200,"headers":{"Content-Type":"application/json"},
            "body":"{\"ok\":true}"},"expect_body_contains":"query"}]}"#,
    )
    .expect("valid scenario");

    let server = MockServer::start();
    scenario.install(&server);

    let (status, body) = post(&server, "/graphql", b"{\"query\":\"x\"}");
    assert_eq!(status, 200);
    assert_eq!(body, b"{\"ok\":true}");

    assert_eq!(
        scenario.body_expectations(),
        vec![(RequestKey::post("/graphql"), "query".to_owned())]
    );
}

#[test]
fn the_loader_groups_a_consume_sequence_in_declaration_order() {
    let scenario = Scenario::from_json(
        r#"{"expectations":[
            {"method":"POST","path":"/graphql","consume":true,
             "response":{"status":200,"headers":{},"body":"first"}},
            {"method":"POST","path":"/graphql","consume":true,
             "response":{"status":200,"headers":{},"body":"second"}}
        ]}"#,
    )
    .expect("valid scenario");

    let server = MockServer::start();
    scenario.install(&server);

    assert_eq!(post(&server, "/graphql", b"a").1, b"first");
    assert_eq!(
        post(&server, "/graphql", b"b").1,
        b"second",
        "the second hit gets the second response, not a repeat of the first"
    );
}
