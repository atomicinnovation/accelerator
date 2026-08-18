//! The on-disk naming grammar, and the checks that make a name safe to join.
//!
//! Every name here is ASCII. `cache::find`'s prefix scan aborts the *whole*
//! scan on one non-UTF-8 directory entry, so a stray name in this directory
//! would turn every single-file resolution into a miss. None is named
//! `*.minisig` either, so a tree's sidecars can never be mistaken for a
//! single-file signature.

use std::path::{Path, PathBuf};

use crate::launch::core::tree::TreeError;

/// The extraction and sealing policy this launcher applies.
///
/// Content addressing means a newer launcher routinely finds an older one's
/// tree in a shared cache root, and the archive digest says nothing about how
/// that tree was extracted or sealed. Without this a policy fix would be
/// silently inherited rather than applied, and verification would pass because
/// it checked against the older table.
pub const LAYOUT_VERSION: u32 = 1;

/// The directory the launcher owns, creates `0700`, and holds to a strict
/// ownership check — as against the cache root, whose mode is inherited from
/// whoever created it and is umask-dependent.
pub const TREES_DIR: &str = "trees";

/// One install's retention claims, read by `prune` so it needs no knowledge of
/// any other install.
pub const CLAIMS_DIR: &str = "claims";

const POINTER_SUFFIX: &str = ".ref";
const ATTESTATION_SUFFIX: &str = ".sealed";
const ATTESTATION_SIGNATURE_SUFFIX: &str = ".sealed.sig";
const LEASE_SUFFIX: &str = ".lease";
const TEMP_PREFIX: &str = ".tmp-";
const ARCHIVE_SUFFIX: &str = ".archive";

/// Hex characters of CSPRNG output in a generation suffix.
const SUFFIX_LEN: usize = 16;
const DIGEST_LEN: usize = 64;

/// Where a tree's files and sidecars live, derived once so no caller composes a
/// path from string fragments of its own.
pub struct TreePaths {
    root: PathBuf,
}

impl TreePaths {
    /// `<cache_root>/trees`, the boundary the launcher owns.
    #[must_use]
    pub fn under(cache_root: &Path) -> Self {
        Self {
            root: cache_root.join(TREES_DIR),
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn claims(&self) -> PathBuf {
        self.root.join(CLAIMS_DIR)
    }

    /// The pointer naming the generation currently published for this digest.
    ///
    /// Keyed by digest rather than by release version, so an unchanged pin
    /// resolves across plugin upgrades and a shared root accumulates one
    /// pointer per distinct artifact version rather than one per release.
    #[must_use]
    pub fn pointer(&self, name: &str, platform: &str, digest: &str) -> PathBuf {
        self.root
            .join(format!("{name}-{platform}-{digest}{POINTER_SUFFIX}"))
    }

    #[must_use]
    pub fn generation(&self, generation: &str) -> PathBuf {
        self.root.join(generation)
    }

    #[must_use]
    pub fn attestation(&self, generation: &str) -> PathBuf {
        self.root.join(format!("{generation}{ATTESTATION_SUFFIX}"))
    }

    #[must_use]
    pub fn attestation_signature(&self, generation: &str) -> PathBuf {
        self.root
            .join(format!("{generation}{ATTESTATION_SIGNATURE_SUFFIX}"))
    }

    /// The in-use lease, a sidecar *beside* the generation rather than a file
    /// inside it.
    ///
    /// Inside, the seal would make it read-only for the very dispatches that
    /// must open it for writing to take a lock, and it would be an entry absent
    /// from the file table — so verification would report every healthy tree as
    /// carrying an unexpected entry, and repair would re-materialise trees that
    /// are fine.
    #[must_use]
    pub fn lease(&self, generation: &str) -> PathBuf {
        self.root.join(format!("{generation}{LEASE_SUFFIX}"))
    }

    /// The partially-downloaded archive.
    ///
    /// Named by artifact, platform and digest rather than by generation: a
    /// generation-keyed name would be unique per attempt, so a later run could
    /// never find the partial to resume, and the reaper could not derive which
    /// lock guards the residue.
    #[must_use]
    pub fn temp_archive(
        &self,
        name: &str,
        platform: &str,
        digest: &str,
    ) -> PathBuf {
        self.root.join(format!(
            "{TEMP_PREFIX}{name}-{platform}-{digest}{ARCHIVE_SUFFIX}"
        ))
    }

    #[must_use]
    pub fn temp_generation(&self, generation: &str) -> PathBuf {
        self.root.join(format!("{TEMP_PREFIX}{generation}"))
    }

    #[must_use]
    pub fn single_flight_lock(&self, name: &str, platform: &str) -> PathBuf {
        self.root
            .join(format!("{TEMP_PREFIX}{name}-{platform}.lock"))
    }
}

/// The fixed part of a generation directory's name.
///
/// The reuse scan looks for this prefix, which is why a generation name is
/// never parsed by splitting on `-`: both the artifact name and the platform
/// alias contain hyphens, so only a comparison against a known prefix is
/// unambiguous.
#[must_use]
pub fn generation_prefix(
    name: &str,
    platform: &str,
    digest: &str,
    layout: u32,
) -> String {
    format!("{name}-{platform}-{digest}-{layout}-")
}

/// A generation directory's name, fresh by construction.
///
/// `suffix` is CSPRNG output rather than a pid, a timestamp or a counter: the
/// freshness is what removes the rename-collision branch when the tree is
/// published, and this repository has already been bitten by pid reuse in cache
/// paths.
#[must_use]
pub fn generation_name(
    name: &str,
    platform: &str,
    digest: &str,
    layout: u32,
    suffix: &str,
) -> String {
    format!(
        "{}{suffix}",
        generation_prefix(name, platform, digest, layout)
    )
}

/// The generation a validated pointer names, or why the pointer is unusable.
///
/// A pointer is unsigned local state whose contents become a path, so it is
/// validated in full before it is joined: the artifact, the platform, the
/// digest and the layout version must all be the ones being resolved, the
/// suffix must be the expected shape, and the result must name a direct child
/// of the trees directory.
///
/// # Errors
///
/// [`TreeError::Pointer`] naming which part of the grammar the contents broke,
/// or [`TreeError::LayoutUnsupported`] when the layout is recognisable but not
/// this launcher's.
pub fn validated_generation(
    contents: &str,
    name: &str,
    platform: &str,
    digest: &str,
    layout: u32,
) -> Result<String, TreeError> {
    let candidate = contents.trim();
    if candidate.is_empty() {
        return Err(pointer_error("the pointer is empty"));
    }
    if candidate.contains('/') || candidate.contains('\\') {
        return Err(pointer_error(
            "the pointer names something other than a direct child of the \
             trees directory",
        ));
    }
    if !candidate.is_ascii() {
        return Err(pointer_error("the pointer is not ASCII"));
    }

    let expected = generation_prefix(name, platform, digest, layout);
    let Some(suffix) = candidate.strip_prefix(&expected) else {
        return Err(mismatched_prefix(
            candidate, name, platform, digest, layout,
        ));
    };
    if suffix.len() != SUFFIX_LEN
        || !suffix
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
    {
        return Err(pointer_error(
            "the generation suffix is not sixteen lowercase hex characters",
        ));
    }
    Ok(candidate.to_owned())
}

/// Distinguish "a different artifact, platform or digest" from "a layout this
/// launcher does not run", because only the second is worth re-materialising
/// for rather than treating as an unrelated pointer.
fn mismatched_prefix(
    candidate: &str,
    name: &str,
    platform: &str,
    digest: &str,
    layout: u32,
) -> TreeError {
    let identity = format!("{name}-{platform}-{digest}-");
    if let Some(rest) = candidate.strip_prefix(&identity) {
        if let Some((found, _)) = rest.split_once('-') {
            if let Ok(found) = found.parse::<u32>() {
                return TreeError::LayoutUnsupported {
                    found,
                    supported: layout,
                };
            }
        }
    }
    pointer_error(
        "the pointer names a different artifact, platform or digest than the \
         one being resolved",
    )
}

fn pointer_error(detail: &str) -> TreeError {
    TreeError::Pointer {
        detail: detail.to_owned(),
    }
}

/// Whether a digest is the lowercase hex the naming grammar admits.
#[must_use]
pub fn is_wellformed_digest(digest: &str) -> bool {
    digest.len() == DIGEST_LEN
        && digest
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
}

/// Whether the path is owned by this user and writable by nobody else.
///
/// `symlink_metadata`, never `stat`: `stat` follows symlinks, so a symlink
/// placed at a generation path and pointing at any user-owned,
/// non-group-writable directory would satisfy an ownership test perfectly.
#[cfg(unix)]
pub fn is_privately_owned(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    use std::os::unix::fs::PermissionsExt as _;

    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() {
        return false;
    }
    let mode = metadata.permissions().mode();
    metadata.uid() == rustix::process::geteuid().as_raw() && mode & 0o022 == 0
}

#[cfg(not(unix))]
pub fn is_privately_owned(_path: &Path) -> bool {
    // Windows is outside the supported matrix; the marker arm keeps the
    // resolver type-checking off Unix, as its neighbours do.
    false
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::path::Path;

    use crate::launch::core::tree::TreeError;

    use super::{
        generation_name, generation_prefix, is_wellformed_digest,
        validated_generation, TreePaths, LAYOUT_VERSION,
    };

    const DIGEST: &str =
        "abc0000000000000000000000000000000000000000000000000000000000123";
    const SUFFIX: &str = "0123456789abcdef";

    fn generation() -> String {
        generation_name("browser", "linux-x64", DIGEST, LAYOUT_VERSION, SUFFIX)
    }

    #[test]
    fn no_sidecar_is_named_like_a_single_file_signature() {
        let paths = TreePaths::under(Path::new("/cache"));
        let generation = generation();
        let names = [
            paths.pointer("browser", "linux-x64", DIGEST),
            paths.attestation(&generation),
            paths.attestation_signature(&generation),
            paths.lease(&generation),
            paths.temp_archive("browser", "linux-x64", DIGEST),
            paths.single_flight_lock("browser", "linux-x64"),
        ];
        for path in names {
            let name = path.to_str().expect("ascii path");
            assert!(
                !name.ends_with(".minisig"),
                "{name} collides with the single-file signature convention"
            );
            assert!(name.is_ascii(), "{name} is not ASCII");
        }
    }

    #[test]
    fn a_pointer_naming_this_generation_validates() {
        let generation = generation();
        let validated = validated_generation(
            &generation,
            "browser",
            "linux-x64",
            DIGEST,
            LAYOUT_VERSION,
        )
        .expect("the pointer names exactly what is being resolved");
        assert_eq!(validated, generation);
    }

    #[test]
    fn a_pointer_is_validated_before_its_contents_become_a_path() {
        let generation = generation();
        let cases = [
            "",
            "   ",
            "../../etc",
            "sub/dir",
            "browser-linux-x64",
            &format!("{generation}-extra"),
            // A different artifact, platform, or digest.
            &generation.replace("browser", "driver"),
            &generation.replace("linux-x64", "darwin-arm64"),
            &generation.replace(DIGEST, &"f".repeat(64)),
            // A suffix that is not sixteen lowercase hex characters.
            &generation.replace(SUFFIX, "0123456789ABCDEF"),
            &generation.replace(SUFFIX, "short"),
            &generation.replace(SUFFIX, "zzzzzzzzzzzzzzzz"),
        ];
        for candidate in cases {
            let outcome = validated_generation(
                candidate,
                "browser",
                "linux-x64",
                DIGEST,
                LAYOUT_VERSION,
            );
            assert!(
                outcome.is_err(),
                "{candidate:?} was accepted as a pointer"
            );
        }
    }

    #[test]
    fn a_higher_layout_version_is_refused_rather_than_parsed() {
        let ahead = generation_name(
            "browser",
            "linux-x64",
            DIGEST,
            LAYOUT_VERSION + 1,
            SUFFIX,
        );
        let outcome = validated_generation(
            &ahead,
            "browser",
            "linux-x64",
            DIGEST,
            LAYOUT_VERSION,
        );
        assert!(matches!(
            outcome,
            Err(TreeError::LayoutUnsupported { found, supported })
                if found == LAYOUT_VERSION + 1 && supported == LAYOUT_VERSION
        ));
    }

    #[test]
    fn the_reuse_scan_prefix_pins_identity_and_layout_but_not_the_generation() {
        let prefix =
            generation_prefix("browser", "linux-x64", DIGEST, LAYOUT_VERSION);
        assert!(generation().starts_with(&prefix));
        assert!(!generation_name(
            "browser",
            "linux-x64",
            DIGEST,
            LAYOUT_VERSION + 1,
            SUFFIX
        )
        .starts_with(&prefix));
    }

    #[test]
    fn the_digest_grammar_admits_only_lowercase_hex_of_the_right_length() {
        assert!(is_wellformed_digest(DIGEST));
        assert!(!is_wellformed_digest(&DIGEST.to_uppercase()));
        assert!(!is_wellformed_digest(&"a".repeat(63)));
        assert!(!is_wellformed_digest(&"a".repeat(65)));
        assert!(!is_wellformed_digest(&"g".repeat(64)));
    }
}
