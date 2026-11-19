#![allow(clippy::expect_used, clippy::panic)]

use shortcut_client::story_type_for_kind;
use shortcut_client::NewEntity;
use shortcut_client::StoryType;
use shortcut_client::UnknownKind;

#[test]
fn each_local_kind_creates_its_shortcut_entity() {
    for (kind, entity) in [
        ("story", NewEntity::Story(StoryType::Feature)),
        ("bug", NewEntity::Story(StoryType::Bug)),
        ("task", NewEntity::Story(StoryType::Chore)),
        ("spike", NewEntity::Story(StoryType::Chore)),
        ("epic", NewEntity::Epic),
        ("", NewEntity::Story(StoryType::Feature)),
    ] {
        assert_eq!(NewEntity::for_kind(kind), Ok(entity), "{kind:?}");
    }
}

#[test]
fn an_unknown_kind_creates_nothing() {
    assert_eq!(
        NewEntity::for_kind("initiative"),
        Err(UnknownKind("initiative".to_owned()))
    );
}

#[test]
fn each_entity_names_the_type_a_preview_shows() {
    assert_eq!(NewEntity::Story(StoryType::Feature).type_name(), "feature");
    assert_eq!(NewEntity::Story(StoryType::Bug).type_name(), "bug");
    assert_eq!(NewEntity::Story(StoryType::Chore).type_name(), "chore");
    assert_eq!(NewEntity::Epic.type_name(), "epic");
}

#[test]
fn a_search_filter_maps_local_kinds_to_story_types() {
    assert_eq!(story_type_for_kind("story"), Some(StoryType::Feature));
    assert_eq!(story_type_for_kind("bug"), Some(StoryType::Bug));
    assert_eq!(story_type_for_kind("task"), Some(StoryType::Chore));
    assert_eq!(story_type_for_kind("spike"), Some(StoryType::Chore));
}

#[test]
fn a_search_filter_finds_no_story_type_for_epics_or_unknown_kinds() {
    for kind in ["epic", "", "initiative"] {
        assert_eq!(story_type_for_kind(kind), None, "{kind:?}");
    }
}

#[test]
fn a_story_type_parses_from_shortcuts_own_name() {
    for (name, story_type) in [
        ("feature", StoryType::Feature),
        ("bug", StoryType::Bug),
        ("chore", StoryType::Chore),
    ] {
        assert_eq!(StoryType::named(name), Some(story_type));
        assert_eq!(story_type.as_str(), name);
    }
    assert_eq!(StoryType::named("epic"), None);
}
