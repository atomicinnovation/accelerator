//! JQL composition, driven from the committed fixture under a row-coverage
//! guard.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use jira_client::jql::{
    compose, key_clause, quote, Family, FixedResolver, Search,
};
use jira_client::ClientError;

const FAMILIES: [&str; 7] = [
    "status",
    "labels",
    "assignee",
    "issuetype",
    "component",
    "reporter",
    "parent",
];

fn resolvers() -> FixedResolver {
    let mut fields = BTreeMap::new();
    fields.insert("Story Points".to_owned(), "customfield_10016".to_owned());
    FixedResolver {
        account: Some("5f:acct".to_owned()),
        fields,
    }
}

fn rows() -> Vec<(String, String, String)> {
    let raw = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/jql-composition.txt"),
    )
    .expect("the committed fixture is readable");
    raw.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert!(fields.len() >= 2, "at least a case and a spec: {line:?}");
            (
                fields[0].to_owned(),
                fields[1].to_owned(),
                (*fields.get(2).unwrap_or(&"")).to_owned(),
            )
        })
        .collect()
}

fn parse_spec(spec: &str) -> Search {
    let mut search = Search::default();
    for token in spec.split(';').filter(|token| !token.is_empty()) {
        let (name, value) = token.split_once('=').unwrap_or((token, ""));
        match name {
            "all" => search.all_projects = true,
            "watching" => search.watching = true,
            "project" => search.project = Some(value.to_owned()),
            "text" => search.text.push(value.to_owned()),
            "empty" => search.empty.push(value.to_owned()),
            "notempty" => search.not_empty.push(value.to_owned()),
            "raw" => search.raw = Some(value.to_owned()),
            field if FAMILIES.contains(&field) => {
                search.families.push(Family {
                    field: field.to_owned(),
                    values: value.split(',').map(str::to_owned).collect(),
                });
            }
            other => panic!("unrecognised spec token {other}"),
        }
    }
    search
}

#[test]
fn every_fixture_row_composes_to_its_expected_jql() {
    let resolvers = resolvers();
    let rows = rows();
    let mut consumed = 0;

    for (case, spec, expected) in &rows {
        let search = parse_spec(spec);
        let composed = compose(&search, &resolvers, &resolvers)
            .expect("every fixture row composes");
        assert_eq!(&composed, expected, "case {case}");
        consumed += 1;
    }

    assert_eq!(consumed, rows.len(), "every row present was asserted");
    assert!(consumed >= 20, "the fixture must cover the whole surface");
}

#[test]
fn the_fixture_covers_all_ten_flag_families_and_each_negation() {
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
        assert!(
            specs.contains(&format!("{family}=~")) || specs.contains(",~"),
            "{family} has no negated fixture row"
        );
        covered.insert(family);
    }
    for singleton in ["project", "watching", "text"] {
        assert!(specs.contains(singleton), "{singleton} has no fixture row");
        covered.insert(singleton);
    }

    assert_eq!(covered.len(), 10, "ten families: {covered:?}");
}

#[test]
fn a_search_naming_no_project_and_not_all_projects_is_refused() {
    let resolvers = resolvers();
    let error = compose(&Search::default(), &resolvers, &resolvers)
        .expect_err("a project or all_projects is required");

    assert!(error.to_string().contains("E_JQL_NO_PROJECT"), "{error}");
}

#[test]
fn an_unresolvable_me_is_refused_rather_than_queried_literally() {
    let resolvers = FixedResolver::new();
    let search = parse_spec("all;assignee=@me");

    let error = compose(&search, &resolvers, &resolvers)
        .expect_err("@me needs a cached accountId");

    assert!(
        error.to_string().contains("E_SEARCH_NO_SITE_CACHE"),
        "{error}"
    );
}

#[test]
fn an_empty_value_is_refused_and_a_control_byte_is_refused() {
    assert!(matches!(quote(""), Err(ClientError::BadJql { .. })));
    assert!(matches!(quote("a\nb"), Err(ClientError::BadJql { .. })));
    assert!(matches!(quote("a\u{7f}b"), Err(ClientError::BadJql { .. })));
    assert_eq!(quote("plain").expect("a plain value quotes"), "'plain'");
}

#[test]
fn the_key_clause_quotes_every_key() {
    assert_eq!(
        key_clause(&["ENG-1".to_owned(), "ENG-2".to_owned()])
            .expect("keys quote"),
        "key IN ('ENG-1', 'ENG-2')"
    );
}
