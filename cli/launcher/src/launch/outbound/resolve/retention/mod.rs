pub mod ports;
pub mod residue;
pub mod root_entries;
pub mod root_lock;
pub mod root_prune;
pub mod root_retention;
pub mod single_file;
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
pub(crate) mod testing;
pub mod version_claims;

/// Hashed from the launcher's own content-addressed path, so two installs
/// sharing a cache root write distinct claim files.
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
