//! The windows every retention rule in the cache root shares.

use std::time::Duration;

/// Residue younger than this with no live holder is spared.
///
/// A generation briefly between its rename and its pointer write is never
/// mistaken for crash residue. Sized well beyond a fetch-plus-extract.
pub const AGE_BACKSTOP: Duration = Duration::from_secs(60 * 60);

/// A claim not refreshed inside this window no longer protects anything.
///
/// Sized to a ~14-day orphan sweep, so a merely-idle install is not evicted.
pub const CLAIM_WINDOW: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// Below this fraction of the window, a fresh claim is not rewritten.
///
/// A crawl of 100-200 invocations then does at most one claim write rather
/// than one each.
pub const REFRESH_FRACTION: u64 = 10;

/// Whether something last modified at `modified_seconds` is at least
/// [`AGE_BACKSTOP`] old at `now`.
///
/// An unknown mtime is never aged and a future one is age zero, so an
/// unreadable or skewed clock can only keep.
#[must_use]
pub fn is_aged(modified_seconds: Option<u64>, now: u64) -> bool {
    modified_seconds.is_some_and(|modified| {
        now.saturating_sub(modified) >= AGE_BACKSTOP.as_secs()
    })
}

#[cfg(test)]
mod tests {
    use super::{is_aged, AGE_BACKSTOP};

    const NOW: u64 = 1_000_000;

    #[test]
    fn an_entry_exactly_at_the_backstop_is_aged() {
        assert!(is_aged(Some(NOW - AGE_BACKSTOP.as_secs()), NOW));
        assert!(!is_aged(Some(NOW - AGE_BACKSTOP.as_secs() + 1), NOW));
    }

    #[test]
    fn an_unknown_or_future_mtime_is_never_aged() {
        assert!(!is_aged(None, NOW));
        assert!(!is_aged(Some(NOW + AGE_BACKSTOP.as_secs()), NOW));
    }
}
