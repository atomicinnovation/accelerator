//! What a sync does once its report is out: provider upkeep that must never
//! change the run's outcome.

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use linear_client::cache::CacheError;
use linear_client::cache::LinearCache;
use linear_client::cache::SystemFilesystem;
use linear_client::discovery::TeamEntryFetch;
use linear_client::healing::CatalogueHealing;
use linear_client::healing::FetchUnavailable;
use linear_client::healing::HealOutcome;
use linear_client::healing::SyncedTeams;
use tracker::ExternalId;
use work_adapters::sync::run::DiscoveryStatus;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunMode;
use work_adapters::sync::run::RunReport;

pub struct FinishedRun<'a> {
    pub report: &'a RunReport,
    pub mode: RunMode,
    /// The `external_id` of every tracked work item in the corpus.
    pub corpus_external_ids: Vec<ExternalId>,
    pub integrations_root: &'a Path,
    pub repo_root: &'a Path,
}

impl FinishedRun<'_> {
    /// The remote issues this run imported as new work items. A
    /// create-from-remote item is planned under its external id, having no
    /// local id until it is authored.
    #[must_use]
    pub fn applied_imports(&self) -> Vec<ExternalId> {
        self.report
            .reported
            .iter()
            .filter(|item| {
                matches!(
                    item.planned.action,
                    work::sync::Action::CreateFromRemote
                ) && matches!(item.outcome, ItemOutcome::Applied)
            })
            .map(|item| ExternalId::new(item.planned.id.clone()))
            .collect()
    }
}

pub trait RunFinaliser {
    /// Runs after the report is rendered, writing anything worth saying to
    /// `diagnostics`.
    fn finalise(&self, run: &FinishedRun<'_>, diagnostics: &mut dyn Write);
}

/// For a tracker with nothing to keep up after a run.
pub struct NoFinaliser;

impl RunFinaliser for NoFinaliser {
    fn finalise(&self, _run: &FinishedRun<'_>, _diagnostics: &mut dyn Write) {}
}

pub type TeamEntryFetchFactory<'a> =
    dyn Fn() -> Result<Box<dyn TeamEntryFetch>, FetchUnavailable> + 'a;

/// Completes the Linear catalogue for every synced team at the end of an
/// apply-mode run whose pull ran.
pub struct LinearCatalogueFinaliser<'a> {
    healing: Arc<CatalogueHealing>,
    fetch: &'a TeamEntryFetchFactory<'a>,
}

impl<'a> LinearCatalogueFinaliser<'a> {
    /// `healing` must be the one whose buffer the run's clients hold into.
    #[must_use]
    pub fn new(
        healing: Arc<CatalogueHealing>,
        fetch: &'a TeamEntryFetchFactory<'a>,
    ) -> Self {
        Self { healing, fetch }
    }
}

impl RunFinaliser for LinearCatalogueFinaliser<'_> {
    fn finalise(&self, run: &FinishedRun<'_>, diagnostics: &mut dyn Write) {
        if run.mode == RunMode::Preview
            || matches!(run.report.discovery, DiscoveryStatus::SkippedPushOnly)
        {
            return;
        }
        let imports = run.applied_imports();
        let synced = SyncedTeams::derive(
            run.corpus_external_ids.iter().chain(imports.iter()),
        );
        let filesystem = SystemFilesystem::new(run.repo_root.to_path_buf());
        let cache =
            LinearCache::new(&filesystem, run.integrations_root.join("linear"));
        let outcome = self.healing.heal(&synced, self.fetch, &cache);
        report_heal(&outcome, diagnostics);
    }
}

fn report_heal(outcome: &HealOutcome, diagnostics: &mut dyn Write) {
    if !outcome.recorded.is_empty() {
        let _ = writeln!(
            diagnostics,
            "note: recorded complete Linear catalogue entries for synced \
             team(s) {} in catalogue.json. The catalogue is version-controlled \
             and repo-wide — commit it.",
            outcome.recorded.join(", ")
        );
    }
    let mut refetched: Vec<String> = outcome.unreturned.clone();
    refetched.extend(outcome.fetch_failed.iter().cloned());
    if !refetched.is_empty() {
        let reason = outcome
            .fetch_unavailable
            .as_ref()
            .or(outcome.fetch_failure.as_ref())
            .map(|reason| format!(" ({reason})"))
            .unwrap_or_default();
        let _ = writeln!(
            diagnostics,
            "warning: the Linear catalogue could not be completed{reason}; \
             each pull will fetch {} again until catalogue.json is updated \
             and committed.",
            refetched.join(", ")
        );
    }
    if !outcome.unconfirmed.is_empty() {
        let identifiers: Vec<&str> = outcome
            .unconfirmed
            .iter()
            .flat_map(|(_, identifiers)| identifiers.iter().map(String::as_str))
            .collect();
        let _ = writeln!(
            diagnostics,
            "warning: Linear does not recognise {} as Linear issues; check \
             their external_id.",
            identifiers.join(", ")
        );
    }
    match &outcome.write_failure {
        Some(error @ CacheError::Unparseable { .. }) => {
            let _ = writeln!(diagnostics, "warning: {error}");
        }
        Some(error) => {
            let _ = writeln!(
                diagnostics,
                "warning: the Linear catalogue could not be written ({error}); \
                 each pull will fetch the teams it lacks again."
            );
        }
        None => {}
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::cell::Cell;
    use std::collections::BTreeMap;

    use linear_client::catalogue::{CatalogueSection, SectionSet, TeamEntry};
    use linear_client::discovery::SectionFetch;
    use linear_client::SurfaceError;
    use serde_json::json;
    use work::sync::{Action, PlannedAction, SyncState};
    use work_adapters::sync::baseline::Degradation;
    use work_adapters::sync::run::ReportedItem;

    use super::*;

    /// Linear as the finaliser's fetch sees it: teams whose every section is
    /// served, and the issues whose team a lookup confirms.
    #[derive(Clone, Default)]
    struct Linear {
        teams: BTreeMap<String, TeamEntry>,
        issues: BTreeMap<String, TeamEntry>,
        unreturned: Vec<String>,
        fetch_fails: bool,
    }

    impl Linear {
        fn team(mut self, id: &str, key: &str) -> Self {
            self.teams.insert(id.to_owned(), complete(id, key));
            self
        }

        fn issue(mut self, identifier: &str, id: &str, key: &str) -> Self {
            self.issues.insert(
                identifier.to_owned(),
                TeamEntry::identified(id, key, key),
            );
            self
        }
    }

    impl TeamEntryFetch for Linear {
        fn fetch_team_entries(
            &self,
            team_ids: &[String],
            sections: &SectionSet,
        ) -> Result<SectionFetch, SurfaceError> {
            if self.fetch_fails {
                return Err(SurfaceError::DeadlineExpired {
                    operation: "scripted fetch",
                });
            }
            let mut fetch = SectionFetch::default();
            for id in team_ids {
                match self.teams.get(id) {
                    Some(team) if !self.unreturned.contains(id) => {
                        fetch.entries.push(team.clone());
                    }
                    _ => fetch.unreturned.push(id.clone()),
                }
            }
            if sections.contains(CatalogueSection::WorkspaceLabels) {
                fetch.workspace_labels = Some(Vec::new());
            }
            Ok(fetch)
        }

        fn team_of_identifier(
            &self,
            identifier: &str,
        ) -> Result<Option<TeamEntry>, SurfaceError> {
            Ok(self.issues.get(identifier).cloned())
        }
    }

    fn complete(id: &str, key: &str) -> TeamEntry {
        serde_json::from_value(json!({
            "id": id, "key": key, "name": key,
            "states": [{ "id": format!("{id}-todo"), "name": "Todo",
                         "type": "unstarted", "position": 0 }],
            "labels": [], "members": [], "projects": []
        }))
        .expect("a team entry")
    }

    fn legacy_catalogue() -> serde_json::Value {
        json!({
            "team": { "id": "t-eng", "key": "ENG", "name": "ENG" },
            "workflowStates": [
                { "id": "s-todo", "name": "Todo", "type": "unstarted",
                  "position": 0 }
            ]
        })
    }

    fn complete_catalogue() -> serde_json::Value {
        json!({
            "baseTeam": "t-eng",
            "labels": [],
            "teams": [complete("t-eng", "ENG")]
        })
    }

    struct Repo {
        root: tempfile::TempDir,
    }

    impl Repo {
        fn seeded(catalogue: &serde_json::Value) -> Self {
            let root = tempfile::tempdir().expect("tempdir");
            let linear = root.path().join("integrations/linear");
            std::fs::create_dir_all(&linear).expect("state dir");
            std::fs::write(
                linear.join("catalogue.json"),
                serde_json::to_string_pretty(catalogue).expect("JSON"),
            )
            .expect("seed");
            Self { root }
        }

        fn catalogue_text(&self) -> String {
            std::fs::read_to_string(
                self.root.path().join("integrations/linear/catalogue.json"),
            )
            .expect("the catalogue")
        }

        fn finalise(
            &self,
            linear: Result<Linear, FetchUnavailable>,
            report: &RunReport,
            mode: RunMode,
            corpus: &[&str],
        ) -> String {
            let factory = move || match linear.clone() {
                Ok(linear) => Ok(Box::new(linear) as Box<dyn TeamEntryFetch>),
                Err(unavailable) => Err(unavailable),
            };
            let finaliser = LinearCatalogueFinaliser::new(
                Arc::new(CatalogueHealing::new()),
                &factory,
            );
            let integrations = self.root.path().join("integrations");
            let mut diagnostics = Vec::new();
            finaliser.finalise(
                &FinishedRun {
                    report,
                    mode,
                    corpus_external_ids: corpus
                        .iter()
                        .map(|id| ExternalId::new((*id).to_owned()))
                        .collect(),
                    integrations_root: &integrations,
                    repo_root: self.root.path(),
                },
                &mut diagnostics,
            );
            String::from_utf8(diagnostics).expect("utf-8")
        }
    }

    fn report(discovery: DiscoveryStatus) -> RunReport {
        RunReport {
            reported: Vec::new(),
            read_failure: None,
            baseline_degradation: Degradation::None,
            finalised: true,
            dossiers: Vec::new(),
            discovery,
            keyed_read_budget_limited: false,
        }
    }

    fn imported(id: &str, outcome: ItemOutcome) -> ReportedItem {
        ReportedItem {
            planned: PlannedAction {
                id: id.to_owned(),
                state: SyncState::Unsynced,
                action: Action::CreateFromRemote,
            },
            outcome,
            validation: None,
        }
    }

    #[test]
    fn finalising_the_linear_catalogue() {
        let statuses = || {
            [
                DiscoveryStatus::Ran { found: 0 },
                DiscoveryStatus::TargetedPull { attempted: 1 },
                DiscoveryStatus::SkippedTargeted,
                DiscoveryStatus::Failed {
                    detail: "boom".to_owned(),
                },
                DiscoveryStatus::SkippedPushOnly,
            ]
        };
        let mut cases: Vec<(RunMode, DiscoveryStatus, bool)> = statuses()
            .into_iter()
            .map(|status| {
                let heals = !matches!(status, DiscoveryStatus::SkippedPushOnly);
                (RunMode::Apply, status, heals)
            })
            .collect();
        cases.extend(
            statuses()
                .into_iter()
                .map(|status| (RunMode::Preview, status, false)),
        );

        for (mode, status, heals) in cases {
            let repo = Repo::seeded(&legacy_catalogue());
            let before = repo.catalogue_text();

            let diagnostics = repo.finalise(
                Ok(Linear::default().team("t-eng", "ENG")),
                &report(status.clone()),
                mode,
                &[],
            );

            if heals {
                assert_ne!(
                    repo.catalogue_text(),
                    before,
                    "{mode:?} {status:?}"
                );
                assert!(diagnostics.starts_with("note:"), "{diagnostics}");
                assert!(diagnostics.contains("ENG"), "{diagnostics}");
            } else {
                assert_eq!(
                    repo.catalogue_text(),
                    before,
                    "{mode:?} {status:?}"
                );
                assert_eq!(diagnostics, "", "{mode:?} {status:?}");
            }
        }
    }

    #[test]
    fn unreturned_teams_warn_that_pulls_will_refetch_them() {
        let repo = Repo::seeded(&complete_catalogue());
        let mut linear = Linear::default().issue("OPS-1", "t-ops", "OPS");
        linear.unreturned = vec!["t-ops".to_owned()];

        let diagnostics = repo.finalise(
            Ok(linear),
            &report(DiscoveryStatus::Ran { found: 0 }),
            RunMode::Apply,
            &["OPS-1"],
        );

        assert!(
            diagnostics.contains("each pull will fetch OPS again"),
            "{diagnostics}"
        );
    }

    #[test]
    fn unconfirmed_identifiers_warn_naming_them() {
        let repo = Repo::seeded(&complete_catalogue());

        let diagnostics = repo.finalise(
            Ok(Linear::default()),
            &report(DiscoveryStatus::Ran { found: 0 }),
            RunMode::Apply,
            &["PROJ-12"],
        );

        assert!(
            diagnostics
                .contains("Linear does not recognise PROJ-12 as Linear issues"),
            "{diagnostics}"
        );
        assert!(diagnostics.contains("external_id"), "{diagnostics}");
    }

    #[test]
    fn a_failed_heal_fetch_warns_that_pulls_will_refetch() {
        let repo = Repo::seeded(&legacy_catalogue());
        let linear = Linear {
            fetch_fails: true,
            ..Linear::default()
        };

        let diagnostics = repo.finalise(
            Ok(linear),
            &report(DiscoveryStatus::Ran { found: 0 }),
            RunMode::Apply,
            &[],
        );

        assert!(
            diagnostics.contains("each pull will fetch ENG again"),
            "{diagnostics}"
        );
    }

    #[test]
    fn a_missing_credential_warns_only_when_healing_is_needed() {
        let unavailable = || {
            Err(FetchUnavailable {
                reason: "no Linear token".to_owned(),
            })
        };
        let needs_healing = Repo::seeded(&legacy_catalogue());
        let complete = Repo::seeded(&complete_catalogue());
        let apply = report(DiscoveryStatus::Ran { found: 0 });

        let warned =
            needs_healing.finalise(unavailable(), &apply, RunMode::Apply, &[]);
        let quiet = complete.finalise(
            unavailable(),
            &apply,
            RunMode::Apply,
            &["ENG-1"],
        );

        assert!(warned.contains("no Linear token"), "{warned}");
        assert_eq!(quiet, "");
    }

    #[test]
    fn a_damaged_catalogue_prints_the_restore_remedy_naming_the_entry() {
        let repo = Repo::seeded(&json!({
            "baseTeam": "t-eng",
            "teams": [{ "id": "t-bad", "key": 7 }]
        }));

        let diagnostics = repo.finalise(
            Ok(Linear::default()),
            &report(DiscoveryStatus::Ran { found: 0 }),
            RunMode::Apply,
            &[],
        );

        assert!(diagnostics.contains("E_CACHE_UNPARSEABLE"), "{diagnostics}");
        assert!(diagnostics.contains("t-bad"), "{diagnostics}");
        assert!(diagnostics.contains("version control"), "{diagnostics}");
    }

    #[test]
    fn an_applied_import_names_a_synced_team_and_a_failed_one_does_not() {
        let run_report = RunReport {
            reported: vec![
                imported("OPS-7", ItemOutcome::Applied),
                imported("QA-3", ItemOutcome::NotApplied),
            ],
            ..report(DiscoveryStatus::Ran { found: 2 })
        };
        let integrations = Path::new("/unused");
        let run = FinishedRun {
            report: &run_report,
            mode: RunMode::Apply,
            corpus_external_ids: Vec::new(),
            integrations_root: integrations,
            repo_root: integrations,
        };

        assert_eq!(
            run.applied_imports(),
            vec![ExternalId::new("OPS-7".to_owned())]
        );
    }

    #[test]
    fn a_heal_never_builds_the_fetch_when_nothing_needs_healing() {
        let repo = Repo::seeded(&complete_catalogue());
        let built = Cell::new(0);
        let factory = || {
            built.set(built.get() + 1);
            Err(FetchUnavailable {
                reason: String::new(),
            })
        };
        let finaliser = LinearCatalogueFinaliser::new(
            Arc::new(CatalogueHealing::new()),
            &factory,
        );
        let integrations = repo.root.path().join("integrations");

        finaliser.finalise(
            &FinishedRun {
                report: &report(DiscoveryStatus::Ran { found: 0 }),
                mode: RunMode::Apply,
                corpus_external_ids: vec![ExternalId::new("ENG-1".to_owned())],
                integrations_root: &integrations,
                repo_root: repo.root.path(),
            },
            &mut Vec::new(),
        );

        assert_eq!(built.get(), 0);
    }
}
