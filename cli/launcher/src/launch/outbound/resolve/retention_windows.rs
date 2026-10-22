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
