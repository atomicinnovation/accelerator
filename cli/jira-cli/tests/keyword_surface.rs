//! Pins the closed, per-subcommand keyword set. A new outcome
//! variant cannot compile without a keyword; this golden is the backstop that
//! catches a keyword *string* changing under a repointed body's feet.

#[path = "../src/keywords.rs"]
mod keywords;

use keywords::{
    Comment, Create, Fields, Init, Search, Show, Transition, Update,
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
            vec![Search::Results.keyword(), Search::Empty.keyword()],
        ),
        ("create", vec![Create::Created.keyword()]),
        ("update", vec![Update::Updated.keyword()]),
        (
            "comment",
            vec![
                Comment::Added.keyword(),
                Comment::Listed.keyword(),
                Comment::Edited.keyword(),
                Comment::Deleted.keyword(),
            ],
        ),
        ("transition", vec![Transition::Transitioned.keyword()]),
        (
            "init",
            vec![
                Init::Verified.keyword(),
                Init::Discovered.keyword(),
                Init::ProjectsListed.keyword(),
                Init::FieldsListed.keyword(),
                Init::DefaultPrompted.keyword(),
            ],
        ),
        (
            "fields",
            vec![
                Fields::Refreshed.keyword(),
                Fields::Resolved.keyword(),
                Fields::Listed.keyword(),
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
            "search: results,empty",
            "create: created",
            "update: updated",
            "comment: added,listed,edited,deleted",
            "transition: transitioned",
            "init: verified,discovered,projects-listed,fields-listed,\
             default-prompted",
            "fields: refreshed,resolved,listed",
        ]
    );
}

#[test]
fn every_keyword_is_lower_kebab() {
    let all = [
        Show::Found.keyword(),
        Show::NotFound.keyword(),
        Search::Results.keyword(),
        Search::Empty.keyword(),
        Create::Created.keyword(),
        Update::Updated.keyword(),
        Comment::Added.keyword(),
        Comment::Listed.keyword(),
        Comment::Edited.keyword(),
        Comment::Deleted.keyword(),
        Transition::Transitioned.keyword(),
        Init::Verified.keyword(),
        Init::Discovered.keyword(),
        Init::ProjectsListed.keyword(),
        Init::FieldsListed.keyword(),
        Init::DefaultPrompted.keyword(),
        Fields::Refreshed.keyword(),
        Fields::Resolved.keyword(),
        Fields::Listed.keyword(),
    ];
    for keyword in all {
        assert!(
            keyword.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "keyword {keyword:?} is not lower-kebab"
        );
    }
}
