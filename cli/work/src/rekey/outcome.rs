//! Why a re-key stopped, with the code and exit code skills branch on and the
//! remedy a person acts on.

use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use crate::draft_id::DraftIdExhausted;
use crate::rekey::entry::RekeyEntry;
use crate::rekey::fingerprint::Staleness;
use crate::rekey::run_plan::Unreadability;
use crate::rekey::run_plan::UnreadablePlan;
use crate::rekey::write_set::Base;
use crate::rekey::write_set::Unreverted;
use crate::retirement::relative_to;
use crate::retirement::RetirementCause;
use crate::retirement::RetirementRefusal;

/// The process holding the identity-run lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub pid: u32,
    pub command: Option<String>,
}

/// Work an earlier `work sync` left unfinished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncBlocker {
    Retirement { old: String, new: String },
    Promotion { draft: String },
    UnreadableRecord(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotRekeyed {
    /// An interrupted run; a running one is [`Self::IdentityRunBusy`].
    InProgress,
    PlanUnreadable(UnreadablePlan),
    NotReverted(Unreverted),
    PartlyApplied {
        items: Vec<PathBuf>,
        recoveries: Vec<PathBuf>,
    },
    SyncUnfinished(Vec<SyncBlocker>),
    PendingPush(Vec<PathBuf>),
    Collision {
        first: PathBuf,
        second: PathBuf,
        target: String,
    },
    Unclassifiable(PathBuf),
    FreshNumbersExhausted {
        needed: u64,
        remaining: u64,
        pattern: String,
    },
    KeptNumberAboveCap {
        item: PathBuf,
        number: u64,
        pattern: String,
    },
    TargetExists(PathBuf),
    PreviewStale(Staleness),
    DirtyWorkingCopy(Vec<String>),
    WorkingCopyUnprobeable(String),
    EntryRefused {
        entry: RekeyEntry,
        refusal: RetirementRefusal,
    },
    EntryRolledBack {
        entry: RekeyEntry,
        cause: Box<RetirementCause>,
    },
    RestoreIncomplete {
        entry: RekeyEntry,
        recovery: PathBuf,
    },
    StateUnwritable(String),
    IdentityRunLockFailed(String),
    CorpusInaccessible {
        path: PathBuf,
        detail: String,
    },
    IdentityRunBusy(Option<Holder>),
    TrackerUnreachable(String),
    DraftIdExhausted(DraftIdExhausted),
}

const INTERNAL_ERROR: u8 = 1;
const AWAITING_HUMAN: u8 = 4;
const RETRYABLE: u8 = 70;

const RUN_REKEY: &str = "Run `accelerator work rekey` to finish it";
const DISCARD_LISTS_PATHS: &str = "to undo it instead, run `accelerator work \
     rekey --discard`, which lists the paths to restore";

impl NotRekeyed {
    /// The refusal naming every path relative to `root`, for display.
    #[must_use]
    pub fn relative_to(self, root: &Path) -> Self {
        let under = |path: PathBuf| relative_to(path, root);
        let all_under =
            |paths: Vec<PathBuf>| paths.into_iter().map(under).collect();
        match self {
            Self::PlanUnreadable(UnreadablePlan { path, cause }) => {
                Self::PlanUnreadable(UnreadablePlan {
                    path: under(path),
                    cause,
                })
            }
            Self::PartlyApplied { items, recoveries } => Self::PartlyApplied {
                items: all_under(items),
                recoveries: all_under(recoveries),
            },
            Self::SyncUnfinished(blockers) => Self::SyncUnfinished(
                blockers
                    .into_iter()
                    .map(|blocker| match blocker {
                        SyncBlocker::UnreadableRecord(path) => {
                            SyncBlocker::UnreadableRecord(under(path))
                        }
                        other => other,
                    })
                    .collect(),
            ),
            Self::PendingPush(paths) => Self::PendingPush(all_under(paths)),
            Self::Collision {
                first,
                second,
                target,
            } => Self::Collision {
                first: under(first),
                second: under(second),
                target,
            },
            Self::Unclassifiable(path) => Self::Unclassifiable(under(path)),
            Self::KeptNumberAboveCap {
                item,
                number,
                pattern,
            } => Self::KeptNumberAboveCap {
                item: under(item),
                number,
                pattern,
            },
            Self::TargetExists(path) => Self::TargetExists(under(path)),
            Self::PreviewStale(Staleness::ItemsChanged(paths)) => {
                Self::PreviewStale(Staleness::ItemsChanged(all_under(paths)))
            }
            Self::EntryRefused { entry, refusal } => Self::EntryRefused {
                entry,
                refusal: refusal.relative_to(root),
            },
            Self::EntryRolledBack { entry, cause } => Self::EntryRolledBack {
                entry,
                cause: Box::new(cause.relative_to(root)),
            },
            Self::RestoreIncomplete { entry, recovery } => {
                Self::RestoreIncomplete {
                    entry,
                    recovery: under(recovery),
                }
            }
            Self::CorpusInaccessible { path, detail } => {
                Self::CorpusInaccessible {
                    path: under(path),
                    detail,
                }
            }
            other => other,
        }
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InProgress => "E_REKEY_IN_PROGRESS",
            Self::PlanUnreadable(_) => "E_REKEY_PLAN_UNREADABLE",
            Self::NotReverted(_) => "E_REKEY_NOT_REVERTED",
            Self::PartlyApplied { .. } => "E_REKEY_PARTLY_APPLIED",
            Self::SyncUnfinished(_) => "E_REKEY_SYNC_UNFINISHED",
            Self::PendingPush(_) => "E_REKEY_PENDING_PUSH",
            Self::Collision { .. } => "E_REKEY_ID_COLLISION",
            Self::Unclassifiable(_) => "E_REKEY_UNCLASSIFIABLE_ID",
            Self::FreshNumbersExhausted { .. }
            | Self::KeptNumberAboveCap { .. } => {
                "E_REKEY_NUMBER_SPACE_EXHAUSTED"
            }
            Self::TargetExists(_) => "E_REKEY_TARGET_EXISTS",
            Self::PreviewStale(_) => "E_REKEY_PREVIEW_STALE",
            Self::DirtyWorkingCopy(_) => "E_REKEY_DIRTY_WORKING_COPY",
            Self::WorkingCopyUnprobeable(_) => {
                "E_REKEY_WORKING_COPY_UNPROBEABLE"
            }
            Self::EntryRefused { .. } => "E_REKEY_ENTRY_REFUSED",
            Self::EntryRolledBack { .. } => "E_REKEY_ENTRY_ROLLED_BACK",
            Self::RestoreIncomplete { .. } => "E_REKEY_RESTORE_INCOMPLETE",
            Self::StateUnwritable(_) => "E_REKEY_STATE_UNWRITABLE",
            Self::IdentityRunLockFailed(_) => {
                "E_REKEY_IDENTITY_RUN_LOCK_FAILED"
            }
            Self::CorpusInaccessible { .. } => "E_REKEY_CORPUS_INACCESSIBLE",
            Self::IdentityRunBusy(_) => "E_IDENTITY_RUN_BUSY",
            Self::TrackerUnreachable(_) => "E_REKEY_TRACKER_UNREACHABLE",
            Self::DraftIdExhausted(_) => "E_DRAFT_ID_EXHAUSTED",
        }
    }

    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::StateUnwritable(_)
            | Self::IdentityRunLockFailed(_)
            | Self::CorpusInaccessible { .. }
            | Self::IdentityRunBusy(_)
            | Self::DraftIdExhausted(_) => INTERNAL_ERROR,
            Self::TrackerUnreachable(_) => RETRYABLE,
            _ => AWAITING_HUMAN,
        }
    }

    fn what(&self) -> String {
        match self {
            Self::InProgress => "a re-key run is in progress".to_owned(),
            Self::PlanUnreadable(plan) => unreadable(plan),
            Self::NotReverted(_) => "the run's write set has changed since \
                 it started"
                .to_owned(),
            Self::PartlyApplied { .. } => {
                "the corpus shows a half-applied re-key".to_owned()
            }
            Self::SyncUnfinished(_) => {
                "an earlier `accelerator work sync` left work unfinished"
                    .to_owned()
            }
            Self::PendingPush(_) => {
                "items to be re-keyed have pushes pending".to_owned()
            }
            Self::Collision {
                first,
                second,
                target,
            } => format!(
                "{target} is claimed by both {} and {}",
                first.display(),
                second.display()
            ),
            Self::Unclassifiable(item) => format!(
                "{} has an `id` of no numeric or tracker shape",
                item.display()
            ),
            Self::FreshNumbersExhausted { pattern, .. } => format!(
                "re-keying needs more fresh numbers than `{pattern}` has left"
            ),
            Self::KeptNumberAboveCap {
                item,
                number,
                pattern,
            } => format!(
                "{} keeps number {number}, wider than `{pattern}` allows",
                item.display()
            ),
            Self::TargetExists(path) => {
                format!("{} occupies a re-key target path", path.display())
            }
            Self::PreviewStale(Staleness::SchemeChanged) => {
                "`work.id_pattern` or `work.key` changed since the preview"
                    .to_owned()
            }
            Self::PreviewStale(Staleness::ItemsChanged(_)) => {
                "these items changed since the preview".to_owned()
            }
            Self::DirtyWorkingCopy(_) => {
                "the working copy has uncommitted changes".to_owned()
            }
            Self::WorkingCopyUnprobeable(detail) => {
                format!("the working copy cannot be checked ({detail})")
            }
            Self::EntryRefused { entry, refusal } => {
                refusal.message(&entry.retirement())
            }
            Self::EntryRolledBack { entry, cause } => format!(
                "re-keying {} as {} was rolled back twice: {cause}",
                entry.old,
                entry.target.id()
            ),
            Self::RestoreIncomplete { entry, .. } => format!(
                "re-keying {} as {} could not restore every path",
                entry.old,
                entry.target.id()
            ),
            Self::StateUnwritable(detail) => {
                format!("the run plan cannot be saved or removed ({detail})")
            }
            Self::IdentityRunLockFailed(detail) => {
                format!("the identity-run lock cannot be taken ({detail})")
            }
            Self::CorpusInaccessible { path, detail } => format!(
                "{} cannot be read or written ({detail})",
                path.display()
            ),
            Self::IdentityRunBusy(holder) => still_running(holder.as_ref()),
            Self::TrackerUnreachable(detail) => {
                format!("the tracker could not be read ({detail})")
            }
            Self::DraftIdExhausted(exhausted) => {
                format!("no free draft ID after {} draws", exhausted.attempts)
            }
        }
    }

    fn remedy(&self) -> String {
        match self {
            Self::InProgress => format!("{RUN_REKEY}; {DISCARD_LISTS_PATHS}"),
            Self::IdentityRunBusy(_) => "Re-run once it finishes".to_owned(),
            Self::PlanUnreadable(UnreadablePlan {
                cause: Unreadability::NewerSchema(schema),
                ..
            }) => format!(
                "Upgrade accelerator to a version that reads schema {schema}, \
                 then run `accelerator work rekey` or `accelerator work rekey \
                 --discard` there"
            ),
            Self::PlanUnreadable(UnreadablePlan {
                cause: Unreadability::Malformed(_),
                ..
            }) => "The run's state cannot be verified; `accelerator work \
                 rekey --discard` prints the evidence of any half-applied \
                 entry; if none was applied, or after restoring one, \
                 `ACCELERATOR_FORCE=1 accelerator work rekey --discard` sets \
                 the plan aside"
                .to_owned(),
            Self::NotReverted(_) => format!(
                "{RUN_REKEY}; or restore each path listed under \"restore\" \
                 from the revision printed beside it and delete those under \
                 \"remove\", then re-run `accelerator work rekey --discard`; \
                 a path edited on purpose since the run started needs \
                 `ACCELERATOR_FORCE=1 accelerator work rekey --discard`"
            ),
            Self::PartlyApplied { .. } => "Either an earlier re-key was \
                 interrupted (restore every corpus root to the revision \
                 before the commit that introduced the named alias, or \
                 finish the named entry by hand: keep the new file, remove \
                 the old and, for a synced item, move its `last-sync.json` \
                 entry from the old ID to the new) or an ID was created onto \
                 an existing alias (rename one item); restore from a named \
                 `RESTORE-PENDING` directory before removing it"
                .to_owned(),
            Self::SyncUnfinished(_) => "Run `accelerator work sync`".to_owned(),
            Self::PendingPush(_) => {
                "Run `accelerator work sync` to settle the push".to_owned()
            }
            Self::Collision { .. } => {
                "Rename or merge one of the named items".to_owned()
            }
            Self::Unclassifiable(_) => {
                "Give the item a numeric or tracker-shaped `id`".to_owned()
            }
            Self::FreshNumbersExhausted {
                needed,
                remaining,
                pattern,
            } => format!(
                "{needed} fresh numbers are needed and {remaining} remain \
                 under `{pattern}`; widen `work.id_pattern` or archive \
                 completed items"
            ),
            Self::KeptNumberAboveCap { .. } => {
                "Widen `work.id_pattern`".to_owned()
            }
            Self::TargetExists(_) => "Move the named file".to_owned(),
            Self::PreviewStale(_) => {
                "Re-run `accelerator work rekey --preview`".to_owned()
            }
            Self::DirtyWorkingCopy(_) => "Commit or revert the named paths, \
                 or set `ACCELERATOR_FORCE=1` for this one invocation"
                .to_owned(),
            Self::WorkingCopyUnprobeable(_) => "Fix the repository, or set \
                 `ACCELERATOR_FORCE=1` knowing nothing can undo the run"
                .to_owned(),
            Self::EntryRefused { .. } => format!(
                "Resolve the named conflict and re-run `accelerator work \
                 rekey`; {DISCARD_LISTS_PATHS}"
            ),
            Self::EntryRolledBack { .. } => {
                "Re-run `accelerator work rekey` to resume".to_owned()
            }
            Self::RestoreIncomplete { recovery, .. } => format!(
                "Restore from {}, then re-run `accelerator work rekey`",
                recovery.display()
            ),
            Self::StateUnwritable(_) => "Check permissions on \
                 `.accelerator/state/rekey/` and re-run"
                .to_owned(),
            Self::IdentityRunLockFailed(_) => "Check permissions on \
                 `.accelerator/state/locks/` and re-run"
                .to_owned(),
            Self::CorpusInaccessible { path, .. } => {
                format!("Check permissions on {} and re-run", path.display())
            }
            Self::TrackerUnreachable(_) => {
                "Retry when the tracker is reachable".to_owned()
            }
            Self::DraftIdExhausted(_) => "Re-run the command".to_owned(),
        }
    }

    /// The paths and records a person needs in hand to act on the remedy.
    fn listing(&self) -> Vec<String> {
        let paths = |paths: &[PathBuf]| -> Vec<String> {
            paths
                .iter()
                .map(|path| format!("  {}", path.display()))
                .collect()
        };
        match self {
            Self::NotReverted(unreverted) => unreverted_listing(unreverted),
            Self::PartlyApplied { items, recoveries } => items
                .iter()
                .map(|item| format!("  item: {}", item.display()))
                .chain(recoveries.iter().map(|recovery| {
                    format!("  recovery: {}", recovery.display())
                }))
                .collect(),
            Self::SyncUnfinished(blockers) => blockers
                .iter()
                .map(|blocker| format!("  {}", blocker_line(blocker)))
                .collect(),
            Self::PendingPush(items)
            | Self::PreviewStale(Staleness::ItemsChanged(items)) => {
                paths(items)
            }
            Self::DirtyWorkingCopy(dirty) => {
                dirty.iter().map(|path| format!("  {path}")).collect()
            }
            _ => Vec::new(),
        }
    }
}

fn unreadable(UnreadablePlan { path, cause }: &UnreadablePlan) -> String {
    match cause {
        Unreadability::NewerSchema(schema) => format!(
            "the run plan {} has schema {schema}, newer than this accelerator \
             reads",
            path.display()
        ),
        Unreadability::Malformed(detail) => {
            format!("the run plan {} is malformed ({detail})", path.display())
        }
    }
}

fn still_running(holder: Option<&Holder>) -> String {
    match holder {
        Some(Holder {
            pid,
            command: Some(command),
        }) => {
            format!("`accelerator work {command}` (pid {pid}) is still running")
        }
        Some(Holder { pid, command: None }) => {
            format!("another accelerator command (pid {pid}) is still running")
        }
        None => "another accelerator command is still running".to_owned(),
    }
}

fn blocker_line(blocker: &SyncBlocker) -> String {
    match blocker {
        SyncBlocker::Retirement { old, new } => {
            format!("retiring {old} as {new}")
        }
        SyncBlocker::Promotion { draft } => format!("promoting {draft}"),
        SyncBlocker::UnreadableRecord(path) => {
            format!("unreadable record {}", path.display())
        }
    }
}

/// The paths to restore, under the revision each was recorded against, and
/// the paths to remove, one indented line each.
#[must_use]
pub fn unreverted_listing(unreverted: &Unreverted) -> Vec<String> {
    let mut lines = Vec::new();
    for (base, paths) in &unreverted.restore {
        lines.push(format!("  {}:", restore_heading(base.as_ref())));
        lines.extend(paths.iter().map(|path| format!("    {path}")));
    }
    if !unreverted.remove.is_empty() {
        lines.push("  remove:".to_owned());
        lines
            .extend(unreverted.remove.iter().map(|path| format!("    {path}")));
    }
    lines
}

fn restore_heading(base: Option<&Base>) -> String {
    match base.map(Base::commits) {
        Some([commit]) => format!("restore from {commit}"),
        Some(parents) if !parents.is_empty() => {
            format!("restore from the parents {}", parents.join(", "))
        }
        _ => "restore to its content when the run first touched it".to_owned(),
    }
}

impl fmt::Display for NotRekeyed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}. {}.",
            self.code(),
            self.what(),
            self.remedy()
        )?;
        for line in self.listing() {
            write!(formatter, "\n{line}")?;
        }
        Ok(())
    }
}

impl std::error::Error for NotRekeyed {}

#[cfg(test)]
mod tests {
    // DSL pattern strings such as "{number:04d}" are literal token markers,
    // not format args.
    #![allow(clippy::literal_string_with_formatting_args)]

    use std::collections::BTreeMap;
    use std::collections::HashSet;
    use std::mem::discriminant;
    use std::mem::Discriminant;
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::StoreError;

    use super::Holder;
    use super::NotRekeyed;
    use super::SyncBlocker;
    use crate::draft_id::DraftIdExhausted;
    use crate::identity::IdentityField;
    use crate::rekey::entry::RekeyEntry;
    use crate::rekey::entry::RekeyTarget;
    use crate::rekey::fingerprint::Staleness;
    use crate::rekey::run_plan::Unreadability;
    use crate::rekey::run_plan::UnreadablePlan;
    use crate::rekey::write_set::Base;
    use crate::rekey::write_set::Unreverted;
    use crate::retirement::RetirementCause;
    use crate::retirement::RetirementCauseKind;
    use crate::retirement::RetirementRefusal;

    const PLAN: &str = ".accelerator/state/rekey/run-plan.json";

    fn path(raw: &str) -> PathBuf {
        PathBuf::from(raw)
    }

    fn entry() -> RekeyEntry {
        RekeyEntry {
            old: "0042".to_owned(),
            target: RekeyTarget::Renumbered("ACC-0042".to_owned()),
        }
    }

    fn holder(command: Option<&str>) -> Holder {
        Holder {
            pid: 4242,
            command: command.map(str::to_owned),
        }
    }

    fn unreadable(cause: Unreadability) -> NotRekeyed {
        NotRekeyed::PlanUnreadable(UnreadablePlan {
            path: path(PLAN),
            cause,
        })
    }

    fn malformed() -> NotRekeyed {
        unreadable(Unreadability::Malformed("expected value".to_owned()))
    }

    /// Fails to compile when a variant is added, so the fixtures below must
    /// gain it.
    const fn covered(outcome: &NotRekeyed) {
        match outcome {
            NotRekeyed::InProgress
            | NotRekeyed::PlanUnreadable(_)
            | NotRekeyed::NotReverted(_)
            | NotRekeyed::PartlyApplied { .. }
            | NotRekeyed::SyncUnfinished(_)
            | NotRekeyed::PendingPush(_)
            | NotRekeyed::Collision { .. }
            | NotRekeyed::Unclassifiable(_)
            | NotRekeyed::FreshNumbersExhausted { .. }
            | NotRekeyed::KeptNumberAboveCap { .. }
            | NotRekeyed::TargetExists(_)
            | NotRekeyed::PreviewStale(_)
            | NotRekeyed::DirtyWorkingCopy(_)
            | NotRekeyed::WorkingCopyUnprobeable(_)
            | NotRekeyed::EntryRefused { .. }
            | NotRekeyed::EntryRolledBack { .. }
            | NotRekeyed::RestoreIncomplete { .. }
            | NotRekeyed::StateUnwritable(_)
            | NotRekeyed::IdentityRunLockFailed(_)
            | NotRekeyed::CorpusInaccessible { .. }
            | NotRekeyed::IdentityRunBusy(_)
            | NotRekeyed::TrackerUnreachable(_)
            | NotRekeyed::DraftIdExhausted(_) => {}
        }
    }

    /// Every remedy form, each with a fragment of the remedy it must print.
    fn every_remedy() -> Vec<(NotRekeyed, &'static str)> {
        [plan_remedies(), corpus_remedies(), run_remedies()].concat()
    }

    fn plan_remedies() -> Vec<(NotRekeyed, &'static str)> {
        vec![
            (
                NotRekeyed::InProgress,
                "Run `accelerator work rekey` to finish it; to undo it \
                 instead, run `accelerator work rekey --discard`, which \
                 lists the paths to restore",
            ),
            (
                NotRekeyed::IdentityRunBusy(Some(holder(Some("rekey")))),
                "`accelerator work rekey` (pid 4242) is still running. Re-run \
                 once it finishes",
            ),
            (
                unreadable(Unreadability::NewerSchema(2)),
                "Upgrade accelerator to a version that reads schema 2, then \
                 run `accelerator work rekey` or `accelerator work rekey \
                 --discard` there",
            ),
            (
                malformed(),
                "The run's state cannot be verified; `accelerator work rekey \
                 --discard` prints the evidence of any half-applied entry; if \
                 none was applied, or after restoring one, \
                 `ACCELERATOR_FORCE=1 accelerator work rekey --discard` sets \
                 the plan aside",
            ),
            (
                NotRekeyed::NotReverted(Unreverted {
                    restore: BTreeMap::from([
                        (
                            Some(Base::new(vec!["c1".to_owned()])),
                            vec!["meta/notes/a.md".to_owned()],
                        ),
                        (
                            Some(Base::new(vec![
                                "p1".to_owned(),
                                "p2".to_owned(),
                            ])),
                            vec!["meta/notes/b.md".to_owned()],
                        ),
                        (None, vec!["meta/notes/c.md".to_owned()]),
                    ]),
                    remove: vec!["meta/work/ACC-0042-search.md".to_owned()],
                }),
                "Run `accelerator work rekey` to finish it; or restore each \
                 path listed under \"restore\" from the revision printed \
                 beside it and delete those under \"remove\", then re-run \
                 `accelerator work rekey --discard`; a path edited on \
                 purpose since the run started needs `ACCELERATOR_FORCE=1 \
                 accelerator work rekey --discard`",
            ),
        ]
    }

    fn corpus_remedies() -> Vec<(NotRekeyed, &'static str)> {
        vec![
            (
                NotRekeyed::PartlyApplied {
                    items: vec![path("meta/work/0042-search.md")],
                    recoveries: vec![path("retirement-recovery/0042--x")],
                },
                "restore every corpus root to the revision before the commit \
                 that introduced the named alias, or finish the named entry \
                 by hand",
            ),
            (
                NotRekeyed::SyncUnfinished(vec![
                    SyncBlocker::Retirement {
                        old: "0042".to_owned(),
                        new: "PP-760".to_owned(),
                    },
                    SyncBlocker::Promotion {
                        draft: "draft-k3x9mq".to_owned(),
                    },
                    SyncBlocker::UnreadableRecord(path("records/x.json")),
                ]),
                "Run `accelerator work sync`",
            ),
            (
                NotRekeyed::PendingPush(vec![path("meta/work/drafts/d.md")]),
                "Run `accelerator work sync` to settle the push",
            ),
            (
                NotRekeyed::Collision {
                    first: path("meta/work/0042-a.md"),
                    second: path("meta/work/ACC-0042-b.md"),
                    target: "ACC-0042".to_owned(),
                },
                "Rename or merge one of the named items",
            ),
            (
                NotRekeyed::Unclassifiable(path("meta/work/notes.md")),
                "Give the item a numeric or tracker-shaped `id`",
            ),
            (
                NotRekeyed::FreshNumbersExhausted {
                    needed: 3,
                    remaining: 1,
                    pattern: "{number:04d}".to_owned(),
                },
                "3 fresh numbers are needed and 1 remain under \
                 `{number:04d}`; widen `work.id_pattern` or archive \
                 completed items",
            ),
            (
                NotRekeyed::KeptNumberAboveCap {
                    item: path("meta/work/PP-12345-big.md"),
                    number: 12_345,
                    pattern: "{number:04d}".to_owned(),
                },
                "meta/work/PP-12345-big.md keeps number 12345, wider than \
                 `{number:04d}` allows. Widen `work.id_pattern`",
            ),
            (
                NotRekeyed::TargetExists(path("meta/work/ACC-0042-x.md")),
                "Move the named file",
            ),
            (
                NotRekeyed::PreviewStale(Staleness::SchemeChanged),
                "Re-run `accelerator work rekey --preview`",
            ),
        ]
    }

    fn run_remedies() -> Vec<(NotRekeyed, &'static str)> {
        vec![
            (
                NotRekeyed::PreviewStale(Staleness::ItemsChanged(vec![path(
                    "meta/work/0042-a.md",
                )])),
                "Re-run `accelerator work rekey --preview`",
            ),
            (
                NotRekeyed::DirtyWorkingCopy(
                    vec!["meta/work/0042-a.md".into()],
                ),
                "Commit or revert the named paths, or set \
                 `ACCELERATOR_FORCE=1` for this one invocation",
            ),
            (
                NotRekeyed::WorkingCopyUnprobeable("no repository".to_owned()),
                "Fix the repository, or set `ACCELERATOR_FORCE=1` knowing \
                 nothing can undo the run",
            ),
            (
                NotRekeyed::EntryRefused {
                    entry: entry(),
                    refusal: RetirementRefusal::IdTaken {
                        holder: path("meta/work/ACC-0042-x.md"),
                        field: IdentityField::Id,
                    },
                },
                "Resolve the named conflict and re-run `accelerator work \
                 rekey`; to undo it instead, run `accelerator work rekey \
                 --discard`, which lists the paths to restore",
            ),
            (
                NotRekeyed::EntryRolledBack {
                    entry: entry(),
                    cause: Box::new(RetirementCause {
                        path: path("meta/work/0042-a.md"),
                        kind: RetirementCauseKind::Store(StoreError::Io {
                            path: "meta/work/0042-a.md".to_owned(),
                            detail: "disk full".to_owned(),
                        }),
                    }),
                },
                "Re-run `accelerator work rekey` to resume",
            ),
            (
                NotRekeyed::RestoreIncomplete {
                    entry: entry(),
                    recovery: path("retirement-recovery/0042--ACC-0042"),
                },
                "Restore from retirement-recovery/0042--ACC-0042, then re-run \
                 `accelerator work rekey`",
            ),
            (
                NotRekeyed::StateUnwritable("permission denied".to_owned()),
                "Check permissions on `.accelerator/state/rekey/` and re-run",
            ),
            (
                NotRekeyed::IdentityRunLockFailed(
                    "permission denied".to_owned(),
                ),
                "Check permissions on `.accelerator/state/locks/` and re-run",
            ),
            (
                NotRekeyed::CorpusInaccessible {
                    path: path("meta/work"),
                    detail: "permission denied".to_owned(),
                },
                "Check permissions on meta/work and re-run",
            ),
            (
                NotRekeyed::IdentityRunBusy(Some(holder(Some("sync")))),
                "`accelerator work sync` (pid 4242) is still running. Re-run \
                 once it finishes",
            ),
            (
                NotRekeyed::IdentityRunBusy(None),
                "another accelerator command is still running. Re-run once it \
                 finishes",
            ),
            (
                NotRekeyed::TrackerUnreachable("connection refused".into()),
                "Retry when the tracker is reachable",
            ),
            (
                NotRekeyed::DraftIdExhausted(DraftIdExhausted { attempts: 16 }),
                "no free draft ID after 16 draws. Re-run the command",
            ),
        ]
    }

    #[test]
    fn every_outcome_has_a_distinct_code_except_the_two_number_space_cases() {
        let outcomes: Vec<NotRekeyed> = every_remedy()
            .into_iter()
            .map(|(outcome, _)| outcome)
            .collect();
        let mut by_code: BTreeMap<&str, HashSet<Discriminant<NotRekeyed>>> =
            BTreeMap::new();
        for outcome in &outcomes {
            covered(outcome);
            by_code
                .entry(outcome.code())
                .or_default()
                .insert(discriminant(outcome));
        }
        for (code, variants) in &by_code {
            let expected = if *code == "E_REKEY_NUMBER_SPACE_EXHAUSTED" {
                2
            } else {
                1
            };
            assert_eq!(variants.len(), expected, "{code}");
            assert!(code.starts_with("E_"), "{code}");
        }
        assert_eq!(by_code.len(), 22);
    }

    #[test]
    fn every_path_a_refusal_names_renders_relative_to_the_root() {
        let root = Path::new("/repo");
        let at = |relative: &str| root.join(relative);
        let item = || at("meta/work/0042-a.md");
        let refusals = vec![
            NotRekeyed::PlanUnreadable(UnreadablePlan {
                path: at(".accelerator/state/rekey/run-plan.json"),
                cause: Unreadability::Malformed("bad".to_owned()),
            }),
            NotRekeyed::PartlyApplied {
                items: vec![item()],
                recoveries: vec![at(
                    ".accelerator/state/retirement-recovery/r",
                )],
            },
            NotRekeyed::SyncUnfinished(vec![SyncBlocker::UnreadableRecord(
                at(".accelerator/state/retirements"),
            )]),
            NotRekeyed::PendingPush(vec![item()]),
            NotRekeyed::Collision {
                first: item(),
                second: at("meta/work/ACC-0042-b.md"),
                target: "ACC-0042".to_owned(),
            },
            NotRekeyed::Unclassifiable(item()),
            NotRekeyed::KeptNumberAboveCap {
                item: item(),
                number: 12_345,
                pattern: "{number:04d}".to_owned(),
            },
            NotRekeyed::TargetExists(item()),
            NotRekeyed::PreviewStale(Staleness::ItemsChanged(vec![item()])),
            NotRekeyed::EntryRefused {
                entry: entry(),
                refusal: RetirementRefusal::IdTaken {
                    holder: item(),
                    field: IdentityField::Id,
                },
            },
            NotRekeyed::EntryRefused {
                entry: entry(),
                refusal: RetirementRefusal::KeyLinked { holder: item() },
            },
            NotRekeyed::EntryRefused {
                entry: entry(),
                refusal: RetirementRefusal::TargetExists(item()),
            },
            NotRekeyed::EntryRolledBack {
                entry: entry(),
                cause: Box::new(RetirementCause {
                    path: item(),
                    kind: RetirementCauseKind::ChangedSinceSnapshot,
                }),
            },
            NotRekeyed::RestoreIncomplete {
                entry: entry(),
                recovery: at(".accelerator/state/retirement-recovery/r"),
            },
            NotRekeyed::CorpusInaccessible {
                path: at("meta/work"),
                detail: "permission denied".to_owned(),
            },
        ];
        for refusal in refusals {
            let rendered = refusal.relative_to(root).to_string();
            assert!(!rendered.contains("/repo"), "{rendered}");
        }
    }

    #[test]
    fn every_outcome_renders_its_code_prefix_and_remedy() {
        for (outcome, remedy) in every_remedy() {
            let rendered = outcome.to_string();
            assert!(
                rendered.starts_with(&format!("{}: ", outcome.code())),
                "{rendered}"
            );
            assert!(rendered.contains(remedy), "{remedy}\n---\n{rendered}");
        }
    }

    #[test]
    fn a_malformed_plan_remedy_never_suggests_finishing_or_deleting_files() {
        let rendered = malformed().to_string();
        for forbidden in ["to finish", "delete", "remove"] {
            assert!(!rendered.contains(forbidden), "{rendered}");
        }
    }

    #[test]
    fn the_unreverted_paths_are_listed_under_their_base_revision() {
        let rendered = NotRekeyed::NotReverted(Unreverted {
            restore: BTreeMap::from([
                (
                    Some(Base::new(vec!["c1".to_owned()])),
                    vec!["meta/notes/a.md".to_owned()],
                ),
                (
                    Some(Base::new(vec!["p1".to_owned(), "p2".to_owned()])),
                    vec!["meta/notes/b.md".to_owned()],
                ),
                (None, vec!["meta/notes/c.md".to_owned()]),
            ]),
            remove: vec!["meta/work/ACC-0042-search.md".to_owned()],
        })
        .to_string();
        for listed in [
            "restore from c1:\n    meta/notes/a.md",
            "restore from the parents p1, p2:\n    meta/notes/b.md",
            "restore to its content when the run first touched it:\n    \
             meta/notes/c.md",
            "remove:\n    meta/work/ACC-0042-search.md",
        ] {
            assert!(rendered.contains(listed), "{listed}\n---\n{rendered}");
        }
    }
}
