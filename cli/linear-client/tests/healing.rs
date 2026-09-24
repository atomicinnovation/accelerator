//! Apply-mode self-heal: every synced team's catalogue entry completed, and
//! nothing written for a team that is not synced.

#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use linear_client::cache::{
    CacheError, Filesystem, LinearCache, SystemFilesystem,
};
use linear_client::catalogue::{
    CatalogueSection, CataloguedLabel, LiveCatalogueData, SectionSet, TeamEntry,
};
use linear_client::discovery::{SectionFetch, TeamEntryFetch};
use linear_client::healing::{
    CatalogueHealing, FetchUnavailable, HealOutcome, SyncedTeams,
};
use linear_client::SurfaceError;
use serde_json::{json, Value};
use support::catalogue::{complete_entry, entry, project, state};
use tracker::ExternalId;

/// What Linear knows, as the fetch double serves it.
#[derive(Default)]
struct Linear {
    issues: BTreeMap<String, Option<TeamEntry>>,
    failing_lookups: Vec<String>,
    teams: BTreeMap<String, TeamEntry>,
    unreturned: Vec<String>,
    workspace_labels: Vec<CataloguedLabel>,
    fetch_fails: bool,
    lookups: RefCell<Vec<String>>,
    fetches: RefCell<Vec<(Vec<String>, Vec<CatalogueSection>)>>,
}

impl Linear {
    fn issue(mut self, identifier: &str, team: Option<(&str, &str)>) -> Self {
        self.issues.insert(
            identifier.to_owned(),
            team.map(|(id, key)| TeamEntry::identified(id, key, key)),
        );
        self
    }

    fn team(mut self, id: &str, key: &str) -> Self {
        self.teams.insert(id.to_owned(), full_entry(id, key));
        self
    }

    fn team_entry(mut self, team: TeamEntry) -> Self {
        self.teams.insert(team.id.clone(), team);
        self
    }
}

struct Handle(Rc<Linear>);

impl TeamEntryFetch for Handle {
    fn fetch_team_entries(
        &self,
        team_ids: &[String],
        sections: &SectionSet,
    ) -> Result<SectionFetch, SurfaceError> {
        let requested: Vec<CatalogueSection> = CatalogueSection::ALL
            .into_iter()
            .filter(|section| sections.contains(*section))
            .collect();
        self.0
            .fetches
            .borrow_mut()
            .push((team_ids.to_vec(), requested.clone()));
        if self.0.fetch_fails {
            return Err(SurfaceError::DeadlineExpired {
                operation: "scripted fetch",
            });
        }
        let mut fetch = SectionFetch::default();
        for id in team_ids {
            match self.0.teams.get(id) {
                Some(team) if !self.0.unreturned.contains(id) => {
                    fetch.entries.push(only_sections(team, &requested));
                }
                _ => fetch.unreturned.push(id.clone()),
            }
        }
        if requested.contains(&CatalogueSection::WorkspaceLabels) {
            fetch.workspace_labels = Some(self.0.workspace_labels.clone());
        }
        Ok(fetch)
    }

    fn team_of_identifier(
        &self,
        identifier: &str,
    ) -> Result<Option<TeamEntry>, SurfaceError> {
        self.0.lookups.borrow_mut().push(identifier.to_owned());
        if self.0.failing_lookups.iter().any(|id| id == identifier) {
            return Err(SurfaceError::DeadlineExpired {
                operation: "scripted lookup",
            });
        }
        Ok(self.0.issues.get(identifier).cloned().flatten())
    }
}

fn only_sections(team: &TeamEntry, sections: &[CatalogueSection]) -> TeamEntry {
    let keep = |section| sections.contains(&section);
    TeamEntry {
        states: team
            .states
            .clone()
            .filter(|_| keep(CatalogueSection::States)),
        labels: team
            .labels
            .clone()
            .filter(|_| keep(CatalogueSection::Labels)),
        members: team
            .members
            .clone()
            .filter(|_| keep(CatalogueSection::Members)),
        projects: team
            .projects
            .clone()
            .filter(|_| keep(CatalogueSection::Projects)),
        ..TeamEntry::identified(&team.id, &team.key, &team.name)
    }
}

fn parsed_entry(value: Value) -> TeamEntry {
    serde_json::from_value(value).expect("a team entry")
}

fn full_entry(id: &str, key: &str) -> TeamEntry {
    parsed_entry(complete_entry(id, key, &json!({})))
}

/// A filesystem that counts locks and can refuse writes.
struct SpyFs {
    real: SystemFilesystem,
    locks: Cell<usize>,
    refuse_writes: bool,
}

impl Filesystem for SpyFs {
    fn exists(&self, path: &Path) -> bool {
        self.real.exists(path)
    }

    fn read(&self, path: &Path) -> Result<Option<String>, CacheError> {
        self.real.read(path)
    }

    fn write_atomic(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), CacheError> {
        if self.refuse_writes {
            return Err(CacheError::Io {
                path: path.display().to_string(),
                detail: "refused".to_owned(),
            });
        }
        self.real.write_atomic(path, bytes)
    }

    fn ensure_present(&self, path: &Path) -> Result<(), CacheError> {
        self.real.ensure_present(path)
    }

    fn append_line(&self, path: &Path, line: &str) -> Result<(), CacheError> {
        self.real.append_line(path, line)
    }

    fn with_lock(
        &self,
        lockdir: &Path,
        body: &mut dyn FnMut() -> Result<(), CacheError>,
    ) -> Result<(), CacheError> {
        self.locks.set(self.locks.get() + 1);
        self.real.with_lock(lockdir, body)
    }
}

struct Harness {
    _root: tempfile::TempDir,
    state_dir: PathBuf,
    fs: SpyFs,
    linear: Rc<Linear>,
    healing: CatalogueHealing,
    factory_calls: Cell<usize>,
    unavailable: Option<String>,
}

impl Harness {
    fn new(catalogue: Option<&Value>, linear: Linear) -> Self {
        let root = tempfile::tempdir().expect("tempdir");
        let state_dir = root.path().join("linear");
        std::fs::create_dir_all(&state_dir).expect("state dir");
        if let Some(catalogue) = catalogue {
            std::fs::write(
                state_dir.join("catalogue.json"),
                serde_json::to_string_pretty(catalogue).expect("JSON"),
            )
            .expect("seed the catalogue");
        }
        let fs = SpyFs {
            real: SystemFilesystem::new(root.path().to_path_buf()),
            locks: Cell::new(0),
            refuse_writes: false,
        };
        Self {
            _root: root,
            state_dir,
            fs,
            linear: Rc::new(linear),
            healing: CatalogueHealing::new(),
            factory_calls: Cell::new(0),
            unavailable: None,
        }
    }

    fn holding(self, live: LiveCatalogueData) -> Self {
        self.healing.backfill().hold(live);
        self
    }

    fn heal(&self, corpus: &[&str]) -> HealOutcome {
        let ids: Vec<ExternalId> = corpus
            .iter()
            .map(|id| ExternalId::new((*id).to_owned()))
            .collect();
        let factory = || {
            self.factory_calls.set(self.factory_calls.get() + 1);
            self.unavailable.as_ref().map_or_else(
                || {
                    Ok(Box::new(Handle(Rc::clone(&self.linear)))
                        as Box<dyn TeamEntryFetch>)
                },
                |reason| {
                    Err(FetchUnavailable {
                        reason: reason.clone(),
                    })
                },
            )
        };
        let cache = LinearCache::new(&self.fs, self.state_dir.clone());
        self.healing
            .heal(&SyncedTeams::derive(&ids), &factory, &cache)
    }

    fn text(&self) -> Option<String> {
        std::fs::read_to_string(self.state_dir.join("catalogue.json")).ok()
    }

    fn catalogue(&self) -> Value {
        serde_json::from_str(&self.text().expect("a catalogue")).expect("JSON")
    }

    fn entry(&self, id: &str) -> Option<Value> {
        self.catalogue()["teams"]
            .as_array()
            .expect("teams")
            .iter()
            .find(|entry| entry["id"] == id)
            .cloned()
    }

    fn lookups(&self) -> Vec<String> {
        self.linear.lookups.borrow().clone()
    }

    fn fetches(&self) -> Vec<(Vec<String>, Vec<CatalogueSection>)> {
        self.linear.fetches.borrow().clone()
    }
}

fn is_complete(entry: &Value) -> bool {
    ["states", "labels", "members", "projects"]
        .iter()
        .all(|section| entry[section].is_array())
}

fn complete_catalogue(teams: &[Value]) -> Value {
    json!({ "baseTeam": "t-eng", "labels": [], "teams": teams })
}

fn eng() -> Value {
    complete_entry("t-eng", "ENG", &json!({}))
}

fn legacy_base() -> Value {
    json!({
        "team": { "id": "t-eng", "key": "ENG", "name": "ENG" },
        "workflowStates": [
            { "id": "s-todo", "name": "Todo", "type": "unstarted",
              "position": 0 }
        ]
    })
}

const fn held(entries: Vec<TeamEntry>) -> LiveCatalogueData {
    LiveCatalogueData {
        entries,
        workspace_labels: None,
    }
}

#[test]
fn healing_with_nothing_to_heal_makes_no_request_and_no_write() {
    let catalogue = complete_catalogue(&[eng()]);
    let harness = Harness::new(Some(&catalogue), Linear::default());
    let before = harness.text();

    let outcome = harness.heal(&["ENG-1"]);

    assert_eq!(harness.factory_calls.get(), 0);
    assert_eq!(harness.fs.locks.get(), 0);
    assert_eq!(harness.text(), before);
    assert!(outcome.recorded.is_empty());
}

#[test]
fn healing_adds_the_base_team_from_the_catalogue() {
    let harness = Harness::new(
        Some(&legacy_base()),
        Linear::default().team("t-eng", "ENG"),
    );

    let outcome = harness.heal(&[]);

    assert_eq!(outcome.recorded, vec!["ENG"]);
    assert!(is_complete(
        &harness.entry("t-eng").expect("the base entry")
    ));
}

#[test]
fn a_held_team_is_confirmed_before_its_sections_are_used() {
    let harness = Harness::new(
        Some(&complete_catalogue(&[eng()])),
        Linear::default().issue("OPS-1", Some(("t-ops", "OPS"))),
    )
    .holding(held(vec![full_entry("t-ops", "OPS")]));

    let outcome = harness.heal(&["OPS-1"]);

    assert_eq!(harness.lookups(), vec!["OPS-1"]);
    assert!(harness.fetches().is_empty(), "held sections are reused");
    assert_eq!(outcome.recorded, vec!["OPS"]);
}

#[test]
fn a_held_team_the_identifier_does_not_confirm_is_never_recorded() {
    let catalogue = complete_catalogue(&[eng()]);
    let harness = Harness::new(Some(&catalogue), Linear::default())
        .holding(held(vec![full_entry("t-proj", "PROJ")]));
    let before = harness.text();

    let outcome = harness.heal(&["PROJ-12"]);

    assert!(outcome.recorded.is_empty());
    assert_eq!(harness.text(), before);
    assert_eq!(harness.fs.locks.get(), 0);
}

#[test]
fn healing_catalogues_an_imported_team_that_was_never_fetched() {
    let harness = Harness::new(
        Some(&complete_catalogue(&[eng()])),
        Linear::default()
            .issue("OPS-7", Some(("t-ops", "OPS")))
            .team("t-ops", "OPS"),
    );

    let outcome = harness.heal(&["OPS-7"]);

    assert_eq!(outcome.recorded, vec!["OPS"]);
    assert!(is_complete(&harness.entry("t-ops").expect("recorded")));
}

#[test]
fn healing_re_derives_synced_teams_erased_from_the_catalogue() {
    let harness = Harness::new(
        Some(&legacy_base()),
        Linear::default()
            .issue("OPS-7", Some(("t-ops", "OPS")))
            .team("t-eng", "ENG")
            .team("t-ops", "OPS"),
    );

    harness.heal(&["ENG-1", "OPS-7"]);

    assert!(is_complete(&harness.entry("t-eng").expect("base")));
    assert!(is_complete(&harness.entry("t-ops").expect("re-derived")));
}

#[test]
fn a_renamed_prefix_resolving_to_a_catalogued_team_records_nothing() {
    let catalogue =
        complete_catalogue(&[complete_entry("t-new", "NEW", &json!({}))]);
    let harness = Harness::new(
        Some(&catalogue),
        Linear::default().issue("OLD-1", Some(("t-new", "NEW"))),
    );
    let before = harness.text();

    let outcome = harness.heal(&["OLD-1"]);

    assert_eq!(harness.text(), before);
    assert!(outcome.recorded.is_empty());
    assert!(outcome.fetch_failed.is_empty() && outcome.unconfirmed.is_empty());
}

#[test]
fn a_moved_issue_does_not_hide_its_prefixes_team() {
    let harness = Harness::new(
        Some(&complete_catalogue(&[complete_entry(
            "t-base",
            "BASE",
            &json!({}),
        )])),
        Linear::default()
            .issue("ENG-3", Some(("t-ops", "OPS")))
            .issue("ENG-4", Some(("t-eng", "ENG")))
            .team("t-ops", "OPS")
            .team("t-eng", "ENG"),
    );

    let outcome = harness.heal(&["ENG-3", "ENG-4"]);

    assert_eq!(harness.lookups(), vec!["ENG-3", "ENG-4"]);
    assert_eq!(outcome.recorded, vec!["ENG", "OPS"]);
}

#[test]
fn a_prefix_stops_after_three_identifiers() {
    let mut linear = Linear::default();
    for n in 1..=5 {
        linear = linear.issue(&format!("OLD-{n}"), Some(("t-new", "NEW")));
    }
    let harness = Harness::new(
        Some(&complete_catalogue(&[complete_entry(
            "t-new",
            "NEW",
            &json!({}),
        )])),
        linear,
    );

    harness.heal(&["OLD-1", "OLD-2", "OLD-3", "OLD-4", "OLD-5"]);

    assert_eq!(harness.lookups(), vec!["OLD-1", "OLD-2", "OLD-3"]);
}

#[test]
fn a_not_found_identifier_falls_through_to_the_next() {
    let harness = Harness::new(
        Some(&complete_catalogue(&[eng()])),
        Linear::default()
            .issue("PP-1", None)
            .issue("PP-2", Some(("t-pp", "PP")))
            .team("t-pp", "PP"),
    );

    let outcome = harness.heal(&["PP-1", "PP-2"]);

    assert_eq!(harness.lookups(), vec!["PP-1", "PP-2"]);
    assert_eq!(outcome.recorded, vec!["PP"]);
}

#[test]
fn a_prefix_linear_cannot_confirm_is_unconfirmed_and_never_written() {
    let catalogue = complete_catalogue(&[eng()]);
    let harness = Harness::new(
        Some(&catalogue),
        Linear::default().issue("PROJ-12", None),
    );
    let before = harness.text();

    let outcome = harness.heal(&["PROJ-12"]);

    assert_eq!(
        outcome.unconfirmed,
        vec![("PROJ".to_owned(), vec!["PROJ-12".to_owned()])]
    );
    assert_eq!(harness.text(), before);
}

#[test]
fn a_colliding_foreign_id_is_indistinguishable_from_a_linear_id() {
    let harness = Harness::new(
        Some(&complete_catalogue(&[eng()])),
        Linear::default()
            .issue("PROJ-12", Some(("t-proj", "PROJ")))
            .team("t-proj", "PROJ"),
    );

    let outcome = harness.heal(&["PROJ-12"]);

    assert_eq!(outcome.recorded, vec!["PROJ"]);
}

#[test]
fn healing_never_records_a_team_that_was_fetched_but_not_synced() {
    let catalogue = complete_catalogue(&[eng()]);
    let harness = Harness::new(Some(&catalogue), Linear::default())
        .holding(held(vec![full_entry("t-far", "FAR")]));
    let before = harness.text();

    harness.heal(&["ENG-1"]);

    assert_eq!(harness.text(), before);
}

#[test]
fn healing_reuses_held_sections_and_fetches_only_the_rest() {
    let partly = parsed_entry(entry(
        "t-ops",
        "OPS",
        &json!({
            "states": [state("s-ops", "Todo")],
            "labels": []
        }),
    ));
    let harness = Harness::new(
        Some(&complete_catalogue(&[eng()])),
        Linear::default()
            .issue("OPS-1", Some(("t-ops", "OPS")))
            .team("t-ops", "OPS"),
    )
    .holding(held(vec![partly]));

    harness.heal(&["OPS-1"]);

    assert_eq!(
        harness.fetches(),
        vec![(
            vec!["t-ops".to_owned()],
            vec![CatalogueSection::Members, CatalogueSection::Projects]
        )]
    );
    let recorded = harness.entry("t-ops").expect("recorded");
    assert!(is_complete(&recorded));
    assert_eq!(
        recorded["states"][0]["id"], "s-ops",
        "the held states stand"
    );
}

#[test]
fn healing_completes_an_incomplete_entry_even_when_nothing_was_imported() {
    let harness = Harness::new(
        Some(&complete_catalogue(&[
            eng(),
            entry("t-ops", "OPS", &json!({})),
        ])),
        Linear::default().team("t-ops", "OPS"),
    );

    let outcome = harness.heal(&[]);

    assert_eq!(outcome.recorded, vec!["OPS"]);
    assert!(is_complete(&harness.entry("t-ops").expect("completed")));
}

#[test]
fn completeness_counts_stored_and_held_sections_together() {
    let stored = entry(
        "t-ops",
        "OPS",
        &json!({
            "states": [state("s-ops", "Todo")],
            "labels": []
        }),
    );
    let held_rest = parsed_entry(entry(
        "t-ops",
        "OPS",
        &json!({
            "members": [],
            "projects": []
        }),
    ));
    let harness = Harness::new(
        Some(&complete_catalogue(&[eng(), stored])),
        Linear::default(),
    )
    .holding(held(vec![held_rest]));

    let outcome = harness.heal(&[]);

    assert!(harness.fetches().is_empty(), "nothing is fetched");
    assert_eq!(outcome.recorded, vec!["OPS"]);
    assert!(is_complete(&harness.entry("t-ops").expect("recorded")));
}

#[test]
fn missing_workspace_labels_alone_trigger_a_labels_only_fetch() {
    let harness = Harness::new(
        Some(&json!({ "baseTeam": "t-eng", "teams": [eng()] })),
        Linear::default(),
    );

    harness.heal(&[]);

    assert_eq!(
        harness.fetches(),
        vec![(Vec::new(), vec![CatalogueSection::WorkspaceLabels])]
    );
    assert_eq!(harness.catalogue()["labels"], json!([]));
}

#[test]
fn healing_records_the_returned_teams_and_names_the_unreturned_ones() {
    let mut linear = Linear::default()
        .issue("OPS-1", Some(("t-ops", "OPS")))
        .issue("QA-1", Some(("t-qa", "QA")))
        .team("t-ops", "OPS")
        .team("t-qa", "QA");
    linear.unreturned = vec!["t-qa".to_owned()];
    let harness = Harness::new(Some(&complete_catalogue(&[eng()])), linear);

    let outcome = harness.heal(&["OPS-1", "QA-1"]);

    assert_eq!(outcome.recorded, vec!["OPS"]);
    assert_eq!(outcome.unreturned, vec!["QA"]);
    assert!(harness.entry("t-qa").is_none(), "no partial entry");
}

#[test]
fn a_failed_identifier_lookup_reports_its_prefixes_as_a_fetch_failure() {
    let mut linear = Linear::default().team("t-ops", "OPS");
    linear.failing_lookups = vec!["PP-1".to_owned()];
    let harness = Harness::new(
        Some(&complete_catalogue(&[
            eng(),
            entry("t-ops", "OPS", &json!({})),
        ])),
        linear,
    );

    let outcome = harness.heal(&["PP-1"]);

    assert_eq!(outcome.fetch_failed, vec!["PP"]);
    assert!(outcome.unconfirmed.is_empty());
    assert_eq!(outcome.recorded, vec!["OPS"], "known teams still heal");
}

#[test]
fn a_failed_section_fetch_still_records_entries_complete_from_held_sections() {
    let mut linear = Linear::default()
        .issue("OPS-1", Some(("t-ops", "OPS")))
        .issue("QA-1", Some(("t-qa", "QA")));
    linear.fetch_fails = true;
    let harness = Harness::new(Some(&complete_catalogue(&[eng()])), linear)
        .holding(held(vec![full_entry("t-ops", "OPS")]));

    let outcome = harness.heal(&["OPS-1", "QA-1"]);

    assert_eq!(outcome.recorded, vec!["OPS"]);
    assert_eq!(outcome.fetch_failed, vec!["QA"]);
    assert!(outcome.fetch_failure.is_some());
    assert!(harness.entry("t-qa").is_none());
}

#[test]
fn a_fetch_that_cannot_be_built_records_held_complete_entries() {
    let stored = entry("t-ops", "OPS", &json!({}));
    let mut harness = Harness::new(
        Some(&complete_catalogue(&[
            eng(),
            stored,
            entry("t-qa", "QA", &json!({})),
        ])),
        Linear::default(),
    )
    .holding(held(vec![full_entry("t-ops", "OPS")]));
    harness.unavailable = Some("no credential".to_owned());

    let outcome = harness.heal(&[]);

    assert_eq!(outcome.recorded, vec!["OPS"]);
    assert_eq!(outcome.fetch_unavailable.as_deref(), Some("no credential"));
    assert_eq!(outcome.fetch_failed, vec!["QA"]);
}

#[test]
fn a_heal_with_nothing_changed_takes_no_lock_and_writes_nothing() {
    let lookups_fail = Linear {
        failing_lookups: vec!["PP-1".to_owned()],
        ..Linear::default()
    };
    let fetch_fails = Linear {
        fetch_fails: true,
        ..Linear::default()
    };
    let mut unavailable = Harness::new(Some(&legacy_base()), Linear::default());
    unavailable.unavailable = Some("no credential".to_owned());
    let complete = complete_catalogue(&[eng()]);
    let cases = [
        (Harness::new(Some(&complete), lookups_fail), &["PP-1"][..]),
        (Harness::new(Some(&legacy_base()), fetch_fails), &[][..]),
        (unavailable, &["PP-1"][..]),
    ];

    for (harness, corpus) in cases {
        let before = harness.text();

        let outcome = harness.heal(corpus);

        assert!(outcome.recorded.is_empty(), "{outcome:?}");
        assert_eq!(harness.fs.locks.get(), 0);
        assert_eq!(harness.text(), before);
    }
}

#[test]
fn a_team_with_no_projects_heals_to_a_complete_entry() {
    let ops = TeamEntry {
        projects: Some(Vec::new()),
        ..full_entry("t-ops", "OPS")
    };
    let harness = Harness::new(
        Some(&complete_catalogue(&[
            eng(),
            entry("t-ops", "OPS", &json!({})),
        ])),
        Linear::default().team_entry(ops),
    );

    harness.heal(&[]);
    let fetches = harness.fetches().len();
    harness.heal(&[]);

    assert_eq!(
        harness.entry("t-ops").expect("healed")["projects"],
        json!([])
    );
    assert_eq!(
        harness.fetches().len(),
        fetches,
        "a later heal fetches nothing"
    );
}

#[test]
fn healing_sets_base_team_from_the_legacy_team_and_never_re_points_it() {
    let harness = Harness::new(
        Some(&legacy_base()),
        Linear::default()
            .issue("OPS-1", Some(("t-ops", "OPS")))
            .team("t-eng", "ENG")
            .team("t-ops", "OPS"),
    );

    harness.heal(&["OPS-1"]);

    assert_eq!(harness.catalogue()["baseTeam"], "t-eng");
}

#[test]
fn a_failed_heal_write_reports_the_failure_and_writes_nothing() {
    let mut harness = Harness::new(
        Some(&legacy_base()),
        Linear::default().team("t-eng", "ENG"),
    );
    harness.fs.refuse_writes = true;
    let before = harness.text();

    let outcome = harness.heal(&[]);

    assert!(matches!(outcome.write_failure, Some(CacheError::Io { .. })));
    assert!(outcome.recorded.is_empty());
    assert_eq!(harness.text(), before);
}

#[test]
fn a_damaged_entry_refuses_the_heal_naming_the_entry() {
    let damaged = json!({
        "baseTeam": "t-eng",
        "teams": [eng(), { "id": "t-bad", "key": 7 }]
    });
    let harness =
        Harness::new(Some(&damaged), Linear::default().team("t-eng", "ENG"));
    let before = harness.text();

    let outcome = harness.heal(&[]);

    let Some(CacheError::Unparseable { reason, .. }) = &outcome.write_failure
    else {
        panic!("a damaged entry refuses the heal: {outcome:?}");
    };
    assert!(reason.to_string().contains("t-bad"), "{reason}");
    assert_eq!(harness.text(), before);
}

#[test]
fn a_held_project_section_folds_into_a_catalogued_entry() {
    let with_projects = TeamEntry {
        projects: Some(vec![
            serde_json::from_value(project("p-1", "Alpha")).expect("a project")
        ]),
        ..TeamEntry::identified("t-eng", "ENG", "ENG")
    };
    let stored = entry(
        "t-eng",
        "ENG",
        &json!({
            "states": [state("s-1", "Todo")],
            "labels": [],
            "members": []
        }),
    );
    let harness = Harness::new(
        Some(&json!({ "baseTeam": "t-eng", "labels": [], "teams": [stored] })),
        Linear::default(),
    )
    .holding(held(vec![with_projects]));

    harness.heal(&[]);

    assert_eq!(
        harness.entry("t-eng").expect("the base")["projects"][0]["id"],
        "p-1"
    );
}
