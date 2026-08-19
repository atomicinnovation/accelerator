//! The sticky-failure marker file: the current session key, path validation,
//! and reading/writing the domain marker.
//!
//! The marker lives in the executor's `0700` state directory, which sits inside
//! the repository being inventoried — routinely an unfamiliar project. Two
//! barriers keep an untrusted repository from suppressing its own findings by
//! planting a file: the path is refused if it is a symlink or not owned by the
//! effective uid, and the marker is keyed to the current session, whose value a
//! repository cannot predict at commit time. Session-keying is the domain's
//! decision; this supplies the key and enforces the path check.

use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr as _;

use design::runtime::marker::Marker;
use design::DowngradeReason;

/// The marker's name under the state directory.
const MARKER_FILE: &str = "downgrade-marker.json";

/// A per-session key: the POSIX session leader and its start time.
///
/// Stable across a crawl's many executor invocations (they share the session)
/// yet unpredictable to a repository committing a file (the leader's pid and
/// boot-relative start time are assigned at runtime). No environment variable
/// carries it, so nothing a repository controls feeds it.
#[must_use]
pub fn current_session() -> String {
    // Safety: `getsid(0)` reads the caller's own session and takes no pointer.
    let session_leader = unsafe { libc::getsid(0) };
    let started = process_probe::start_time(session_leader).unwrap_or(0);
    format!("{session_leader}.{started}")
}

/// The sticky-failure marker in a state directory.
pub struct MarkerStore {
    path: PathBuf,
}

impl MarkerStore {
    #[must_use]
    pub fn in_state_dir(state_dir: &Path) -> Self {
        Self {
            path: state_dir.join(MARKER_FILE),
        }
    }

    /// The recorded marker, or `None` when absent, unparseable, or refused by
    /// the path check — a refused file never suppresses a crawl.
    #[must_use]
    pub fn read(&self) -> Option<Marker> {
        if !path_is_trusted(&self.path) {
            return None;
        }
        deserialise(&std::fs::read(&self.path).ok()?)
    }

    /// Record a marker, best-effort: a symlink or another user's file at the
    /// path is left untouched rather than written through, and suppression is
    /// the safe error, so a skipped write merely forgoes negative caching.
    pub fn write(&self, marker: &Marker) {
        if !path_is_trusted(&self.path) {
            return;
        }
        let _ = std::fs::write(&self.path, serialise(marker));
    }

    /// Remove the marker. Clearing is the direction that must not strand a user,
    /// so a best-effort remove is right: removing a planted symlink drops the
    /// symlink itself, and a file the effective uid cannot remove is one `read`
    /// already refuses.
    pub fn clear(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Whether the path may be honoured or written: present-and-a-symlink or
/// present-and-another-user's is refused; absent is fine.
fn path_is_trusted(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;

    std::fs::symlink_metadata(path).map_or(true, |metadata| {
        is_trusted(
            metadata.file_type().is_symlink(),
            metadata.uid(),
            effective_uid(),
        )
    })
}

/// The path-trust predicate, separated from the filesystem so the uid rule is
/// testable without a second user.
#[must_use]
const fn is_trusted(
    is_symlink: bool,
    owner_uid: u32,
    effective_uid: u32,
) -> bool {
    !is_symlink && owner_uid == effective_uid
}

fn effective_uid() -> u32 {
    // Safety: `geteuid` takes no argument and cannot fail.
    unsafe { libc::geteuid() }
}

fn serialise(marker: &Marker) -> String {
    serde_json::json!({
        "reason": marker.reason.key(),
        "session": marker.session,
        "recorded_at": marker.recorded_at,
        "digest": marker.digest,
    })
    .to_string()
}

fn deserialise(bytes: &[u8]) -> Option<Marker> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let reason =
        DowngradeReason::from_str(value.get("reason")?.as_str()?).ok()?;
    let session = value.get("session")?.as_str()?.to_owned();
    let recorded_at = value.get("recorded_at")?.as_u64()?;
    let digest = value
        .get("digest")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    Some(Marker {
        reason,
        session,
        recorded_at,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::current_session;
    use super::is_trusted;
    use super::MarkerStore;
    use design::runtime::marker::Marker;
    use design::DowngradeReason;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_regular_file_owned_by_the_effective_uid_is_trusted() {
        assert!(is_trusted(false, 1000, 1000));
    }

    #[test]
    fn a_symlink_is_refused_even_when_owned_by_the_effective_uid() {
        assert!(!is_trusted(true, 1000, 1000));
    }

    #[test]
    fn a_file_owned_by_another_uid_is_refused() {
        assert!(!is_trusted(false, 0, 1000));
    }

    #[test]
    fn the_session_key_is_stable_and_non_empty() {
        let first = current_session();
        assert!(!first.is_empty());
        assert_eq!(first, current_session());
    }

    fn host_marker() -> Marker {
        Marker {
            reason: DowngradeReason::GlibcTooOld,
            session: "12.345".to_owned(),
            recorded_at: 1000,
            digest: Some("/cache/trees/driver-abc".to_owned()),
        }
    }

    #[test]
    fn a_written_marker_reads_back_identically() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let store = MarkerStore::in_state_dir(work.path());
        assert_eq!(store.read(), None, "nothing recorded yet");
        store.write(&host_marker());
        assert_eq!(store.read(), Some(host_marker()));
        Ok(())
    }

    #[test]
    fn a_fetch_marker_round_trips_with_no_digest() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let store = MarkerStore::in_state_dir(work.path());
        let marker = Marker {
            reason: DowngradeReason::DiskFloorNotMet,
            session: "12.345".to_owned(),
            recorded_at: 42,
            digest: None,
        };
        store.write(&marker);
        assert_eq!(store.read(), Some(marker));
        Ok(())
    }

    #[test]
    fn clearing_removes_the_marker() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let store = MarkerStore::in_state_dir(work.path());
        store.write(&host_marker());
        store.clear();
        assert_eq!(store.read(), None);
        Ok(())
    }

    #[test]
    fn a_symlink_at_the_marker_path_is_refused() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let target = work.path().join("elsewhere.json");
        std::fs::write(&target, serialise_host_marker())?;
        let link = work.path().join(super::MARKER_FILE);
        std::os::unix::fs::symlink(&target, &link)?;
        let store = MarkerStore::in_state_dir(work.path());
        assert_eq!(
            store.read(),
            None,
            "a symlinked marker must not be honoured"
        );
        Ok(())
    }

    fn serialise_host_marker() -> String {
        super::serialise(&host_marker())
    }

    #[test]
    fn a_writable_directory_survives_a_write_through_a_symlink(
    ) -> Result<(), TestError> {
        // The write is skipped rather than followed, so the symlink target is
        // untouched.
        let work = tempfile::tempdir()?;
        let target = work.path().join("elsewhere.json");
        std::fs::write(&target, "original")?;
        let link = work.path().join(super::MARKER_FILE);
        std::os::unix::fs::symlink(&target, &link)?;
        MarkerStore::in_state_dir(work.path()).write(&host_marker());
        assert_eq!(std::fs::read_to_string(&target)?, "original");
        Ok(())
    }
}
