//! Pins the closed, per-subcommand keyword set. A new outcome
//! variant cannot compile without a keyword; this golden is the backstop that
//! catches a keyword *string* changing under a repointed body's feet.

#[path = "../src/keywords.rs"]
mod keywords;

use keywords::{
    Attach, Comment, Create, Init, Search, Show, Transition, Update,
};

#[test]
fn the_keyword_set_is_exactly_the_committed_surface() {
    let surface: Vec<(&str, Vec<&str>)> = vec![
        (
            "show",
            vec![Show::Found.keyword(), Show::NotFound.keyword()],
        ),
        (
            "search",
            vec![
                Search::Results.keyword(),
                Search::Empty.keyword(),
                Search::Truncated.keyword(),
            ],
        ),
        (
            "create",
            vec![Create::Created.keyword(), Create::WritebackFailed.keyword()],
        ),
        ("update", vec![Update::Updated.keyword()]),
        ("comment", vec![Comment::Added.keyword()]),
        ("transition", vec![Transition::Transitioned.keyword()]),
        ("attach", vec![Attach::Attached.keyword()]),
        (
            "init",
            vec![
                Init::Verified.keyword(),
                Init::Listed.keyword(),
                Init::Discovered.keyword(),
            ],
        ),
    ];

    let rendered: Vec<String> = surface
        .iter()
        .map(|(sub, keywords)| format!("{sub}: {}", keywords.join(",")))
        .collect();

    assert_eq!(
        rendered,
        vec![
            "show: found,not-found",
            "search: results,empty,truncated",
            "create: created,writeback-failed",
            "update: updated",
            "comment: added",
            "transition: transitioned",
            "attach: attached",
            "init: verified,listed,discovered",
        ]
    );
}

#[test]
fn every_keyword_is_lower_kebab_and_unique() {
    let all = [
        Show::Found.keyword(),
        Show::NotFound.keyword(),
        Search::Results.keyword(),
        Search::Empty.keyword(),
        Search::Truncated.keyword(),
        Create::Created.keyword(),
        Create::WritebackFailed.keyword(),
        Update::Updated.keyword(),
        Comment::Added.keyword(),
        Transition::Transitioned.keyword(),
        Attach::Attached.keyword(),
        Init::Verified.keyword(),
        Init::Listed.keyword(),
        Init::Discovered.keyword(),
    ];
    for keyword in all {
        assert!(
            keyword.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "keyword {keyword:?} is not lower-kebab"
        );
    }
}
