//! Pins the closed outcome keyword set the skills branch on.

#[path = "../src/keywords.rs"]
mod keywords;

use keywords::Init;
use keywords::Search;
use keywords::Show;

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
            "init: verified,listed,discovered",
        ]
    );
}
