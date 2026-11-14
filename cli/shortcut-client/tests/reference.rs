#![allow(clippy::expect_used, clippy::panic)]

use shortcut_client::paths::WorkspaceSlug;
use shortcut_client::reference::ReferenceRefusal;
use shortcut_client::reference::ShortcutReference;
use shortcut_client::EpicId;
use shortcut_client::ShortcutKey;
use shortcut_client::StoryId;

fn slug() -> WorkspaceSlug {
    WorkspaceSlug::parse("scratch-workspace").expect("a safe slug")
}

fn resolved(text: &str) -> Result<ShortcutKey, ReferenceRefusal> {
    ShortcutReference::parse(text)
        .and_then(|reference| reference.key_in(&slug()))
}

#[test]
fn a_key_resolves_to_itself() {
    assert_eq!(resolved("sc-14"), Ok(ShortcutKey::Story(StoryId::new(14))));
    assert_eq!(resolved("EPIC-17"), Ok(ShortcutKey::Epic(EpicId::new(17))));
}

#[test]
fn a_story_url_resolves_to_its_story_key() {
    for url in [
        "https://app.shortcut.com/scratch-workspace/story/14",
        "https://app.shortcut.com/scratch-workspace/story/14/contract-story",
        "https://app.shortcut.com/scratch-workspace/story/14/contract-story#activity-15",
    ] {
        assert_eq!(
            resolved(url),
            Ok(ShortcutKey::Story(StoryId::new(14))),
            "{url}"
        );
    }
}

#[test]
fn an_epic_url_resolves_to_its_epic_key() {
    assert_eq!(
        resolved(
            "https://app.shortcut.com/scratch-workspace/epic/17#activity-18"
        ),
        Ok(ShortcutKey::Epic(EpicId::new(17)))
    );
}

#[test]
fn a_url_for_another_workspace_is_refused_naming_both_workspaces() {
    let refusal = resolved("https://app.shortcut.com/elsewhere/story/14")
        .expect_err("another workspace is refused");

    assert_eq!(
        refusal,
        ReferenceRefusal::ForeignWorkspace {
            found: "elsewhere".to_owned(),
            configured: "scratch-workspace".to_owned(),
        }
    );
    let message = refusal.to_string();
    assert!(message.contains("elsewhere"), "{message}");
    assert!(message.contains("scratch-workspace"), "{message}");
}

#[test]
fn text_that_is_neither_a_key_nor_an_entity_url_is_refused() {
    for text in [
        "14",
        "ENG-14",
        "http://app.shortcut.com/scratch-workspace/story/14",
        "https://evil.example.com/scratch-workspace/story/14",
        "https://app.shortcut.com/scratch-workspace/team/14",
        "https://app.shortcut.com/scratch-workspace/story/014",
        "https://app.shortcut.com/scratch-workspace/story/",
        "https://app.shortcut.com/scratch-workspace",
    ] {
        assert!(
            matches!(resolved(text), Err(ReferenceRefusal::Unrecognised(_))),
            "{text:?} must be refused"
        );
    }
}
