#![allow(clippy::expect_used, clippy::panic)]

use shortcut_client::team_key::KeyLookup;
use shortcut_client::team_key::TeamIdentity;
use shortcut_client::team_key::TeamKey;
use shortcut_client::team_key::TeamKeys;

fn team(uuid: &str, mention_name: Option<&str>, name: &str) -> TeamIdentity {
    TeamIdentity {
        uuid: uuid.to_owned(),
        mention_name: mention_name.map(str::to_owned),
        name: name.to_owned(),
    }
}

#[test]
fn a_teams_key_is_its_mention_name() {
    assert_eq!(
        TeamKey::of(Some("mobile"), "Mobile Team").as_str(),
        "mobile"
    );
}

#[test]
fn a_team_without_a_mention_name_is_keyed_by_its_display_name() {
    assert_eq!(TeamKey::of(None, "Mobile Team").as_str(), "Mobile Team");
    assert_eq!(TeamKey::of(Some(""), "Mobile Team").as_str(), "Mobile Team");
}

#[test]
fn a_key_resolves_to_its_teams_uuid() {
    let keys = TeamKeys::over(&[
        team("u-mobile", Some("mobile"), "Mobile Team"),
        team("u-web", None, "Web"),
    ]);

    assert_eq!(keys.lookup("mobile"), KeyLookup::Found("u-mobile"));
    assert_eq!(keys.lookup("Web"), KeyLookup::Found("u-web"));
}

#[test]
fn a_display_name_is_not_a_key_for_a_team_with_a_mention_name() {
    let keys =
        TeamKeys::over(&[team("u-mobile", Some("mobile"), "Mobile Team")]);

    assert_eq!(keys.lookup("Mobile Team"), KeyLookup::Unknown);
}

#[test]
fn two_null_mention_teams_with_one_display_name_collide() {
    let keys =
        TeamKeys::over(&[team("u-1", None, "Ops"), team("u-2", None, "Ops")]);

    assert_eq!(keys.lookup("Ops"), KeyLookup::Colliding);
}

#[test]
fn a_display_name_that_matches_another_teams_mention_name_collides() {
    let keys = TeamKeys::over(&[
        team("u-1", Some("ops"), "Operations"),
        team("u-2", None, "ops"),
    ]);

    assert_eq!(keys.lookup("ops"), KeyLookup::Colliding);
}

#[test]
fn a_colliding_team_is_made_visible_under_its_uuid() {
    let keys = TeamKeys::over(&[
        team("u-1", None, "Ops"),
        team("u-2", None, "Ops"),
        team("u-3", Some("web"), "Web"),
    ]);

    assert_eq!(keys.visible_key("u-1"), Some("u-1".to_owned()));
    assert_eq!(keys.visible_key("u-2"), Some("u-2".to_owned()));
    assert_eq!(keys.visible_key("u-3"), Some("web".to_owned()));
    assert_eq!(keys.visible_key("u-missing"), None);
}

#[test]
fn the_known_keys_list_every_usable_key_in_order() {
    let keys = TeamKeys::over(&[
        team("u-1", Some("web"), "Web"),
        team("u-2", None, "Ops"),
        team("u-3", None, "Ops"),
        team("u-4", Some("api"), "API"),
    ]);

    assert_eq!(keys.known(), vec!["api", "web"]);
}
