//! Pins the closed outcome keyword set the skills branch on.

#[path = "../src/keywords.rs"]
mod keywords;

use keywords::Attach;
use keywords::Comment;
use keywords::Create;
use keywords::Init;
use keywords::Search;
use keywords::Show;
use keywords::Transition;
use keywords::Update;

#[test]
fn the_keyword_set_is_exactly_the_committed_surface() {
    let surface = [
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
                Search::CapHit.keyword(),
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
        [
            "show: found,not-found",
            "search: results,empty,truncated,cap-hit",
            "create: created,writeback-failed",
            "update: updated",
            "comment: added",
            "transition: transitioned",
            "attach: attached",
            "init: verified,listed,discovered",
        ]
    );
}
