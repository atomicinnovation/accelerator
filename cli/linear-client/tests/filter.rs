//! The filter vocabulary and `IssueFilter` composition: the configured,
//! validated and lowerable stages, the refusals, and the committed fixture
//! under a row-coverage guard.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use linear_client::filter::{
    complete_for_teams, compose, preflight, ConfiguredSearch, FilterKey,
    UnresolvedFilter, UnresolvedFilters, ValidatedSearch, FETCH_PAGE_SIZE,
};
use linear_client::resolution::{
    FilterFamily, ResolverSet, TeamRef, Unresolved,
};
use serde_json::json;
use support::catalogue::{
    complete_entry, label, member, project, resolvers, state,
};
use tracker_support::pull::{validate, PullConfig, Tracker};

const FAMILIES: [&str; 7] = [
    "team", "state", "assignee", "label", "project", "text", "teams",
];

fn fixture_catalogue() -> ResolverSet {
    resolvers(&json!({
        "baseTeam": "T1",
        "labels": [],
        "teams": [
            complete_entry("T1", "ONE", &json!({
                "states": [state("s1-ip", "In Progress"),
                           state("s1-done", "Done")],
                "labels": [label("l1-infra", "infra"), label("l1-a", "a"),
                           label("l1-b", "b")],
                "members": [member("u-alice", "alice", "alice@x.io"),
                            member("u-bob", "bob", "bob@x.io")],
                "projects": [project("p-alpha", "Alpha"),
                             project("p-beta", "Beta")]
            })),
            complete_entry("T2", "TWO", &json!({
                "states": [state("s2-ip", "In Progress")],
                "labels": [label("l2-infra", "infra")],
                "members": [member("u-alice", "alice", "alice@x.io")],
                "projects": [project("p-alpha", "Alpha")]
            })),
            complete_entry("T3", "THREE", &json!({
                "states": [state("s3-ip", "In Progress")]
            }))
        ]
    }))
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

fn pairs(entries: &[(&str, &str)]) -> Vec<(String, String)> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn parse_spec(spec: &str) -> (Vec<String>, Vec<(String, String)>) {
    let mut team_ids = Vec::new();
    let mut filters = Vec::new();
    for token in spec.split(';').filter(|token| !token.is_empty()) {
        let (name, value) = token
            .split_once('=')
            .unwrap_or_else(|| panic!("a spec token is name=value: {token}"));
        match name {
            "team" | "teams" => {
                team_ids.extend(value.split(',').map(str::to_owned));
            }
            "text" => filters.push((name.to_owned(), value.to_owned())),
            _ => filters.extend(
                value
                    .split(',')
                    .map(|value| (name.to_owned(), value.to_owned())),
            ),
        }
    }
    (team_ids, filters)
}

fn team_refs(ids: &[String]) -> Vec<TeamRef> {
    ids.iter()
        .map(|id| TeamRef {
            id: id.clone(),
            key: None,
        })
        .collect()
}

fn validated(
    team_ids: &[String],
    filters: &[(String, String)],
    set: &ResolverSet,
) -> ValidatedSearch {
    let configured = ConfiguredSearch::from_pairs(team_ids.to_vec(), filters)
        .expect("the fixture's pairs are well formed");
    preflight(configured, set, None).expect("pre-flight defers every value")
}

fn refusal(result: Result<ValidatedSearch, UnresolvedFilters>) -> String {
    match result {
        Err(refusal) => refusal.to_string(),
        Ok(_) => panic!("expected a refusal"),
    }
}

#[test]
fn every_fixture_row_composes_to_its_expected_filter() {
    let set = fixture_catalogue();
    let rows = rows();
    let mut consumed = 0;

    for (case, spec, expected) in &rows {
        let (team_ids, filters) = parse_spec(spec);
        let lowerable = complete_for_teams(
            validated(&team_ids, &filters, &set),
            &team_refs(&team_ids),
            &set,
        )
        .unwrap_or_else(|refusal| panic!("case {case}: {refusal}"));
        assert_eq!(
            serde_json::to_string(&compose(&lowerable)).expect("serialises"),
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

    assert_eq!(covered.len(), 7, "seven families: {covered:?}");
}

#[test]
fn every_family_is_carried_as_validated_names() {
    let set = fixture_catalogue();
    let filters = pairs(&[
        ("state", "In Progress"),
        ("label", "infra"),
        ("assignee", "alice"),
        ("project", "Alpha"),
        ("text", "needle"),
    ]);

    let carried = validated(&["T1".to_owned()], &filters, &set).to_pairs();

    assert_eq!(
        carried,
        pairs(&[
            ("validated:state", "In Progress"),
            ("validated:project", "Alpha"),
            ("validated:label", "infra"),
            ("validated:assignee", "alice"),
            ("text", "needle"),
        ])
    );
    for (key, _) in &carried {
        assert!(!key.contains('.'), "{key} reads as no GraphQL path");
    }
}

#[test]
fn a_validated_bag_round_trips() {
    let set = fixture_catalogue();
    let team_ids = vec!["T1".to_owned()];
    let carried = validated(
        &team_ids,
        &pairs(&[("label", "infra"), ("label", "a")]),
        &set,
    )
    .to_pairs();

    let round_tripped = ValidatedSearch::from_pairs(team_ids, &carried)
        .expect("a validated bag parses back");

    assert_eq!(round_tripped.to_pairs(), carried);
}

#[test]
fn a_named_pair_in_a_validated_bag_is_refused() {
    let error = ValidatedSearch::from_pairs(
        vec!["T1".to_owned()],
        &pairs(&[("label", "infra")]),
    )
    .expect_err("a named pair has not passed pre-flight");

    assert!(
        error.to_string().contains("E_SEARCH_UNRESOLVED_SCOPE"),
        "{error}"
    );
}

#[test]
fn a_blank_value_is_refused() {
    let error = ConfiguredSearch::from_pairs(
        vec!["T1".to_owned()],
        &pairs(&[("label", "  ")]),
    )
    .expect_err("a blank value names nothing");

    assert!(
        error.to_string().contains("E_SEARCH_BLANK_FILTER"),
        "{error}"
    );
}

#[test]
fn an_unrecognised_filter_key_is_refused() {
    let error = ConfiguredSearch::from_pairs(
        vec!["T1".to_owned()],
        &pairs(&[("colour", "red")]),
    )
    .expect_err("an unknown key is never silently dropped");

    let rendered = error.to_string();
    assert!(rendered.contains("E_SEARCH_UNKNOWN_FILTER"), "{rendered}");
    assert!(rendered.contains("colour"), "{rendered}");
}

fn base_only_preflight(
    filters: &[(&str, &str)],
) -> Result<ValidatedSearch, UnresolvedFilters> {
    let set = fixture_catalogue();
    let base = set.team_by_key("ONE").expect("the base team is catalogued");
    let configured =
        ConfiguredSearch::from_pairs(vec![base.id.clone()], &pairs(filters))
            .expect("well formed");
    preflight(configured, &set, Some(&base))
}

#[test]
fn resolution_reports_every_unresolved_value_at_once() {
    let error = base_only_preflight(&[
        ("state", "Nope"),
        ("label", "typo"),
        ("label", "infra"),
    ])
    .expect_err("two values resolve to nothing");

    let unresolved: Vec<(FilterFamily, String)> = error
        .iter()
        .filter_map(|entry| match entry {
            UnresolvedFilter::Value { family, value, .. } => {
                Some((*family, value.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        unresolved,
        vec![
            (FilterFamily::State, "Nope".to_owned()),
            (FilterFamily::Label, "typo".to_owned()),
        ]
    );
}

#[test]
fn the_refusal_renders_a_header_then_one_indented_line_per_value() {
    let rendered =
        refusal(base_only_preflight(&[("state", "Nope"), ("label", "typo")]));

    let lines: Vec<&str> = rendered.lines().collect();
    assert_eq!(lines[0], "pull filters could not be resolved:");
    assert_eq!(lines.len(), 3, "{rendered}");
    assert!(
        lines[1].starts_with("  E_SEARCH_UNKNOWN_STATE: "),
        "{rendered}"
    );
    assert!(
        lines[2].starts_with("  E_SEARCH_UNKNOWN_LABEL: "),
        "{rendered}"
    );
}

#[test]
fn each_unknown_value_names_its_code_and_the_refresh_remedy() {
    for (family, code) in [
        ("state", "E_SEARCH_UNKNOWN_STATE"),
        ("label", "E_SEARCH_UNKNOWN_LABEL"),
        ("project", "E_SEARCH_UNKNOWN_PROJECT"),
        ("assignee", "E_SEARCH_UNKNOWN_ASSIGNEE"),
    ] {
        let rendered = refusal(base_only_preflight(&[(family, "Nope")]));

        assert!(rendered.contains(code), "{rendered}");
        assert!(rendered.contains("\"Nope\""), "{rendered}");
        assert!(
            rendered.contains("accelerator linear init discover --team-id T1"),
            "the remedy names the base team: {rendered}"
        );
    }
    let assignee = refusal(base_only_preflight(&[("assignee", "Nope")]));
    assert!(
        assignee.contains("member of a team in scope"),
        "an assignee must be a member: {assignee}"
    );
}

#[test]
fn the_remedy_names_a_placeholder_without_a_base_team() {
    let set = resolvers(&json!({
        "labels": [],
        "teams": [complete_entry("T1", "ONE", &json!({}))]
    }));
    let team = set.team_by_key("ONE").expect("catalogued");
    let configured = ConfiguredSearch::from_pairs(
        vec![team.id.clone()],
        &pairs(&[("state", "Nope")]),
    )
    .expect("well formed");

    let rendered = refusal(preflight(configured, &set, Some(&team)));

    assert!(rendered.contains("--team-id <uuid>"), "{rendered}");
}

#[test]
fn ambiguous_values_name_the_team_or_the_candidates() {
    let set = resolvers(&json!({
        "baseTeam": "T1",
        "labels": [],
        "teams": [complete_entry("T1", "ONE", &json!({
            "states": [state("s-a", "Doing"), state("s-b", "Doing")],
            "labels": [label("l-a", "dup"), label("l-b", "dup")],
            "members": [member("u-a", "Sam", "a@x.io"),
                        member("u-b", "Sam", "b@x.io")],
            "projects": (1..=6)
                .map(|n| project(&format!("p-{n}"), "Same"))
                .collect::<Vec<_>>()
        }))]
    }));
    let team = set.team_by_key("ONE").expect("catalogued");
    let rendered = |family: &str, value: &str| {
        let configured = ConfiguredSearch::from_pairs(
            vec![team.id.clone()],
            &pairs(&[(family, value)]),
        )
        .expect("well formed");
        refusal(preflight(configured, &set, Some(&team)))
    };

    let state = rendered("state", "Doing");
    assert!(state.contains("E_SEARCH_AMBIGUOUS_STATE"), "{state}");
    assert!(state.contains("ONE") && state.contains('2'), "{state}");
    let label = rendered("label", "dup");
    assert!(label.contains("E_SEARCH_AMBIGUOUS_LABEL"), "{label}");
    assert!(label.contains("ONE"), "{label}");
    let assignee = rendered("assignee", "Sam");
    assert!(
        assignee.contains("E_SEARCH_AMBIGUOUS_ASSIGNEE"),
        "{assignee}"
    );
    assert!(assignee.contains("full name"), "names the tier: {assignee}");
    assert!(assignee.contains("email"), "suggests the email: {assignee}");
    let project = rendered("project", "Same");
    assert!(project.contains("E_SEARCH_AMBIGUOUS_PROJECT"), "{project}");
    assert!(
        project.contains("p-5") && !project.contains("p-6"),
        "lists up to five candidates: {project}"
    );
}

#[test]
fn an_uncatalogued_or_damaged_catalogue_names_its_own_code() {
    let damaged = UnresolvedFilters::of(
        vec![UnresolvedFilter::Value {
            family: FilterFamily::Label,
            value: "bug".to_owned(),
            reason: Unresolved::NotCatalogued {
                section: linear_client::catalogue::CatalogueSection::Labels,
                cause: linear_client::resolution::CatalogueGap::Damaged,
            },
        }],
        None,
    )
    .expect("one entry");
    let unfetched = UnresolvedFilters::of(
        vec![UnresolvedFilter::Value {
            family: FilterFamily::State,
            value: "Todo".to_owned(),
            reason: Unresolved::TeamUnfetched {
                team: TeamRef {
                    id: "t-ops".to_owned(),
                    key: Some("OPS".to_owned()),
                },
            },
        }],
        None,
    )
    .expect("one entry");

    let damaged = damaged.to_string();
    assert!(damaged.contains("E_SEARCH_CATALOGUE_DAMAGED"), "{damaged}");
    assert!(damaged.contains("version control"), "{damaged}");
    let unfetched = unfetched.to_string();
    assert!(unfetched.contains("E_SEARCH_TEAM_UNFETCHED"), "{unfetched}");
    assert!(unfetched.contains("OPS"), "{unfetched}");
}

#[test]
fn every_filter_key_spells_and_parses_back() {
    for family in FilterFamily::ALL {
        for key in [FilterKey::Named(family), FilterKey::ValidatedName(family)]
        {
            assert_eq!(FilterKey::parse(key.as_str()), Some(key));
        }
    }
    assert_eq!(FilterKey::parse("text"), Some(FilterKey::Text));
    assert_eq!(FilterKey::parse("colour"), None);
}

#[test]
fn every_named_filter_key_is_accepted_by_linear_pull_validation() {
    let accepted = |family: FilterFamily| {
        validate(
            &PullConfig {
                filters: vec![(
                    FilterKey::Named(family).as_str().to_owned(),
                    vec!["x".to_owned()],
                )],
                ..PullConfig::default()
            },
            Tracker::Linear,
        )
        .is_ok()
    };

    for family in [
        FilterFamily::State,
        FilterFamily::Label,
        FilterFamily::Assignee,
        FilterFamily::Project,
    ] {
        assert!(accepted(family), "{family:?}");
    }
}

#[test]
fn the_fetch_page_size_is_linears_bulk_ceiling() {
    assert_eq!(FETCH_PAGE_SIZE, 250);
}
