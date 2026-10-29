#![allow(clippy::expect_used)]

use network::HostError;
use research::sources::web::page_url::PageUrl;
use research::sources::web::page_url::PageUrlError;

fn parsed(raw: &str) -> PageUrl {
    PageUrl::parse(raw).expect("a fetchable URL")
}

fn displayed(raw: &str) -> String {
    parsed(raw).to_string()
}

fn followed(from: &str, location: &str) -> Result<String, PageUrlError> {
    parsed(from).follow(location).map(|url| url.to_string())
}

#[test]
fn only_http_and_https_are_fetched() {
    for raw in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "ftp://example.org/",
        "example.org/a",
    ] {
        assert_eq!(PageUrl::parse(raw), Err(PageUrlError::Scheme), "{raw}");
    }
}

#[test]
fn the_scheme_and_host_are_read_whatever_their_case() {
    assert_eq!(displayed("HTTPS://Example.org/a"), "https://example.org/a");
}

#[test]
fn a_url_carrying_user_information_is_refused() {
    assert_eq!(
        PageUrl::parse("http://user@host/"),
        Err(PageUrlError::Host(HostError::Userinfo))
    );
}

#[test]
fn a_numerically_encoded_host_is_refused() {
    assert_eq!(
        PageUrl::parse("http://0x7f000001/"),
        Err(PageUrlError::Host(HostError::NumericEncoding))
    );
}

#[test]
fn a_url_without_a_host_is_refused() {
    for raw in ["http:///x", "http:example.org"] {
        assert_eq!(
            PageUrl::parse(raw),
            Err(PageUrlError::Host(HostError::Empty)),
            "{raw}"
        );
    }
}

#[test]
fn an_explicit_port_is_kept_and_an_absent_or_empty_one_is_the_default() {
    assert_eq!(parsed("http://example.org:8080/").port(), 8080);
    assert_eq!(parsed("http://example.org/").port(), 80);
    assert_eq!(parsed("https://example.org/").port(), 443);
    assert_eq!(parsed("http://example.org:/x").port(), 80);
    assert_eq!(displayed("http://example.org:/x"), "http://example.org/x");
    assert_eq!(
        displayed("https://example.org:443/"),
        "https://example.org/"
    );
}

#[test]
fn a_port_outside_one_to_65535_is_refused() {
    assert_eq!(
        PageUrl::parse("http://example.org:99999/"),
        Err(PageUrlError::Port)
    );
}

#[test]
fn an_ipv6_host_is_bracketed_when_displayed() {
    assert_eq!(
        displayed("http://[2001:db8::1]:8080/"),
        "http://[2001:db8::1]:8080/"
    );
    assert_eq!(
        displayed("https://[2001:db8::1]/"),
        "https://[2001:db8::1]/"
    );
    assert_eq!(
        followed("http://[2001:db8::1]:8080/", "/x"),
        Ok("http://[2001:db8::1]:8080/x".to_owned())
    );
}

#[test]
fn whitespace_control_characters_and_backslashes_are_refused_anywhere() {
    for raw in [
        "http://example.org/a b",
        "http://example.org/a\tb",
        "http://example.org/a\u{7f}",
        "http://127.0.0.1\\x.example/",
    ] {
        assert_eq!(
            PageUrl::parse(raw),
            Err(PageUrlError::Character),
            "{raw:?}"
        );
    }
}

#[test]
fn a_unicode_host_is_kept_as_written_in_lower_case() {
    let url = parsed("http://Bücher.example/");
    assert_eq!(url.host().to_string(), "bücher.example");
    assert_eq!(url.to_string(), "http://bücher.example/");
}

#[test]
fn a_fragment_is_dropped() {
    assert_eq!(
        displayed("http://example.org/a#frag"),
        "http://example.org/a"
    );
}

#[test]
fn the_authority_ends_at_a_query() {
    let url = parsed("http://example.org?x");
    assert_eq!(url.host().to_string(), "example.org");
    assert_eq!(url.to_string(), "http://example.org/?x");
}

#[test]
fn a_displayed_url_parses_back_to_itself() {
    for raw in [
        "https://example.org/a/b?q=1",
        "http://[2001:db8::1]:8080/",
        "http://bücher.example/",
        "http://example.org:8080/?x",
    ] {
        let url = parsed(raw);
        assert_eq!(PageUrl::parse(&url.to_string()), Ok(url), "{raw}");
    }
}

#[test]
fn each_rule_names_itself_without_echoing_the_input() {
    let cases = [
        (
            "gopher://secret.example/",
            "only http and https are fetched",
        ),
        (
            "http://secret.example/a b",
            "the URL contains whitespace, a control character or a backslash",
        ),
        (
            "http://user@secret.example/",
            "the URL carries user information",
        ),
        ("http:///secret", "the URL has no host"),
        (
            "http://secret%01.example/",
            "the host contains a control character",
        ),
        (
            "http://0x7f000001/secret",
            "the host is a numerically encoded address",
        ),
        (
            "http://secret.example:99999/",
            "the port is not a number from 1 to 65535",
        ),
    ];
    for (raw, rule) in cases {
        let error = PageUrl::parse(raw).expect_err("a refusal");
        assert_eq!(error.to_string(), rule, "{raw}");
        assert!(!error.to_string().contains("secret"), "{raw}");
    }
}

#[test]
fn a_location_resolves_against_the_current_url() {
    let from = "http://example.org/a/b/c?q=1";
    for (location, expected) in [
        ("https://other.example/x", "https://other.example/x"),
        ("//other.example/x", "http://other.example/x"),
        ("/x", "http://example.org/x"),
        ("/x/./y/../z", "http://example.org/x/z"),
        ("x", "http://example.org/a/b/x"),
        ("../x", "http://example.org/a/x"),
        ("./x?y=1", "http://example.org/a/b/x?y=1"),
        ("../../../../x", "http://example.org/x"),
        ("..", "http://example.org/a/"),
        ("?q=2", "http://example.org/a/b/c?q=2"),
        ("#s1", "http://example.org/a/b/c?q=1"),
        ("", "http://example.org/a/b/c?q=1"),
    ] {
        assert_eq!(
            followed(from, location),
            Ok(expected.to_owned()),
            "{location}"
        );
    }
}

#[test]
fn a_query_only_location_replaces_only_the_query() {
    assert_eq!(
        followed("http://example.org/a/b?q=1", "?q=2"),
        Ok("http://example.org/a/b?q=2".to_owned())
    );
}

#[test]
fn a_location_the_rules_refuse_is_refused_by_follow() {
    for (location, error) in [
        ("file:///etc/passwd", PageUrlError::Scheme),
        ("javascript:alert(1)", PageUrlError::Scheme),
        (
            "http://0x7f000001/",
            PageUrlError::Host(HostError::NumericEncoding),
        ),
        ("http://user@host/", PageUrlError::Host(HostError::Userinfo)),
        ("/a b", PageUrlError::Character),
    ] {
        assert_eq!(
            followed("http://example.org/", location),
            Err(error),
            "{location}"
        );
    }
}
