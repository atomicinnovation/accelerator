//! Learning the key each synced item's issue answers to now, so a
//! `{tracker}` re-key targets where the issue is rather than where it was.

use tracker::ExternalId;
use tracker::Located;
use tracker::RemoteTracker;
use tracker::TrackerError;
use work::rekey::mapping::RefreshedKeys;
use work::rekey::outcome::NotRekeyed;
use work::sync::is_move;
use work::tracker_key::TrackerKey;

use crate::sync::fetch::RetrievalStrategy;

/// The current key of every issue in `keys` that has moved.
///
/// A bulk read accounts for unmoved keys at once and `locate`s the rest;
/// per-item reads `locate` every key. A key the tracker no longer has keeps
/// its stored value, as does an answer not shaped like a tracker key.
///
/// # Errors
///
/// [`NotRekeyed::TrackerUnreachable`] at the first read that fails, since a
/// re-key must not target a key it could not confirm.
pub fn refresh_external_ids(
    keys: &[ExternalId],
    tracker: &dyn RemoteTracker,
    strategy: RetrievalStrategy,
) -> Result<RefreshedKeys, NotRekeyed> {
    let unaccounted: Vec<&ExternalId> = match strategy {
        RetrievalStrategy::PerItem => keys.iter().collect(),
        RetrievalStrategy::Bulk => {
            let outcome = tracker.fetch_all(keys).map_err(unreachable)?;
            keys.iter()
                .filter(|key| {
                    !outcome.found.iter().any(|(found, _)| found == *key)
                })
                .collect()
        }
    };
    let mut moves = Vec::new();
    for stored in unaccounted {
        if let Located::Found(issue) =
            tracker.locate(stored).map_err(unreachable)?
        {
            let current = TrackerKey::parse(issue.key.as_str())
                .filter(|_| is_move(stored, &issue.key));
            moves.extend(
                current.map(|current| (stored.as_str().to_owned(), current)),
            );
        }
    }
    Ok(moves.into_iter().collect())
}

fn unreachable(error: TrackerError) -> NotRekeyed {
    NotRekeyed::TrackerUnreachable(error.into_detail())
}
