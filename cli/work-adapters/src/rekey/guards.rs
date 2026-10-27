//! What an earlier sync or re-key left behind that a person must settle
//! before a re-key may start.

use std::path::Path;
use std::path::PathBuf;

use corpus::store::KeptState;
use corpus::store::RecoveryCopies;
use work::identity::ItemIdentity;
use work::promotion::PromotionStage;
use work::rekey::entry::RekeyEntry;
use work::rekey::outcome::SyncBlocker;
use work::retirement::RECOVERY_PARENT;

use crate::retirement_records::RetirementRecords;
use crate::sync::pending_push;
use crate::sync::pending_push::Marker;
use crate::sync::pending_push::MarkerEntry;

/// Where an integration keeps its pending-push markers.
#[derive(Clone, Copy)]
pub struct PendingPushDir<'a> {
    pub integrations_root: &'a Path,
    pub integration: &'a str,
}

/// The sync state a re-key reads; `pending_push` is `None` when no
/// integration is configured.
#[derive(Clone, Copy)]
pub struct SyncState<'a> {
    pub state_dir: &'a Path,
    pub records: &'a dyn RetirementRecords,
    pub recovery: &'a dyn RecoveryCopies,
    pub pending_push: Option<PendingPushDir<'a>>,
}

impl SyncState<'_> {
    fn markers(&self) -> Vec<MarkerEntry> {
        let Some(dir) = self.pending_push else {
            return Vec::new();
        };
        let listed =
            pending_push::outstanding(dir.integrations_root, dir.integration);
        listed.unwrap_or_else(|error| {
            vec![Err(pending_push::UnreadableMarker {
                path: dir.integrations_root.join(dir.integration),
                detail: error.to_string(),
            })]
        })
    }

    fn recovery_parent(&self) -> PathBuf {
        self.state_dir.join(RECOVERY_PARENT)
    }
}

/// Every retirement and `retiring` promotion an earlier `work sync` left
/// unfinished, and every record it cannot read.
#[must_use]
pub fn unfinished_sync_work(sync: &SyncState<'_>) -> Vec<SyncBlocker> {
    let mut blockers: Vec<SyncBlocker> =
        sync.records.outstanding().map_or_else(
            |_| {
                vec![SyncBlocker::UnreadableRecord(
                    sync.state_dir.join("retirement-records"),
                )]
            },
            |records| {
                records
                    .into_iter()
                    .map(|record| SyncBlocker::Retirement {
                        old: record.old,
                        new: record.new,
                    })
                    .collect()
            },
        );
    blockers.extend(sync.markers().into_iter().filter_map(
        |marker| match marker {
            Ok((_, Marker::Promotion(record))) => {
                matches!(record.stage, PromotionStage::Retiring { .. }).then(
                    || SyncBlocker::Promotion {
                        draft: record.draft_id.as_str().to_owned(),
                    },
                )
            }
            Ok((_, Marker::Legacy(_))) | Err(_) => None,
        },
    ));
    blockers
}

/// The slug a file is named with after its `id`.
fn slug_of(item: &ItemIdentity) -> Option<&str> {
    let stem = item.path.file_stem()?.to_str()?;
    let after_id = stem
        .get(..item.id.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(&item.id))
        .map_or(stem, |_| &stem[item.id.len()..]);
    Some(after_id.trim_start_matches('-')).filter(|slug| !slug.is_empty())
}

fn names_item(marker: &Path, item: &ItemIdentity) -> bool {
    let Some(stem) = marker.file_stem().and_then(std::ffi::OsStr::to_str)
    else {
        return false;
    };
    stem.eq_ignore_ascii_case(&item.id) || slug_of(item) == Some(stem)
}

/// The item behind every push still pending on an item about to be
/// re-keyed, by its `id`, draft ID or title slug, and every marker that
/// cannot be read, by its file.
#[must_use]
pub fn pending_pushes(
    sync: &SyncState<'_>,
    entries: &[RekeyEntry],
    items: &[ItemIdentity],
) -> Vec<PathBuf> {
    let rekeyed: Vec<&ItemIdentity> = items
        .iter()
        .filter(|item| {
            entries
                .iter()
                .any(|entry| entry.old.eq_ignore_ascii_case(&item.id))
        })
        .collect();
    let mut blocked: Vec<PathBuf> = sync
        .markers()
        .into_iter()
        .flat_map(|marker| match marker {
            Ok((path, _)) => rekeyed
                .iter()
                .filter(|item| names_item(&path, item))
                .map(|item| item.path.clone())
                .collect(),
            Err(unreadable) => vec![unreadable.path],
        })
        .collect();
    blocked.sort();
    blocked.dedup();
    blocked
}

/// Every retirement recovery directory left by a retirement that never
/// completed: one with no marker, or one still awaiting a restore.
#[must_use]
pub fn unsettled_recoveries(sync: &SyncState<'_>) -> Vec<PathBuf> {
    let parent = sync.recovery_parent();
    let Ok(children) = std::fs::read_dir(&parent) else {
        return Vec::new();
    };
    let completed: Vec<PathBuf> = sync
        .recovery
        .kept(Path::new(RECOVERY_PARENT))
        .unwrap_or_default()
        .into_iter()
        .filter(|kept| kept.state == KeptState::Completed)
        .filter_map(|kept| kept.dir.file_name().map(PathBuf::from))
        .collect();
    let mut unsettled: Vec<PathBuf> = children
        .flatten()
        .filter(|child| child.path().is_dir())
        .filter(|child| !completed.contains(&PathBuf::from(child.file_name())))
        .map(|child| child.path())
        .collect();
    unsettled.sort();
    unsettled
}
