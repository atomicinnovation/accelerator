//! How long the cache root keeps what each plugin version left in it.

pub mod ports;
pub mod root_retention;
pub mod version_claims;

/// A per-install identity for a retention claim.
///
/// Derived from the launcher's own content-addressed path, so two installs
/// sharing a cache root write distinct claim files. Falls back to the version
/// when the path is unavailable.
#[must_use]
pub fn launcher_id() -> String {
    use std::fmt::Write as _;

    use sha2::{Digest as _, Sha256};
    let seed = std::env::current_exe().map_or_else(
        |_| env!("CARGO_PKG_VERSION").to_owned(),
        |path| path.to_string_lossy().into_owned(),
    );
    let digest = Sha256::digest(seed.as_bytes());
    digest.iter().take(8).fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}
