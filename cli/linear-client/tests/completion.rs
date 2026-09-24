//! Completion: every validated value resolved over the scoped teams, once
//! enumeration has named them, into a search that lowers to ids only.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use linear_client::catalogue::{
    CataloguedProject, CataloguedState, LiveCatalogueData, TeamEntry,
};
use linear_client::filter::{
    complete_for_teams, compose, preflight, ConfiguredSearch, LowerableSearch,
    UnresolvedFilter, UnresolvedFilters, ValidatedSearch,
};
use linear_client::resolution::{
    CatalogueGap, FilterFamily, ResolverSet, TeamRef, Unresolved,
};
use serde_json::{json, Map, Number, Value};
use support::catalogue::{
    complete_entry, entry, label, member, project, resolvers, state,
};

fn pairs(entries: &[(&str, &str)]) -> Vec<(String, String)> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn scoped(ids: &[&str]) -> Vec<TeamRef> {
    ids.iter()
        .map(|id| TeamRef {
            id: (*id).to_owned(),
            key: None,
        })
        .collect()
}

fn validated(
    teams: &[&str],
    filters: &[(&str, &str)],
    set: &ResolverSet,
) -> ValidatedSearch {
    let configured = ConfiguredSearch::from_pairs(
        teams.iter().map(|id| (*id).to_owned()).collect(),
        &pairs(filters),
    )
    .expect("well formed");
    preflight(configured, set, None).expect("pre-flight defers")
}

fn complete(
    teams: &[&str],
    filters: &[(&str, &str)],
    set: &ResolverSet,
) -> Result<LowerableSearch, UnresolvedFilters> {
    complete_for_teams(validated(teams, filters, set), &scoped(teams), set)
}

fn lowered(search: &LowerableSearch) -> Value {
    compose(search)
}

fn refused(
    result: Result<LowerableSearch, UnresolvedFilters>,
) -> Vec<Unresolved> {
    match result {
        Err(refusal) => refusal
            .iter()
            .map(|entry| match entry {
                UnresolvedFilter::Value { reason, .. } => reason.clone(),
                other => panic!("expected a value refusal, got {other:?}"),
            })
            .collect(),
        Ok(search) => panic!("expected a refusal, got {}", compose(&search)),
    }
}

fn two_teams() -> ResolverSet {
    resolvers(&json!({
        "baseTeam": "t-eng",
        "labels": [label("wl-sec", "Security")],
        "teams": [
            complete_entry("t-eng", "ENG", &json!({
                "states": [state("s-eng-ip", "In Progress")],
                "labels": [label("l-eng-bug", "Bug")],
                "members": [member("u-ann", "Ann", "ann@x.io")],
                "projects": [project("p-alpha", "Alpha")]
            })),
            complete_entry("t-ops", "OPS", &json!({
                "states": [state("s-ops-ip", "In Progress")],
                "labels": [label("l-ops-bug", "Bug")],
                "members": [member("u-ann", "Ann", "ann@x.io"),
                            member("u-bo", "Bo", "bo@x.io")],
                "projects": [project("p-alpha", "Alpha")]
            }))
        ]
    }))
}

#[test]
fn each_scoped_team_contributes_its_own_ids() {
    let set = two_teams();

    let search = complete(
        &["t-eng", "t-ops"],
        &[("state", "In Progress"), ("label", "Bug")],
        &set,
    )
    .expect("both teams carry both values");

    let filter = lowered(&search);
    assert_eq!(filter["state"]["id"]["in"], json!(["s-eng-ip", "s-ops-ip"]));
    assert_eq!(
        filter["labels"]["id"]["in"],
        json!(["l-eng-bug", "l-ops-bug"])
    );
}

#[test]
fn a_workspace_label_contributes_its_id_once_however_many_teams_are_scoped() {
    let set = two_teams();

    let search = complete(&["t-eng", "t-ops"], &[("label", "Security")], &set)
        .expect("a workspace label resolves anywhere");

    assert_eq!(lowered(&search)["labels"]["id"]["eq"], "wl-sec");
}

#[test]
fn an_assignee_who_is_a_member_of_several_scoped_teams_resolves_once() {
    let set = two_teams();

    let search =
        complete(&["t-eng", "t-ops"], &[("assignee", "ann@x.io")], &set)
            .expect("a shared member resolves");

    assert_eq!(lowered(&search)["assignee"]["id"]["eq"], "u-ann");
}

#[test]
fn an_assignee_outside_every_scoped_team_is_not_found() {
    let set = two_teams();

    assert_eq!(
        refused(complete(&["t-eng"], &[("assignee", "bo@x.io")], &set)),
        vec![Unresolved::NotFound]
    );
}

#[test]
fn a_damaged_teams_section_refuses_as_damaged_before_completion() {
    let set = resolvers(&json!({ "baseTeam": "t-eng", "teams": "oops" }));

    let reasons = refused(complete(&["t-eng"], &[("state", "Todo")], &set));

    assert!(
        matches!(
            reasons.as_slice(),
            [Unresolved::NotCatalogued {
                cause: CatalogueGap::Damaged,
                ..
            }]
        ),
        "{reasons:?}"
    );
}

#[test]
fn a_value_no_scoped_team_carries_refuses_as_unknown() {
    let set = two_teams();

    let error = complete(&["t-eng"], &[("project", "Gamma")], &set)
        .expect_err("never a search without the project family");

    assert!(error.to_string().contains("\"Gamma\""), "{error}");
    assert!(error.to_string().contains("E_SEARCH_UNKNOWN_PROJECT"));
}

#[test]
fn an_unknown_value_refuses_even_when_its_familys_other_values_resolve() {
    let set = two_teams();

    let error =
        complete(&["t-eng"], &[("label", "Bug"), ("label", "typo")], &set)
            .expect_err("typo resolves to nothing");

    let values: Vec<String> = error
        .iter()
        .filter_map(|entry| match entry {
            UnresolvedFilter::Value { value, .. } => Some(value.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec!["typo"]);
}

fn legacy_base() -> ResolverSet {
    resolvers(&json!({
        "team": { "id": "t-eng", "key": "ENG", "name": "Engineering" },
        "workflowStates": [
            { "id": "s-todo", "name": "Todo", "type": "unstarted",
              "position": 0 }
        ]
    }))
}

#[test]
fn a_team_folded_with_only_the_needed_sections_completes() {
    let set = legacy_base();

    let search = complete(&["t-eng"], &[("state", "Todo")], &set)
        .expect("a legacy base entry covers state");

    assert_eq!(lowered(&search)["state"]["id"]["eq"], "s-todo");
}

fn fetched_projects(
    id: &str,
    key: &str,
    projects: &[(&str, &str)],
) -> TeamEntry {
    TeamEntry {
        projects: Some(
            projects
                .iter()
                .map(|(id, name)| CataloguedProject {
                    id: (*id).to_owned(),
                    name: (*name).to_owned(),
                    archived_at: None,
                    extra: Map::new(),
                })
                .collect(),
        ),
        ..TeamEntry::identified(id, key, key)
    }
}

#[test]
fn a_scoped_team_with_no_projects_does_not_refuse() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [
            entry("t-eng", "ENG", &json!({})),
            entry("t-ops", "OPS", &json!({}))
        ]
    }))
    .with_fetched(&LiveCatalogueData {
        entries: vec![
            fetched_projects("t-eng", "ENG", &[("p-alpha", "Alpha")]),
            fetched_projects("t-ops", "OPS", &[]),
        ],
        workspace_labels: None,
    });

    let search = complete(&["t-eng", "t-ops"], &[("project", "Alpha")], &set)
        .expect("an empty projects section covers the family");

    assert_eq!(lowered(&search)["project"]["id"]["eq"], "p-alpha");
}

#[test]
fn a_team_folded_without_a_needed_section_refuses_as_unfetched() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({}))]
    }));

    let reasons = refused(complete(&["t-eng"], &[("project", "Alpha")], &set));

    assert_eq!(
        reasons,
        vec![Unresolved::TeamUnfetched {
            team: TeamRef {
                id: "t-eng".to_owned(),
                key: Some("ENG".to_owned()),
            }
        }]
    );
}

#[test]
fn a_value_ambiguous_within_one_scoped_team_refuses_naming_the_team() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "labels": [],
        "teams": [complete_entry("t-eng", "ENG", &json!({
            "states": [state("s-a", "Doing"), state("s-b", "Doing")]
        }))]
    }));

    let reasons = refused(complete(&["t-eng"], &[("state", "Doing")], &set));

    assert!(
        matches!(
            reasons.as_slice(),
            [Unresolved::AmbiguousRecord { team: Some(team), .. }]
                if team.key.as_deref() == Some("ENG")
        ),
        "{reasons:?}"
    );
}

#[test]
fn a_value_served_only_on_a_second_page_resolves() {
    let second_page_state = CataloguedState {
        id: "s-late".to_owned(),
        name: "Late".to_owned(),
        kind: "started".to_owned(),
        position: Number::from(2),
        archived_at: None,
        extra: Map::new(),
    };
    let first_page_state = CataloguedState {
        id: "s-early".to_owned(),
        name: "Early".to_owned(),
        position: Number::from(1),
        ..second_page_state.clone()
    };
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({}))]
    }))
    .with_fetched(&LiveCatalogueData {
        entries: vec![TeamEntry {
            states: Some(vec![first_page_state, second_page_state]),
            ..TeamEntry::identified("t-eng", "ENG", "Engineering")
        }],
        workspace_labels: None,
    });

    let search = complete(&["t-eng"], &[("state", "Late")], &set)
        .expect("every fetched page is folded in");

    assert_eq!(lowered(&search)["state"]["id"]["eq"], "s-late");
}

#[test]
fn a_search_without_filters_needs_no_coverage() {
    let set = resolvers(&json!({}));

    let search = complete(&["t-unknown"], &[("text", "needle")], &set)
        .expect("text needs no catalogue");

    assert_eq!(
        lowered(&search),
        json!({
            "team": { "id": { "eq": "t-unknown" } },
            "title": { "containsIgnoreCase": "needle" }
        })
    );
}

#[test]
fn compose_accepts_only_a_lowerable_search() {
    let set = two_teams();
    let search: LowerableSearch =
        complete(&["t-eng"], &[("project", "Alpha")], &set).expect("resolves");

    let filter = compose(&search);

    assert_eq!(
        filter,
        json!({
            "project": { "id": { "eq": "p-alpha" } },
            "team": { "id": { "eq": "t-eng" } }
        })
    );
    assert!(
        !filter.to_string().contains("name"),
        "no name comparator survives lowering: {filter}"
    );
    assert_eq!(
        FilterFamily::ALL.len(),
        4,
        "every family lowers through the same id comparator"
    );
}
