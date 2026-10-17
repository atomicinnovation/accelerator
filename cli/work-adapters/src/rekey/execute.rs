//! Retiring each run-plan entry in turn, recording every path a retirement
//! is about to write before it writes it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest as _;
use sha2::Sha256;
use work::rekey::entry::RekeyEntry;
use work::rekey::fingerprint::Staleness;
use work::rekey::outcome::NotRekeyed;
use work::rekey::run_plan::InProgressPlan;
use work::rekey::run_plan::RunPlan;
use work::rekey::write_set::Base;
use work::rekey::write_set::Digest;
use work::rekey::write_set::Recorded;
use work::rekey::write_set::StartState;
use work::rekey::write_set::WriteSet;
use work::retirement::plan_retirement;
use work::retirement::RetirementFailure;
use work::retirement::RetirementPlan;
use work::retirement::RetirementRefusal;

use crate::rekey::run_plan_store::RunPlanStore;
use crate::retirement::corpus_snapshot;
use crate::retirement::finish_retirement_with;
use crate::retirement::retirement_outstanding;
use crate::retirement::FinishFailure;
use crate::retirement::HookedFinishFailure;
use crate::retirement::RetirementLockGuard;
use crate::retirement::RetirementPorts;

/// The corpus a re-key retires over, with the repository root its write set
/// is recorded relative to and the state directory its recovery copies sit
/// under.
#[derive(Clone, Copy)]
pub struct RekeyCorpus<'a> {
    pub repo_root: &'a Path,
    pub state_dir: &'a Path,
    pub ports: RetirementPorts<'a>,
}

impl RekeyCorpus<'_> {
    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(self.repo_root)
            .unwrap_or(path)
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn state_of(&self, relative: &str) -> Result<StartState, NotRekeyed> {
        let path = self.repo_root.join(relative);
        let content = self.ports.files.reader.read(&path).map_err(|error| {
            NotRekeyed::StateUnwritable(format!(
                "cannot read {}: {error}",
                path.display()
            ))
        })?;
        Ok(content.map_or(StartState::Absent, |content| {
            StartState::Content(digest_of(&content))
        }))
    }

    /// Every path `plan` touches, repo-relative, with the sync baseline when
    /// an integration is configured.
    fn touched_by(&self, plan: &RetirementPlan) -> Vec<String> {
        let mut paths: Vec<String> = [&plan.from, &plan.to]
            .into_iter()
            .chain(plan.rewrites.iter().map(|rewrite| &rewrite.path))
            .map(|path| self.relative(path))
            .collect();
        paths.extend(
            self.ports
                .baseline
                .map(|baseline| self.relative(baseline.path())),
        );
        paths
    }

    fn recorded(
        &self,
        paths: impl IntoIterator<Item = String>,
        base: Option<&Base>,
    ) -> Result<Vec<(String, Recorded)>, NotRekeyed> {
        paths
            .into_iter()
            .map(|path| {
                let state = self.state_of(&path)?;
                Ok((
                    path,
                    Recorded {
                        state,
                        base: base.cloned(),
                    },
                ))
            })
            .collect()
    }

    fn recovery_location(&self, entry: &RekeyEntry) -> PathBuf {
        self.state_dir.join(entry.retirement().recovery_dir())
    }

    fn stopped(
        &self,
        entry: &RekeyEntry,
        failure: FinishFailure,
    ) -> NotRekeyed {
        match failure {
            FinishFailure::Refused(refusal) => NotRekeyed::EntryRefused {
                entry: entry.clone(),
                refusal,
            },
            FinishFailure::Failed(RetirementFailure::RolledBack { cause }) => {
                NotRekeyed::EntryRolledBack {
                    entry: entry.clone(),
                    cause: Box::new(cause),
                }
            }
            FinishFailure::Failed(RetirementFailure::RestoreIncomplete {
                ..
            }) => NotRekeyed::RestoreIncomplete {
                entry: entry.clone(),
                recovery: self.recovery_location(entry),
            },
        }
    }
}

fn digest_of(content: &str) -> Digest {
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(content.as_bytes()) {
        let _ = write!(hex, "{byte:02x}");
    }
    Digest::new(hex)
}

/// What stops a run before it writes: an entry whose target is held, whose
/// path is occupied, or whose item has gone.
fn refused_before_writing(
    entry: &RekeyEntry,
    mover: PathBuf,
    refusal: RetirementRefusal,
) -> NotRekeyed {
    match refusal {
        RetirementRefusal::IdTaken { holder, .. }
        | RetirementRefusal::KeyLinked { holder } => NotRekeyed::Collision {
            first: mover,
            second: holder,
            target: entry.target.id().to_owned(),
        },
        RetirementRefusal::TargetExists(path) => NotRekeyed::TargetExists(path),
        RetirementRefusal::ItemNotFound(_) => {
            NotRekeyed::PreviewStale(Staleness::ItemsChanged(vec![mover]))
        }
    }
}

/// Every path the run may write, each with its state now, planned against
/// one walk of the corpus.
///
/// # Errors
///
/// [`NotRekeyed`] when an entry's retirement refuses to plan, or a path
/// cannot be read.
pub fn write_set(
    entries: &[RekeyEntry],
    corpus: &RekeyCorpus<'_>,
    base: Option<&Base>,
) -> Result<WriteSet, NotRekeyed> {
    let snapshot = corpus_snapshot(&corpus.ports).map_err(|failure| {
        NotRekeyed::StateUnwritable(format!("{failure:?}"))
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let mover = snapshot
            .items
            .iter()
            .find(|item| item.id.eq_ignore_ascii_case(&entry.old))
            .map_or_else(
                || PathBuf::from(&entry.old),
                |item| item.path.clone(),
            );
        let plan = plan_retirement(
            &entry.retirement(),
            corpus.ports.layout.work_dir,
            &snapshot.items,
            &snapshot.files,
        )
        .map_err(|refusal| refused_before_writing(entry, mover, refusal))?;
        paths.extend(corpus.touched_by(&plan));
    }
    paths.sort();
    paths.dedup();
    Ok(WriteSet::default().extended_with(corpus.recorded(paths, base)?))
}

/// The state of every write-set path now.
///
/// # Errors
///
/// [`NotRekeyed::StateUnwritable`] when a path cannot be read.
pub fn current_states(
    write_set: &WriteSet,
    corpus: &RekeyCorpus<'_>,
) -> Result<BTreeMap<String, StartState>, NotRekeyed> {
    write_set
        .iter()
        .map(|(path, _)| Ok((path.clone(), corpus.state_of(path)?)))
        .collect()
}

/// Every entry of `plan` not yet retired, in order.
///
/// # Errors
///
/// [`NotRekeyed`] when an entry's state cannot be read.
pub fn outstanding_entries(
    plan: &InProgressPlan,
    corpus: &RekeyCorpus<'_>,
) -> Result<Vec<RekeyEntry>, NotRekeyed> {
    let mut outstanding = Vec::new();
    for entry in plan.entries() {
        if retirement_outstanding(&entry.retirement(), &corpus.ports)
            .map_err(|failure| corpus.stopped(entry, failure))?
        {
            outstanding.push(entry.clone());
        }
    }
    Ok(outstanding)
}

/// Retires every entry not yet retired, in order, reporting each through
/// `applied` with its new path.
///
/// Before each retirement writes, every path it touches that the plan has
/// not recorded is added, at its state now and against `base`, and the plan
/// saved, so a discard can see everything the run has written.
///
/// # Errors
///
/// [`NotRekeyed`] when an entry is refused, rolls back twice or leaves a
/// restore incomplete, or the plan cannot be saved; every earlier entry
/// stays retired.
pub fn execute(
    plan: InProgressPlan,
    base: Option<&Base>,
    store: &dyn RunPlanStore,
    corpus: &RekeyCorpus<'_>,
    lock: &RetirementLockGuard,
    applied: &dyn Fn(&RekeyEntry, &str),
) -> Result<InProgressPlan, NotRekeyed> {
    let mut plan = plan;
    for entry in plan.entries().to_vec() {
        let retirement = entry.retirement();
        let outstanding = retirement_outstanding(&retirement, &corpus.ports)
            .map_err(|failure| corpus.stopped(&entry, failure))?;
        if !outstanding {
            continue;
        }
        let mut target = None;
        finish_retirement_with(
            &retirement,
            &corpus.ports,
            lock,
            &mut |about_to_apply| {
                let unrecorded: Vec<String> = corpus
                    .touched_by(about_to_apply)
                    .into_iter()
                    .filter(|path| plan.write_set().recorded(path).is_none())
                    .collect();
                if !unrecorded.is_empty() {
                    let additions = corpus.recorded(unrecorded, base)?;
                    plan = plan.clone().extended_with(additions);
                    store.save(&RunPlan::InProgress(plan.clone()))?;
                }
                target = Some(corpus.relative(&about_to_apply.to));
                Ok(())
            },
        )
        .map_err(|failure| match failure {
            HookedFinishFailure::Retirement(failure) => {
                corpus.stopped(&entry, failure)
            }
            HookedFinishFailure::BeforeApply(stopped) => stopped,
        })?;
        applied(&entry, target.as_deref().unwrap_or_default());
    }
    Ok(plan)
}
