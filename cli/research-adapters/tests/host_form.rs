#![allow(clippy::expect_used)]

use std::time::Duration;

use network::Host;
use reqwest::Url;
use research_adapters::host_form::admitted_host_matches;
use research_adapters::host_form::ascii_host;
use research_adapters::host_resolver::resolver_opts;

fn host(raw: &str) -> Host {
    Host::canonicalise(raw).expect("a host")
}

fn url(raw: &str) -> Url {
    Url::parse(raw).expect("a URL")
}

#[test]
fn a_unicode_name_takes_its_punycode_form() {
    assert_eq!(
        ascii_host(&host("bücher.example")).as_deref(),
        Some("xn--bcher-kva.example")
    );
}

#[test]
fn an_ascii_name_is_unchanged() {
    assert_eq!(
        ascii_host(&host("fixture.accelerator.test")).as_deref(),
        Some("fixture.accelerator.test")
    );
}

#[test]
fn a_name_the_url_parser_refuses_has_no_ascii_form() {
    for refused in ["a<b.example", "exa mple.org", "a.1"] {
        assert_eq!(ascii_host(&host(refused)), None, "{refused}");
    }
}

#[test]
fn a_name_that_folds_into_an_address_has_no_ascii_form() {
    for folding in ["１２７.０.０.１", "０ｘ７ｆ０００００１"] {
        let name = host(folding);
        assert_eq!(name.address(), None, "{folding} was admitted as a name");
        assert_eq!(ascii_host(&name), None, "{folding}");
    }
}

#[test]
fn an_address_is_its_own_ascii_form() {
    assert_eq!(ascii_host(&host("[::1]")).as_deref(), Some("::1"));
    assert_eq!(ascii_host(&host("127.0.0.1")).as_deref(), Some("127.0.0.1"));
}

#[test]
fn the_host_the_url_parser_reads_must_be_the_one_admitted() {
    for (raw, admitted) in [
        ("http://example.org/", "example.org"),
        ("http://xn--bcher-kva.example/", "bücher.example"),
        ("http://127.0.0.1:8080/", "127.0.0.1"),
        ("http://[::1]:8080/", "[::1]"),
        ("http://[2001:db8::1]/", "[2001:DB8:0::1]"),
    ] {
        assert!(admitted_host_matches(&url(raw), &host(admitted)), "{raw}");
    }
}

#[test]
fn a_host_the_url_parser_reads_differently_is_refused() {
    for (raw, admitted) in [
        ("http://127.0.0.1/", "localhost"),
        ("http://example.org/", "93.184.216.34"),
        ("http://xn--bcher-kva.example/", "bucher.example"),
        ("http://example.org./", "example.org"),
        ("http://[2001:db8::2]/", "[2001:db8::1]"),
    ] {
        assert!(!admitted_host_matches(&url(raw), &host(admitted)), "{raw}");
    }
}

#[test]
fn a_lookup_is_tried_once_within_its_bound() {
    let within = Duration::from_millis(1_500);
    let opts = resolver_opts(within);
    assert_eq!(opts.timeout, within);
    assert_eq!(opts.attempts, 1);
}
