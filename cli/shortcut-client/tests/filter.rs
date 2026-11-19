#![allow(clippy::expect_used, clippy::panic)]

use shortcut_client::filter::SearchFilters;

fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(field, value)| ((*field).to_owned(), (*value).to_owned()))
        .collect()
}

fn queries(values: &[(&str, &str)], team: &str) -> Vec<String> {
    SearchFilters::parse(&pairs(values))
        .expect("parses")
        .queries(team)
        .expect("lowers")
}

#[test]
fn no_filters_search_the_team_alone() {
    assert_eq!(queries(&[], "Team 1"), vec!["team:\"Team 1\""]);
}

#[test]
fn each_filter_lowers_to_its_operator() {
    assert_eq!(
        queries(
            &[("label", "ux"), ("state", "In Progress"), ("owner", "ada"), ("type", "bug")],
            "Team 1"
        ),
        vec!["team:\"Team 1\" label:\"ux\" state:\"In Progress\" owner:ada type:bug"]
    );
}

#[test]
fn values_for_one_field_become_one_search_each() {
    let mut lowered = queries(&[("label", "a"), ("label", "b")], "T");
    lowered.sort();

    assert_eq!(
        lowered,
        vec!["team:\"T\" label:\"a\"", "team:\"T\" label:\"b\""]
    );
}

#[test]
fn two_fields_with_two_values_each_make_four_searches() {
    let lowered = queries(
        &[
            ("label", "a"),
            ("label", "b"),
            ("state", "x"),
            ("state", "y"),
        ],
        "T",
    );

    assert_eq!(lowered.len(), 4);
    for query in &lowered {
        assert_eq!(query.matches("label:").count(), 1, "{query}");
        assert_eq!(query.matches("state:").count(), 1, "{query}");
    }
}

#[test]
fn a_filter_needing_more_than_ten_searches_is_refused() {
    let many: Vec<(&str, &str)> =
        ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"]
            .iter()
            .map(|label| ("label", *label))
            .collect();

    let error = SearchFilters::parse(&pairs(&many))
        .expect("parses")
        .queries("T")
        .expect_err("refused");

    assert!(error.to_string().contains("11"), "{error}");
}

#[test]
fn a_negated_value_lowers_to_a_negated_operator_in_every_search() {
    assert_eq!(
        queries(&[("label", "~wontfix"), ("label", "~dupe")], "T"),
        vec!["team:\"T\" -label:\"wontfix\" -label:\"dupe\""]
    );
}

#[test]
fn a_local_kind_lowers_to_its_story_type() {
    assert_eq!(
        queries(&[("type", "task")], "T"),
        vec!["team:\"T\" type:chore"]
    );
    assert_eq!(
        queries(&[("type", "feature")], "T"),
        vec!["team:\"T\" type:feature"]
    );
}

#[test]
fn an_epic_or_unknown_type_is_refused_naming_the_accepted_values() {
    for value in ["epic", "initiative"] {
        let error = SearchFilters::parse(&pairs(&[("type", value)]))
            .expect_err("refused");
        let message = error.to_string();
        assert!(message.contains("feature"), "{message}");
        assert!(message.contains("task"), "{message}");
    }
}

#[test]
fn a_value_holding_a_quote_or_backslash_is_refused() {
    for value in ["a\"b", "a\\b"] {
        assert!(
            SearchFilters::parse(&pairs(&[("label", value)])).is_err(),
            "{value}"
        );
    }
}

#[test]
fn a_value_cannot_add_a_second_operator() {
    let lowered = queries(&[("label", "x label:y")], "T");

    assert_eq!(lowered, vec!["team:\"T\" label:\"x label:y\""]);
}

#[test]
fn an_owner_must_be_a_mention_name() {
    assert!(SearchFilters::parse(&pairs(&[("owner", "ada lovelace")])).is_err());
}

#[test]
fn an_unknown_field_is_refused() {
    let error = SearchFilters::parse(&pairs(&[("colour", "red")]))
        .expect_err("refused");

    assert!(error.to_string().contains("colour"), "{error}");
}

#[test]
fn a_team_name_holding_a_quote_is_refused_naming_the_team() {
    let error = SearchFilters::parse(&[])
        .expect("parses")
        .queries("Ops \"Night\"")
        .expect_err("refused");

    assert!(error.to_string().contains("Ops"), "{error}");
    assert!(error.to_string().contains("rename"), "{error}");
}

#[test]
fn the_filtered_states_are_listed_for_validation() {
    let filters =
        SearchFilters::parse(&pairs(&[("state", "Ready"), ("state", "~Done")]))
            .expect("parses");

    assert_eq!(filters.states(), vec!["Ready", "Done"]);
}

#[test]
fn free_text_lowers_to_a_quoted_phrase() {
    let lowered = queries(&[("text", "login fails")], "T");

    assert_eq!(lowered, vec!["team:\"T\" \"login fails\""]);
}
