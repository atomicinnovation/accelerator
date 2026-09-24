//! The resolution domain over a parsed catalogue: every filter family resolved
//! over the given teams, section coverage, team identity, and the base team's
//! states that transition reads.

#![allow(clippy::expect_used, clippy::panic)]

use linear_client::catalogue::{
    Catalogue, CatalogueSection, CataloguedLabel, CataloguedMember,
    CataloguedState, LiveCatalogueData, SectionSet, TeamEntry,
};
use linear_client::resolution::{
    CatalogueGap, FilterFamily, MatchTier, NameUnresolved, NonEmpty,
    Resolution, ResolvedIds, ResolverSet, SingleResolution, TeamRef,
    TeamScopedResolver as _, Unresolved,
};
use serde_json::{json, Value};

const ARCHIVED: &str = "2026-01-01T00:00:00Z";

fn state(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "type": "started", "position": 1,
            "archivedAt": null })
}

fn archived_state(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "type": "started", "position": 1,
            "archivedAt": ARCHIVED })
}

fn label(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": null })
}

fn archived_label(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": ARCHIVED })
}

fn project(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": null })
}

fn archived_project(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "archivedAt": ARCHIVED })
}

fn member(
    id: &str,
    name: &str,
    display_name: &str,
    email: &str,
    active: bool,
) -> Value {
    json!({ "id": id, "name": name, "displayName": display_name,
            "email": email, "active": active })
}

fn entry(id: &str, key: &str, sections: &Value) -> Value {
    let mut entry = json!({ "id": id, "key": key, "name": key });
    for (section, records) in sections.as_object().expect("an object") {
        entry[section] = records.clone();
    }
    entry
}

fn resolvers(catalogue: &Value) -> ResolverSet {
    Catalogue::from_text(&catalogue.to_string()).resolver_set()
}

fn teams(ids: &[&str]) -> Vec<TeamRef> {
    ids.iter()
        .map(|id| TeamRef {
            id: (*id).to_owned(),
            key: None,
        })
        .collect()
}

fn resolve(
    set: &ResolverSet,
    family: FilterFamily,
    value: &str,
    scoped: &[&str],
) -> Resolution {
    set.for_family(family).resolve(value, &teams(scoped))
}

fn ids(resolution: Resolution) -> Vec<String> {
    match resolution {
        Resolution::Resolved(ids) => ids.iter().map(str::to_owned).collect(),
        Resolution::Unresolved(unresolved) => {
            panic!("expected ids, got {unresolved:?}")
        }
    }
}

fn unresolved(resolution: Resolution) -> Unresolved {
    match resolution {
        Resolution::Unresolved(unresolved) => unresolved,
        Resolution::Resolved(ids) => panic!("expected a refusal, got {ids:?}"),
    }
}

fn two_teams() -> Value {
    json!({
        "baseTeam": "t-eng",
        "labels": [label("wl-sec", "Security")],
        "teams": [
            entry("t-eng", "ENG", &json!({
                "states": [state("s-eng-ip", "In Progress"),
                           state("s-eng-done", "Done")],
                "labels": [label("l-eng-bug", "Bug")],
                "members": [member("u-ann", "Ann Lee", "ann", "ann@x.io", true)],
                "projects": [project("p-alpha", "Alpha")]
            })),
            entry("t-ops", "OPS", &json!({
                "states": [state("s-ops-ip", "In Progress")],
                "labels": [label("l-ops-bug", "Bug"),
                           label("l-ops-infra", "Infra")],
                "members": [member("u-ann", "Ann Lee", "ann", "ann@x.io", true),
                            member("u-bo", "Bo Chan", "bo", "bo@x.io", true)],
                "projects": [project("p-alpha", "Alpha"),
                             project("p-beta", "Beta")]
            }))
        ]
    })
}

#[test]
fn a_state_name_resolves_to_each_given_teams_ids() {
    let set = resolvers(&two_teams());

    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::State,
            "in progress",
            &["t-eng", "t-ops"]
        )),
        vec!["s-eng-ip", "s-ops-ip"]
    );
    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::State,
            "In Progress",
            &["t-ops"]
        )),
        vec!["s-ops-ip"]
    );
}

#[test]
fn a_state_name_no_given_team_carries_is_not_found() {
    let set = resolvers(&two_teams());

    assert_eq!(
        unresolved(resolve(&set, FilterFamily::State, "Done", &["t-ops"])),
        Unresolved::NotFound
    );
}

#[test]
fn a_team_label_resolves_only_for_its_team() {
    let set = resolvers(&two_teams());

    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::Label,
            "infra",
            &["t-eng", "t-ops"]
        )),
        vec!["l-ops-infra"]
    );
    assert_eq!(
        unresolved(resolve(&set, FilterFamily::Label, "infra", &["t-eng"])),
        Unresolved::NotFound
    );
}

#[test]
fn a_workspace_label_resolves_whenever_any_team_is_given() {
    let set = resolvers(&two_teams());

    assert_eq!(
        ids(resolve(&set, FilterFamily::Label, "security", &["t-eng"])),
        vec!["wl-sec"]
    );
    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::Label,
            "security",
            &["t-eng", "t-ops"]
        )),
        vec!["wl-sec"]
    );
    assert!(matches!(
        unresolved(resolve(&set, FilterFamily::Label, "security", &[])),
        Unresolved::NotCatalogued {
            cause: CatalogueGap::Absent,
            ..
        }
    ));
}

#[test]
fn a_member_of_any_given_team_resolves_once_however_many_teams_share_them() {
    let set = resolvers(&two_teams());

    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::Assignee,
            "ann@x.io",
            &["t-eng", "t-ops"]
        )),
        vec!["u-ann"]
    );
    assert_eq!(
        ids(resolve(&set, FilterFamily::Assignee, "Bo Chan", &["t-ops"])),
        vec!["u-bo"]
    );
}

#[test]
fn a_project_linked_to_several_given_teams_resolves_once() {
    let set = resolvers(&two_teams());

    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::Project,
            "alpha",
            &["t-eng", "t-ops"]
        )),
        vec!["p-alpha"]
    );
}

fn one_team(sections: &Value) -> ResolverSet {
    resolvers(&json!({
        "baseTeam": "t-eng",
        "labels": [],
        "teams": [entry("t-eng", "ENG", sections)]
    }))
}

#[test]
fn an_active_state_wins_over_an_archived_state_of_the_same_name() {
    let set = one_team(&json!({
        "states": [archived_state("s-old", "Review"),
                   state("s-new", "Review"),
                   archived_state("s-older", "Review")]
    }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::State, "review", &["t-eng"])),
        vec!["s-new"]
    );
}

#[test]
fn a_single_archived_state_resolves() {
    let set = one_team(&json!({
        "states": [archived_state("s-old", "Review")]
    }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::State, "Review", &["t-eng"])),
        vec!["s-old"]
    );
}

#[test]
fn two_active_labels_of_one_name_in_one_team_are_ambiguous() {
    let set = one_team(&json!({
        "labels": [label("l-1", "Bug"), label("l-2", "bug")]
    }));

    match unresolved(resolve(&set, FilterFamily::Label, "Bug", &["t-eng"])) {
        Unresolved::AmbiguousRecord { team, candidates } => {
            assert_eq!(
                team,
                Some(TeamRef {
                    id: "t-eng".to_owned(),
                    key: Some("ENG".to_owned()),
                }),
                "the refusal names the team"
            );
            let ids: Vec<&str> =
                candidates.iter().map(|each| each.id.as_str()).collect();
            assert_eq!(ids, vec!["l-1", "l-2"]);
        }
        other => panic!("expected an ambiguous label, got {other:?}"),
    }
}

#[test]
fn same_named_labels_in_different_teams_both_contribute() {
    let set = resolvers(&two_teams());

    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::Label,
            "bug",
            &["t-eng", "t-ops"]
        )),
        vec!["l-eng-bug", "l-ops-bug"]
    );
}

#[test]
fn a_workspace_label_and_a_team_label_are_separate_groups() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "labels": [label("wl-bug", "Bug")],
        "teams": [entry("t-eng", "ENG", &json!({
            "labels": [label("l-bug", "Bug")]
        }))]
    }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::Label, "bug", &["t-eng"])),
        vec!["l-bug", "wl-bug"],
        "one match per group is no ambiguity"
    );
}

#[test]
fn a_workspace_label_archived_and_active_resolve_to_the_active_one() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "labels": [archived_label("wl-old", "Sec"), label("wl-new", "Sec")],
        "teams": [entry("t-eng", "ENG", &json!({ "labels": [] }))]
    }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::Label, "sec", &["t-eng"])),
        vec!["wl-new"]
    );
}

#[test]
fn a_unique_project_name_resolves() {
    let set = resolvers(&two_teams());

    assert_eq!(
        ids(resolve(&set, FilterFamily::Project, "Beta", &["t-ops"])),
        vec!["p-beta"]
    );
}

#[test]
fn an_unknown_project_name_is_not_found() {
    let set = resolvers(&two_teams());

    assert_eq!(
        unresolved(resolve(&set, FilterFamily::Project, "Gamma", &["t-ops"])),
        Unresolved::NotFound
    );
}

#[test]
fn the_active_project_wins_over_archived_ones() {
    let set = one_team(&json!({
        "projects": [archived_project("p-1", "Alpha"),
                     project("p-2", "Alpha"),
                     archived_project("p-3", "Alpha")]
    }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::Project, "Alpha", &["t-eng"])),
        vec!["p-2"]
    );
}

#[test]
fn a_single_archived_project_resolves() {
    let set = one_team(&json!({
        "projects": [archived_project("p-1", "Alpha")]
    }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::Project, "Alpha", &["t-eng"])),
        vec!["p-1"]
    );
}

#[test]
fn two_active_projects_are_ambiguous_listing_the_candidates() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [
            entry("t-eng", "ENG", &json!({
                "projects": [project("p-1", "Alpha")]
            })),
            entry("t-ops", "OPS", &json!({
                "projects": [project("p-2", "Alpha")]
            }))
        ]
    }));

    match unresolved(resolve(
        &set,
        FilterFamily::Project,
        "Alpha",
        &["t-eng", "t-ops"],
    )) {
        Unresolved::AmbiguousRecord { team, candidates } => {
            assert_eq!(team, None, "a project is not one team's record");
            let ids: Vec<&str> =
                candidates.iter().map(|each| each.id.as_str()).collect();
            assert_eq!(ids, vec!["p-1", "p-2"]);
        }
        other => panic!("expected an ambiguous project, got {other:?}"),
    }
}

#[test]
fn a_name_shared_only_by_archived_projects_is_ambiguous() {
    let set = one_team(&json!({
        "projects": [archived_project("p-1", "Alpha"),
                     archived_project("p-2", "Alpha")]
    }));

    assert!(matches!(
        unresolved(resolve(&set, FilterFamily::Project, "Alpha", &["t-eng"])),
        Unresolved::AmbiguousRecord { .. }
    ));
}

fn members(records: &[Value]) -> ResolverSet {
    one_team(&json!({ "members": records }))
}

#[test]
fn a_member_matches_on_email_then_full_name_then_display_name() {
    let set = members(&[member("u-1", "Ann Lee", "annie", "ann@x.io", true)]);

    for value in ["ANN@x.io", "ann lee", "Annie"] {
        assert_eq!(
            ids(resolve(&set, FilterFamily::Assignee, value, &["t-eng"])),
            vec!["u-1"],
            "{value}"
        );
    }
}

#[test]
fn one_active_member_wins_over_any_number_of_disabled_ones() {
    let set = members(&[
        member("u-1", "Ann", "a1", "a1@x.io", false),
        member("u-2", "Ann", "a2", "a2@x.io", true),
        member("u-3", "Ann", "a3", "a3@x.io", false),
    ]);

    assert_eq!(
        ids(resolve(&set, FilterFamily::Assignee, "Ann", &["t-eng"])),
        vec!["u-2"]
    );
}

#[test]
fn exactly_one_disabled_member_resolves_when_none_is_active() {
    let set = members(&[member("u-1", "Ann", "a1", "a1@x.io", false)]);

    assert_eq!(
        ids(resolve(&set, FilterFamily::Assignee, "Ann", &["t-eng"])),
        vec!["u-1"]
    );
}

#[test]
fn two_active_or_two_disabled_members_are_ambiguous() {
    for active in [true, false] {
        let set = members(&[
            member("u-1", "Ann", "a1", "a1@x.io", active),
            member("u-2", "Ann", "a2", "a2@x.io", active),
        ]);

        match unresolved(resolve(
            &set,
            FilterFamily::Assignee,
            "Ann",
            &["t-eng"],
        )) {
            Unresolved::AmbiguousMember { tier, candidates } => {
                assert_eq!(tier, MatchTier::FullName);
                assert_eq!(candidates.len(), 2);
            }
            other => panic!("expected an ambiguous member, got {other:?}"),
        }
    }
}

#[test]
fn an_ambiguous_full_name_refuses_even_when_the_display_name_is_unique() {
    let set = members(&[
        member("u-1", "Ann", "Ann", "a1@x.io", true),
        member("u-2", "Ann", "a2", "a2@x.io", true),
    ]);

    assert!(matches!(
        unresolved(resolve(&set, FilterFamily::Assignee, "Ann", &["t-eng"])),
        Unresolved::AmbiguousMember {
            tier: MatchTier::FullName,
            ..
        }
    ));
}

#[test]
fn an_earlier_tier_match_wins_over_another_users_later_tier_match() {
    let set = members(&[
        member("u-1", "Sam", "s1", "s1@x.io", true),
        member("u-2", "Samuel Ray", "Sam", "s2@x.io", true),
    ]);

    assert_eq!(
        ids(resolve(&set, FilterFamily::Assignee, "sam", &["t-eng"])),
        vec!["u-1"]
    );
}

#[test]
fn a_former_member_is_not_found() {
    let set = resolvers(&two_teams());

    assert_eq!(
        unresolved(resolve(
            &set,
            FilterFamily::Assignee,
            "bo@x.io",
            &["t-eng"]
        )),
        Unresolved::NotFound,
        "a user outside every given team does not resolve"
    );
}

#[test]
fn a_member_value_that_matches_nothing_is_not_found() {
    let set = resolvers(&two_teams());

    assert_eq!(
        unresolved(resolve(
            &set,
            FilterFamily::Assignee,
            "nobody",
            &["t-eng", "t-ops"]
        )),
        Unresolved::NotFound
    );
}

fn fetched_member(id: &str, name: &str, active: bool) -> CataloguedMember {
    serde_json::from_value(member(
        id,
        name,
        name,
        &format!("{id}@x.io"),
        active,
    ))
    .expect("a member")
}

#[test]
fn a_fetched_member_record_wins_over_a_stored_one() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [
            entry("t-eng", "ENG", &json!({
                "members": [member("u-1", "Ann", "Ann", "u-1@x.io", false)]
            })),
            entry("t-ops", "OPS", &json!({}))
        ]
    }))
    .with_fetched(&LiveCatalogueData {
        entries: vec![TeamEntry {
            members: Some(vec![
                fetched_member("u-1", "Ann", true),
                fetched_member("u-2", "Ann", false),
            ]),
            ..TeamEntry::identified("t-ops", "OPS", "OPS")
        }],
        workspace_labels: None,
    });

    assert_eq!(
        ids(resolve(
            &set,
            FilterFamily::Assignee,
            "Ann",
            &["t-eng", "t-ops"]
        )),
        vec!["u-1"],
        "the fetched, active record of u-1 wins over its stored, disabled one"
    );
}

#[test]
fn an_entry_covers_exactly_the_sections_it_carries() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "labels": [],
        "teams": [entry("t-eng", "ENG", &json!({
            "states": [state("s-1", "Todo")],
            "labels": []
        }))]
    }));
    let cases: [(&[CatalogueSection], bool); 6] = [
        (&[CatalogueSection::States], true),
        (&[CatalogueSection::States, CatalogueSection::Labels], true),
        (
            &[CatalogueSection::Labels, CatalogueSection::WorkspaceLabels],
            true,
        ),
        (&[CatalogueSection::Members], false),
        (
            &[CatalogueSection::States, CatalogueSection::Projects],
            false,
        ),
        (&[], true),
    ];

    for (sections, expected) in cases {
        assert_eq!(
            set.covers("t-eng", &SectionSet::of(sections)),
            Ok(expected),
            "{sections:?}"
        );
    }
    assert_eq!(
        set.covers("t-unknown", &SectionSet::of(&[CatalogueSection::States])),
        Ok(false),
        "an uncatalogued team covers nothing"
    );
}

#[test]
fn a_legacy_base_entry_covers_states_only() {
    let set = resolvers(&json!({
        "team": { "id": "t-eng", "key": "ENG", "name": "Engineering" },
        "workflowStates": [
            { "id": "s-1", "name": "Todo", "type": "unstarted", "position": 0 }
        ]
    }));

    assert_eq!(
        set.covers("t-eng", &FilterFamily::State.sections()),
        Ok(true)
    );
    for family in [
        FilterFamily::Label,
        FilterFamily::Assignee,
        FilterFamily::Project,
    ] {
        assert_eq!(
            set.covers("t-eng", &family.sections()),
            Ok(false),
            "{family:?}"
        );
    }
}

#[test]
fn a_0229_teams_entry_covers_nothing() {
    let set = resolvers(&json!({
        "team": { "id": "t-eng", "key": "ENG", "name": "Engineering" },
        "teams": [{ "id": "t-ops", "key": "OPS", "name": "Operations" }]
    }));

    for section in CatalogueSection::ALL {
        assert_eq!(
            set.covers("t-ops", &SectionSet::of(&[section])),
            Ok(false),
            "{section:?}"
        );
    }
}

#[test]
fn the_label_family_needs_workspace_labels_too() {
    assert_eq!(
        FilterFamily::Label.sections(),
        SectionSet::of(&[
            CatalogueSection::Labels,
            CatalogueSection::WorkspaceLabels
        ])
    );
    let without_workspace_labels = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({ "labels": [] }))]
    }));

    assert_eq!(
        without_workspace_labels
            .covers("t-eng", &FilterFamily::Label.sections()),
        Ok(false)
    );
}

#[test]
fn is_complete_is_true_exactly_for_entries_with_all_four_sections() {
    let complete = json!({
        "states": [state("s-1", "Todo")],
        "labels": [],
        "members": [],
        "projects": []
    });
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "labels": [],
        "teams": [
            entry("t-eng", "ENG", &complete),
            entry("t-ops", "OPS", &json!({
                "states": [state("s-2", "Todo")],
                "labels": [],
                "members": []
            }))
        ]
    }));

    assert_eq!(set.is_complete("t-eng"), Ok(true));
    assert_eq!(set.is_complete("t-ops"), Ok(false));
    assert_eq!(set.is_complete("t-unknown"), Ok(false));
}

#[test]
fn covers_reports_a_damaged_teams_section() {
    let set = resolvers(&json!({
        "team": { "id": "t-eng", "key": "ENG", "name": "Engineering" },
        "teams": "not an array"
    }));

    assert_eq!(
        set.covers("t-eng", &FilterFamily::State.sections()),
        Err(CatalogueGap::Damaged)
    );
    assert_eq!(set.is_complete("t-eng"), Err(CatalogueGap::Damaged));
}

#[test]
fn covers_reports_a_damaged_section_of_one_entry() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({
            "states": [state("s-1", "Todo")],
            "members": "not an array"
        }))]
    }));

    assert_eq!(
        set.covers("t-eng", &FilterFamily::Assignee.sections()),
        Err(CatalogueGap::Damaged)
    );
    assert_eq!(
        set.covers("t-eng", &FilterFamily::State.sections()),
        Ok(true)
    );
}

fn fetched_state(id: &str, name: &str) -> CataloguedState {
    serde_json::from_value(state(id, name)).expect("a state")
}

#[test]
fn folded_entries_resolve_like_catalogued_ones() {
    let catalogued = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({
            "states": [state("s-1", "In Review")]
        }))]
    }));
    let folded = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({}))]
    }))
    .with_fetched(&LiveCatalogueData {
        entries: vec![TeamEntry {
            states: Some(vec![fetched_state("s-1", "In Review")]),
            ..TeamEntry::identified("t-eng", "ENG", "ENG")
        }],
        workspace_labels: None,
    });

    for set in [&catalogued, &folded] {
        assert_eq!(
            ids(resolve(
                set,
                FilterFamily::State,
                "  iN rEVIEW ",
                &["t-eng"]
            )),
            vec!["s-1"]
        );
    }
}

#[test]
fn folding_only_the_needed_sections_covers_the_family() {
    let set =
        resolvers(&json!({ "labels": [] })).with_fetched(&LiveCatalogueData {
            entries: vec![TeamEntry {
                states: Some(vec![fetched_state("s-1", "Todo")]),
                ..TeamEntry::identified("t-new", "NEW", "New")
            }],
            workspace_labels: None,
        });

    assert_eq!(
        set.covers("t-new", &FilterFamily::State.sections()),
        Ok(true)
    );
    assert_eq!(
        set.covers("t-new", &FilterFamily::Label.sections()),
        Ok(false)
    );
}

#[test]
fn fetched_workspace_labels_fold_in() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({ "labels": [] }))]
    }))
    .with_fetched(&LiveCatalogueData {
        entries: Vec::new(),
        workspace_labels: Some(vec![
            serde_json::from_value::<CataloguedLabel>(label(
                "wl-1", "Security",
            ))
            .expect("a label"),
        ]),
    });

    assert_eq!(
        set.covers("t-eng", &FilterFamily::Label.sections()),
        Ok(true)
    );
    assert_eq!(
        ids(resolve(&set, FilterFamily::Label, "security", &["t-eng"])),
        vec!["wl-1"]
    );
}

#[test]
fn a_fetched_section_replaces_a_damaged_one() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [entry("t-eng", "ENG", &json!({ "states": "damaged" }))]
    }))
    .with_fetched(&LiveCatalogueData {
        entries: vec![TeamEntry {
            states: Some(vec![fetched_state("s-1", "Todo")]),
            ..TeamEntry::identified("t-eng", "ENG", "ENG")
        }],
        workspace_labels: None,
    });

    assert_eq!(
        ids(resolve(&set, FilterFamily::State, "todo", &["t-eng"])),
        vec!["s-1"]
    );
}

#[test]
fn a_given_team_without_the_section_is_unfetched() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [
            entry("t-eng", "ENG", &json!({ "states": [state("s-1", "Todo")] })),
            entry("t-ops", "OPS", &json!({}))
        ]
    }));

    assert_eq!(
        unresolved(resolve(
            &set,
            FilterFamily::State,
            "Todo",
            &["t-eng", "t-ops"]
        )),
        Unresolved::TeamUnfetched {
            team: TeamRef {
                id: "t-ops".to_owned(),
                key: Some("OPS".to_owned()),
            }
        }
    );
}

#[test]
fn matching_is_unicode_case_insensitive() {
    let set = one_team(&json!({ "labels": [label("l-1", "Ürgent")] }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::Label, "üRGENT", &["t-eng"])),
        vec!["l-1"]
    );
}

#[test]
fn an_empty_email_or_display_name_never_matches_and_a_blank_value_never_resolves(
) {
    let set = members(&[member("u-1", "Ann", "", "", true)]);

    for value in ["", "   "] {
        assert_eq!(
            unresolved(resolve(
                &set,
                FilterFamily::Assignee,
                value,
                &["t-eng"]
            )),
            Unresolved::NotFound,
            "{value:?}"
        );
    }
    let states = one_team(&json!({ "states": [state("s-1", "Todo")] }));
    assert_eq!(
        unresolved(resolve(&states, FilterFamily::State, " ", &["t-eng"])),
        Unresolved::NotFound
    );
}

#[test]
fn an_entry_with_a_blank_id_is_ignored() {
    let set = resolvers(&json!({
        "teams": [{ "id": "  ", "key": "ENG", "name": "Engineering" }]
    }));

    assert_eq!(set.team_by_key("ENG"), None);
    assert!(set.catalogued_teams().is_empty());
}

#[test]
fn a_malformed_entry_is_not_catalogued_as_damaged_and_leaves_the_others_intact()
{
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [
            { "id": "t-bad", "key": 5, "states": [state("s-x", "Todo")] },
            entry("t-eng", "ENG", &json!({ "states": [state("s-1", "Todo")] }))
        ]
    }));

    assert_eq!(
        unresolved(resolve(&set, FilterFamily::State, "Todo", &["t-bad"])),
        Unresolved::NotCatalogued {
            section: CatalogueSection::States,
            cause: CatalogueGap::Damaged,
        }
    );
    assert_eq!(
        ids(resolve(&set, FilterFamily::State, "Todo", &["t-eng"])),
        vec!["s-1"]
    );
}

fn team_state(set: &ResolverSet, name: &str) -> SingleResolution {
    set.team_states().resolve(name)
}

#[test]
fn team_states_resolve_a_single_id_over_the_base_entrys_active_states() {
    let set = resolvers(&json!({
        "baseTeam": "t-eng",
        "teams": [
            entry("t-eng", "ENG", &json!({
                "states": [archived_state("s-old", "Done"),
                           state("s-done", "Done")]
            })),
            entry("t-ops", "OPS", &json!({
                "states": [state("s-ops", "Blocked")]
            }))
        ]
    }));

    assert_eq!(
        team_state(&set, "  done "),
        SingleResolution::Resolved("s-done".to_owned())
    );
    assert_eq!(
        team_state(&set, "Blocked"),
        SingleResolution::Unresolved(NameUnresolved::NotFound),
        "another team's state is not the base team's"
    );
}

#[test]
fn an_ambiguous_team_state_reports_its_count() {
    let set = one_team(&json!({
        "states": [state("s-1", "In Review"), state("s-2", "in review")]
    }));

    assert_eq!(
        team_state(&set, "In Review"),
        SingleResolution::Unresolved(NameUnresolved::Ambiguous { count: 2 })
    );
}

#[test]
fn a_legacy_file_resolves_team_states_through_workflow_states() {
    let set = resolvers(&json!({
        "team": { "id": "t-eng", "key": "ENG", "name": "Engineering" },
        "workflowStates": [
            { "id": "s-1", "name": "In Progress", "type": "started",
              "position": 1 }
        ]
    }));

    assert_eq!(
        team_state(&set, "in progress"),
        SingleResolution::Resolved("s-1".to_owned())
    );
}

#[test]
fn no_base_team_is_not_catalogued() {
    let set = resolvers(&json!({
        "teams": [entry("t-eng", "ENG", &json!({
            "states": [state("s-1", "Todo")]
        }))]
    }));

    assert_eq!(
        team_state(&set, "Todo"),
        SingleResolution::Unresolved(NameUnresolved::NotCatalogued(
            CatalogueGap::Absent
        ))
    );
}

fn written(dir: &std::path::Path, catalogue: &Value) {
    let linear = dir.join("linear");
    std::fs::create_dir_all(&linear).expect("mkdir");
    std::fs::write(linear.join("catalogue.json"), catalogue.to_string())
        .expect("write the catalogue");
}

#[test]
fn an_absent_catalogue_resolves_nothing_as_absent() {
    let set =
        Catalogue::load(std::path::Path::new("/nonexistent")).resolver_set();

    assert_eq!(
        team_state(&set, "anything"),
        SingleResolution::Unresolved(NameUnresolved::NotCatalogued(
            CatalogueGap::Absent
        ))
    );
    assert_eq!(set.team_by_key("ENG"), None);
}

#[test]
fn an_unparseable_catalogue_is_damaged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let linear = dir.path().join("linear");
    std::fs::create_dir_all(&linear).expect("mkdir");
    std::fs::write(linear.join("catalogue.json"), "<<<<<<< HEAD\n{")
        .expect("write");

    let set = Catalogue::load(dir.path()).resolver_set();

    assert_eq!(
        team_state(&set, "Todo"),
        SingleResolution::Unresolved(NameUnresolved::NotCatalogued(
            CatalogueGap::Damaged
        ))
    );
    assert_eq!(
        set.covers("t-eng", &FilterFamily::State.sections()),
        Err(CatalogueGap::Damaged)
    );
}

#[test]
fn resolvers_loaded_once_keep_resolving_after_the_file_is_removed() {
    let dir = tempfile::tempdir().expect("tempdir");
    written(dir.path(), &two_teams());
    let catalogue = Catalogue::load(dir.path());
    std::fs::remove_file(dir.path().join("linear/catalogue.json"))
        .expect("remove the file");

    let set = catalogue.resolver_set();

    assert_eq!(
        ids(resolve(&set, FilterFamily::Project, "Beta", &["t-ops"])),
        vec!["p-beta"]
    );
    assert_eq!(
        team_state(&set, "Done"),
        SingleResolution::Resolved("s-eng-done".to_owned())
    );
}

#[test]
fn a_damaged_members_section_does_not_prevent_state_resolution() {
    let set = one_team(&json!({
        "states": [state("s-1", "Todo")],
        "members": [{ "id": "u-1" }]
    }));

    assert_eq!(
        ids(resolve(&set, FilterFamily::State, "Todo", &["t-eng"])),
        vec!["s-1"]
    );
    assert_eq!(
        team_state(&set, "Todo"),
        SingleResolution::Resolved("s-1".to_owned())
    );
    assert_eq!(
        unresolved(resolve(&set, FilterFamily::Assignee, "Ann", &["t-eng"])),
        Unresolved::NotCatalogued {
            section: CatalogueSection::Members,
            cause: CatalogueGap::Damaged,
        }
    );
}

#[test]
fn catalogue_exposes_the_base_team_and_its_catalogued_teams() {
    let catalogue = Catalogue::from_text(&two_teams().to_string());

    assert_eq!(catalogue.base_team_id(), Some("t-eng".to_owned()));
    assert_eq!(
        catalogue.catalogued_teams(),
        vec![
            ("ENG".to_owned(), "t-eng".to_owned()),
            ("OPS".to_owned(), "t-ops".to_owned()),
        ]
    );
}

#[test]
fn a_team_key_resolves_through_the_entries() {
    let set = resolvers(&two_teams());

    assert_eq!(
        set.team_by_key("  OPS "),
        Some(TeamRef {
            id: "t-ops".to_owned(),
            key: Some("OPS".to_owned()),
        })
    );
    assert_eq!(set.team_by_key("OTHER"), None);
}

#[test]
fn every_key_in_scope_has_its_id_among_the_teams_paged() {
    let cases: [(&str, Value, &[&str]); 5] = [
        (
            "legacy",
            json!({
                "team": { "id": "t-eng", "key": "ENG", "name": "Eng" },
                "workflowStates": []
            }),
            &["ENG"],
        ),
        (
            "0229",
            json!({
                "team": { "id": "t-eng", "key": "ENG", "name": "Eng" },
                "teams": [{ "id": "t-ops", "key": "OPS", "name": "Ops" }]
            }),
            &["ENG", "OPS"],
        ),
        ("converged", two_teams(), &["ENG", "OPS"]),
        (
            "partly damaged",
            json!({
                "baseTeam": "t-eng",
                "teams": [
                    entry("t-eng", "ENG", &json!({ "members": "damaged" })),
                    entry("t-ops", "OPS", &json!({}))
                ]
            }),
            &["ENG", "OPS"],
        ),
        (
            "blank id",
            json!({
                "baseTeam": "t-eng",
                "teams": [
                    entry("t-eng", "ENG", &json!({})),
                    { "id": "", "key": "OPS", "name": "Ops" }
                ]
            }),
            &["ENG"],
        ),
    ];

    for (case, catalogue, expected_keys) in cases {
        let set = resolvers(&catalogue);
        let catalogued = set.catalogued_teams();
        let keys: Vec<&str> =
            catalogued.iter().map(|(key, _)| key.as_str()).collect();
        let paged: Vec<&str> =
            catalogued.iter().map(|(_, id)| id.as_str()).collect();

        assert_eq!(keys, expected_keys, "{case}: the keys in scope");
        for key in keys {
            let team = set
                .team_by_key(key)
                .unwrap_or_else(|| panic!("{case}: {key} resolves"));
            assert!(
                paged.contains(&team.id.as_str()),
                "{case}: {key}'s id is among the teams paged"
            );
        }
    }
}

#[test]
fn a_grown_catalogue_resolves_every_team_in_the_teams_array() {
    let set = resolvers(&json!({
        "team": { "id": "team-1", "key": "ENG", "name": "Engineering" },
        "teams": [
            { "id": "team-1", "key": "ENG", "name": "Engineering" },
            { "id": "team-2", "key": "OPS", "name": "Operations" }
        ],
        "workflowStates": []
    }));

    assert_eq!(
        set.team_by_key("ENG").map(|team| team.id),
        Some("team-1".to_owned())
    );
    assert_eq!(
        set.team_by_key("OPS").map(|team| team.id),
        Some("team-2".to_owned())
    );
}

#[test]
fn a_pre_upgrade_catalogue_resolves_base_only_and_lists_the_base() {
    let set = resolvers(&json!({
        "team": { "id": "team-1", "key": "ENG", "name": "Engineering" }
    }));

    assert_eq!(
        set.team_by_key("ENG").map(|team| team.id),
        Some("team-1".to_owned())
    );
    assert_eq!(
        set.catalogued_teams(),
        vec![("ENG".to_owned(), "team-1".to_owned())]
    );
}

#[test]
fn a_blank_team_key_resolves_nothing() {
    let set = resolvers(&json!({
        "team": { "id": "team-1", "key": "" }
    }));

    assert_eq!(set.team_by_key(""), None, "a blank key must not resolve");
    assert_eq!(set.team_by_key("ENG"), None);
}

#[test]
fn a_non_empty_collection_cannot_be_built_empty() {
    assert_eq!(NonEmpty::<String>::from_vec(Vec::new()), None);
    let built = NonEmpty::from_vec(vec![1, 2, 3]).expect("non-empty");
    assert_eq!(built.len(), 3);
    assert_eq!(built.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(NonEmpty::new(7, Vec::new()).len(), 1);
}

#[test]
fn resolved_ids_reject_a_blank_id() {
    let blank = NonEmpty::new("id-1".to_owned(), vec!["  ".to_owned()]);
    assert_eq!(ResolvedIds::new(blank), None);

    let ids = ResolvedIds::new(NonEmpty::new(
        "id-1".to_owned(),
        vec!["id-2".to_owned()],
    ))
    .expect("no blank id");
    assert_eq!(ids.iter().collect::<Vec<_>>(), vec!["id-1", "id-2"]);
}
