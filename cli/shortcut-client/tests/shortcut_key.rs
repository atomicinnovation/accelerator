#![allow(clippy::expect_used, clippy::panic)]

use shortcut_client::Collection;
use shortcut_client::EpicId;
use shortcut_client::NotAStory;
use shortcut_client::ShortcutKey;
use shortcut_client::StoryId;

fn parsed(text: &str) -> ShortcutKey {
    ShortcutKey::parse(text).expect("a well-formed key parses")
}

#[test]
fn a_story_key_parses_in_either_case() {
    assert_eq!(parsed("sc-1234"), parsed("SC-1234"));
    assert_eq!(parsed("sc-1234"), ShortcutKey::Story(StoryId::new(1234)));
}

#[test]
fn an_epic_key_parses_in_either_case() {
    assert_eq!(parsed("epic-56"), parsed("EPIC-56"));
    assert_eq!(parsed("epic-56"), ShortcutKey::Epic(EpicId::new(56)));
}

#[test]
fn a_story_and_an_epic_with_one_number_are_different_keys() {
    assert_ne!(parsed("sc-56"), parsed("epic-56"));
}

#[test]
fn malformed_keys_are_refused() {
    for text in [
        "1234", "sc-", "sc-12a", "ENG-12", "sc-0", "sc-012", "epic-0", "sc--1",
        "-12", "sc-1 2", "", "sc_12",
    ] {
        assert!(
            ShortcutKey::parse(text).is_err(),
            "{text:?} must be refused"
        );
    }
}

#[test]
fn a_number_beyond_the_integer_range_is_refused() {
    assert!(ShortcutKey::parse("sc-99999999999999999999999").is_err());
}

#[test]
fn keys_render_lowercase() {
    assert_eq!(parsed("SC-1234").to_string(), "sc-1234");
    assert_eq!(parsed("EPIC-56").to_string(), "epic-56");
}

#[test]
fn each_key_names_its_collection() {
    assert_eq!(parsed("sc-1").collection(), Collection::Stories);
    assert_eq!(parsed("epic-1").collection(), Collection::Epics);
    assert_eq!(Collection::Stories.path_segment(), "stories");
    assert_eq!(Collection::Epics.path_segment(), "epics");
}

#[test]
fn each_key_exposes_the_number_the_api_takes() {
    assert_eq!(parsed("sc-1234").number(), 1234);
    assert_eq!(parsed("epic-56").number(), 56);
}

#[test]
fn a_story_key_converts_into_a_story_id() {
    assert_eq!(parsed("sc-7").into_story(), Ok(StoryId::new(7)));
}

#[test]
fn an_epic_key_is_not_a_story() {
    assert_eq!(
        parsed("epic-7").into_story(),
        Err(NotAStory(EpicId::new(7)))
    );
}

#[test]
fn every_rendered_key_passes_the_shared_issue_key_rule() {
    for key in [parsed("sc-1"), parsed("sc-98765"), parsed("epic-3")] {
        let rendered = key.to_string();
        assert!(
            tracker_support::issue_key_is_safe(&rendered).is_ok(),
            "{rendered} must pass issue_key_is_safe"
        );
    }
}
