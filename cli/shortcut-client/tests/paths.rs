#![allow(clippy::expect_used, clippy::panic)]

use shortcut_client::paths::url_is_allowed;
use shortcut_client::paths::Paths;
use shortcut_client::paths::WorkspaceSlug;
use shortcut_client::paths::PRODUCTION_BASE;
use shortcut_client::Collection;
use shortcut_client::ShortcutKey;
use shortcut_client::Url;

fn paths() -> Paths {
    Paths::new(
        Url::parse(PRODUCTION_BASE).expect("the production base parses"),
        WorkspaceSlug::parse("acme").expect("a valid slug"),
    )
}

#[test]
fn the_production_host_is_admitted() {
    assert!(url_is_allowed("https://api.app.shortcut.com", false));
    assert!(url_is_allowed("https://api.app.shortcut.com/", false));
}

#[test]
fn lookalike_and_weakened_urls_are_refused() {
    for url in [
        "https://api.app.shortcut.com.evil.io",
        "https://xapi.app.shortcut.com",
        "https://evil.io/api.app.shortcut.com",
        "http://api.app.shortcut.com",
        "https://user@api.app.shortcut.com",
        "https://user:pass@api.app.shortcut.com",
        "https://api.app.shortcut.com:8443",
        "https://api.app.shortcut.com/api/v4",
        "https://api.app.shortcut.com/?token=x",
        "not a url",
    ] {
        assert!(!url_is_allowed(url, false), "{url} must be refused");
        assert!(!url_is_allowed(url, true), "{url} must be refused");
    }
}

#[test]
fn loopback_is_admitted_only_when_allowed() {
    for url in ["http://127.0.0.1:9000", "http://localhost:9000"] {
        assert!(!url_is_allowed(url, false), "{url}");
        assert!(url_is_allowed(url, true), "{url}");
    }
}

#[test]
fn a_slug_of_lowercase_letters_digits_and_hyphens_is_valid() {
    for slug in ["acme", "acme-co", "a1", "9lives"] {
        assert!(WorkspaceSlug::parse(slug).is_ok(), "{slug}");
    }
}

#[test]
fn a_slug_that_could_redirect_a_request_is_refused() {
    for slug in [
        "", "-acme", "Acme", "ac/me", "ac?me", "ac#me", "..", "ac me",
    ] {
        assert!(WorkspaceSlug::parse(slug).is_err(), "{slug:?}");
    }
}

#[test]
fn whoami_sits_outside_the_workspace() {
    assert_eq!(
        Paths::whoami(&Url::parse(PRODUCTION_BASE).expect("parses")).as_str(),
        "https://api.app.shortcut.com/api/v4/whoami"
    );
}

#[test]
fn an_entity_path_follows_its_key() {
    let paths = paths();
    let story = ShortcutKey::parse("sc-12").expect("parses");
    let epic = ShortcutKey::parse("epic-3").expect("parses");

    assert_eq!(
        paths.entity(story).as_str(),
        "https://api.app.shortcut.com/api/v4/acme/stories/12"
    );
    assert_eq!(
        paths.entity(epic).as_str(),
        "https://api.app.shortcut.com/api/v4/acme/epics/3"
    );
    assert_eq!(
        paths.comments(story).as_str(),
        "https://api.app.shortcut.com/api/v4/acme/stories/12/comments"
    );
}

#[test]
fn collection_and_catalogue_paths_sit_under_the_workspace() {
    let paths = paths();

    assert_eq!(
        paths.collection(Collection::Stories).as_str(),
        "https://api.app.shortcut.com/api/v4/acme/stories"
    );
    assert_eq!(
        paths.search_stories().as_str(),
        "https://api.app.shortcut.com/api/v4/acme/stories/search"
    );
    assert_eq!(
        paths.teams().as_str(),
        "https://api.app.shortcut.com/api/v4/acme/teams"
    );
    assert_eq!(
        paths.team("8f1c2d3e-0000-4000-8000-000000000001").as_str(),
        "https://api.app.shortcut.com/api/v4/acme/teams/\
         8f1c2d3e-0000-4000-8000-000000000001"
    );
    assert_eq!(
        paths.workflows().as_str(),
        "https://api.app.shortcut.com/api/v4/acme/workflows"
    );
    assert_eq!(
        paths.team_workflows("t-1").as_str(),
        "https://api.app.shortcut.com/api/v4/acme/teams/t-1/workflows"
    );
    assert_eq!(
        paths.workflow_states(500).as_str(),
        "https://api.app.shortcut.com/api/v4/acme/workflows/500/states"
    );
    assert_eq!(
        paths.members().as_str(),
        "https://api.app.shortcut.com/api/v4/acme/members"
    );
}

#[test]
fn a_segment_a_url_would_split_stays_one_segment() {
    let url = paths().team("a/b?c#d");

    assert_eq!(url.path_segments().expect("segments").count(), 5);
    assert!(url.query().is_none() && url.fragment().is_none(), "{url}");
}

#[test]
fn a_loopback_base_keeps_its_port() {
    let paths = Paths::new(
        Url::parse("http://127.0.0.1:9000").expect("parses"),
        WorkspaceSlug::parse("acme").expect("valid"),
    );

    assert_eq!(
        paths.teams().as_str(),
        "http://127.0.0.1:9000/api/v4/acme/teams"
    );
}

#[test]
fn a_next_page_url_must_share_the_origin_and_the_api_prefix() {
    let paths = paths();

    assert!(paths.admits_next_page(
        "https://api.app.shortcut.com/api/v4/acme/stories?cursor=abc"
    ));
    for next in [
        "https://evil.io/api/v4/acme/stories?cursor=abc",
        "http://api.app.shortcut.com/api/v4/acme/stories?cursor=abc",
        "https://api.app.shortcut.com:8443/api/v4/acme/stories",
        "https://api.app.shortcut.com/api/v3/stories",
        "https://api.app.shortcut.com/api/v4x/stories",
        "/api/v4/acme/stories?cursor=abc",
    ] {
        assert!(!paths.admits_next_page(next), "{next}");
    }
}
