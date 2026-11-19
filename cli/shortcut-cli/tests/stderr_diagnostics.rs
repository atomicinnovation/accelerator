//! Pins the `E_*` names the binary writes to stderr for refusals made before
//! any request.
#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "support/mod.rs"]
mod support;

use support::Token;

fn stderr_of(args: &[&str], api_url: Option<&str>) -> String {
    let dir = support::scratch(support::CONFIG);
    support::stderr(&support::run_with(
        dir.path(),
        args,
        api_url,
        &Token::Present,
    ))
}

#[test]
fn refusals_before_any_request_name_their_error() {
    let cases: [(&str, &[&str], Option<&str>); 3] = [
        (
            "E_BAD_API_URL:",
            &["show", "sc-1"],
            Some("https://evil.example.com"),
        ),
        ("E_SHOW_BAD_REFERENCE:", &["show", "ENG-1"], None),
        ("E_NO_CATALOGUE:", &["show", "sc-1"], None),
    ];
    for (name, args, api_url) in cases {
        let stderr = stderr_of(args, api_url);
        assert!(stderr.starts_with(name), "{args:?}: {stderr}");
    }
}
