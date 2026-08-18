//! The launcher's compiled-in expected digests and the artifact-name set.
//!
//! `acquire` resolves **only** the launcher's own compiled-in digest for an
//! `(artifact, platform)`, so a superseded artifact's generation is never even
//! looked for — that is the rollback defence, and it is strictly stronger than a
//! signed version field: it needs no field the producer cannot know, and it
//! holds offline because the digest is in the binary rather than on the network.

include!(concat!(env!("OUT_DIR"), "/tree_pins.rs"));

use super::super::HOST_PLATFORM;

/// The tree artifacts this launcher knows how to resolve, in sorted order.
///
/// Held to `TREE_ARTIFACTS` on the Python side and to the `artifacts` keys in
/// `manifest.example.json` by one drift test, so retiring an artifact cannot
/// leave the launcher requesting a name the manifest no longer carries.
#[must_use]
pub fn artifact_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = EXPECTED_TREE_DIGESTS
        .iter()
        .map(|(name, _, _)| *name)
        .collect();
    names.dedup();
    names
}

/// Whether `name` is a tree artifact this launcher recognises.
///
/// Every `cache` verb validates its `<name>` argument through this, so no path
/// is ever constructed from an unrecognised token, and it does so offline
/// rather than against the manifest.
#[must_use]
pub fn is_known_artifact(name: &str) -> bool {
    EXPECTED_TREE_DIGESTS
        .iter()
        .any(|(known, _, _)| *known == name)
}

/// The digest this launcher expects for `artifact` on the host platform.
///
/// `None` when the launcher publishes no such artifact for this platform, which
/// is a `unsupported-platform`-shaped miss rather than a reason to fetch.
#[must_use]
pub fn expected_digest(artifact: &str) -> Option<&'static str> {
    expected_digest_on(artifact, HOST_PLATFORM)
}

#[must_use]
pub fn expected_digest_on(
    artifact: &str,
    platform: &str,
) -> Option<&'static str> {
    EXPECTED_TREE_DIGESTS
        .iter()
        .find(|(name, entry_platform, _)| {
            *name == artifact && *entry_platform == platform
        })
        .map(|(_, _, digest)| *digest)
}

#[cfg(test)]
mod tests {
    use super::{
        artifact_names, expected_digest_on, is_known_artifact,
        EXPECTED_TREE_DIGESTS,
    };

    #[test]
    fn the_compiled_in_map_covers_every_artifact_on_four_platforms() {
        // driver and browser across four platforms.
        assert_eq!(EXPECTED_TREE_DIGESTS.len(), 8);
        assert_eq!(artifact_names(), vec!["browser", "driver"]);
    }

    #[test]
    fn every_digest_is_lowercase_hex() {
        for (_, _, digest) in EXPECTED_TREE_DIGESTS {
            assert_eq!(digest.len(), 64, "{digest} is not 64 chars");
            assert!(
                digest
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
                "{digest} is not lowercase hex"
            );
        }
    }

    #[test]
    fn only_known_artifacts_resolve() {
        assert!(is_known_artifact("driver"));
        assert!(is_known_artifact("browser"));
        assert!(!is_known_artifact("nonesuch"));
        assert!(expected_digest_on("nonesuch", "linux-x64").is_none());
    }

    #[test]
    fn a_known_artifact_yields_a_digest_for_a_published_platform() {
        assert!(expected_digest_on("browser", "linux-x64").is_some());
        assert!(expected_digest_on("browser", "solaris-sparc").is_none());
    }
}
