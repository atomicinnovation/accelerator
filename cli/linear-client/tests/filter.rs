//! `IssueFilter` composition, driven from the committed fixture under a
//! row-coverage guard.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use linear_client::filter::{compose, FixedStates, Search, FETCH_PAGE_SIZE};
use linear_client::ClientError;

const FAMILIES: [&str; 5] = ["team", "state", "assignee", "label", "text"];

fn states() -> FixedStates {
    let mut map = BTreeMap::new();
    map.insert("In Progress".to_owned(), "state-uuid".to_owned());
    FixedStates(map)
}

fn rows() -> Vec<(String, String, String)> {
    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/issue-filter.txt"),
    )
    .expect("the committed fixture is readable");
    raw.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 3, "three columns: {line:?}");
            (
                fields[0].to_owned(),
                fields[1].to_owned(),
                fields[2].to_owned(),
            )
        })
        .collect()
}

fn parse_spec(spec: &str) -> Search {
    let mut search = Search::default();
    for token in spec.split(';').filter(|token| !token.is_empty()) {
        let (name, value) = token
            .split_once('=')
            .unwrap_or_else(|| panic!("a spec token is name=value: {token}"));
        let value = value.to_owned();
        match name {
            "team" => search.team_id = Some(value),
            "state" => search.state = Some(value),
            "assignee" => search.assignee = Some(value),
            "label" => search.label = Some(value),
            "text" => search.text = Some(value),
            other => panic!("unrecognised spec token {other}"),
        }
    }
    search
}

#[test]
fn every_fixture_row_composes_to_its_expected_filter() {
    let states = states();
    let rows = rows();
    let mut consumed = 0;

    for (case, spec, expected) in &rows {
        let composed = compose(&parse_spec(spec), &states)
            .expect("every fixture row composes");
        assert_eq!(
            serde_json::to_string(&composed).expect("it serialises"),
            *expected,
            "case {case}"
        );
        consumed += 1;
    }

    assert_eq!(consumed, rows.len(), "every row present was asserted");
}

#[test]
fn the_fixture_covers_every_family() {
    let specs: String = rows()
        .into_iter()
        .map(|(_, spec, _)| spec)
        .collect::<Vec<_>>()
        .join(";");
    let mut covered = BTreeSet::new();

    for family in FAMILIES {
        assert!(
            specs.contains(&format!("{family}=")),
            "{family} has no fixture row"
        );
        covered.insert(family);
    }

    assert_eq!(covered.len(), 5, "five families: {covered:?}");
}

#[test]
fn an_unknown_state_is_refused_rather_than_filtered_literally() {
    let search = Search {
        state: Some("Nonexistent".to_owned()),
        ..Search::default()
    };

    let error = compose(&search, &states())
        .expect_err("an unresolvable state name is a refusal");

    assert!(matches!(error, ClientError::UnknownState { .. }));
    assert!(error.to_string().contains("Nonexistent"), "{error}");
}

#[test]
fn the_fetch_page_size_is_linears_bulk_ceiling() {
    assert_eq!(FETCH_PAGE_SIZE, 250);
}
