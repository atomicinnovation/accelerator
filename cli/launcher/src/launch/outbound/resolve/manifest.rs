//! The release manifest — the launcher's signed read contract.
//!
//! Deserialised leniently (unknown additive fields ignored) under "no launcher
//! self-update", but the `schema_version` is gated *first* via a minimal,
//! version-stable envelope so a breaking future schema yields a clear
//! "unsupported `schema_version`" diagnostic rather than a misleading parse
//! error,
//! and the `version` is checked for exact equality (anti-rollback).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::launch::core::ResolutionError;

/// The manifest schema the launcher understands. Bumped only on a breaking
/// shape change; kept coherent with the shared contract artifact and the 0165
/// signer.
pub const SUPPORTED_SCHEMA_VERSION: u64 = 1;

/// The all-zeros sentinel meaning "no binary published for this version". Pinned
/// as bare 64-char zero hex in the shared contract; a `sha256:`-prefixed form is
/// also treated as the sentinel (matching the strip-if-present tolerance).
const SENTINEL_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// The minimal, version-stable outer envelope: only the fields that must be
/// readable across *every* schema version, so the schema gate can run before the
/// rest of the (possibly reshaped) document is parsed.
#[derive(Debug, Deserialize)]
struct SchemaEnvelope {
    schema_version: u64,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub schema_version: u64,
    pub version: String,
    #[serde(default)]
    pub binaries: BTreeMap<String, BinaryEntry>,
}

#[derive(Debug, Deserialize)]
pub struct BinaryEntry {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub platforms: BTreeMap<String, PlatformEntry>,
}

#[derive(Debug, Deserialize)]
pub struct PlatformEntry {
    pub sha256: String,
    pub signature: String,
}

impl PlatformEntry {
    /// The bare lowercase-hex sha256, tolerating a `sha256:` prefix — the shared
    /// Python hashing path returns bare hex while the legacy `checksums.json`
    /// writer prefixes it, so the reader accepts both rather than silently
    /// failing verification on a prefixed digest.
    ///
    /// # Errors
    ///
    /// [`ResolutionError::AssetNotFound`] if the digest is the all-zeros
    /// "no binary for this version" sentinel (never a real hash).
    pub fn bare_sha256(&self, asset: &str) -> Result<&str, ResolutionError> {
        let bare = self.sha256.strip_prefix("sha256:").unwrap_or(&self.sha256);
        if bare == SENTINEL_SHA256 {
            return Err(ResolutionError::AssetNotFound {
                target: asset.to_owned(),
                url: "manifest sentinel (no binary for this version)"
                    .to_owned(),
            });
        }
        Ok(bare)
    }
}

impl Manifest {
    /// Parse the schema envelope and gate the schema version, then parse the
    /// full document and gate the version. The caller MUST have verified the
    /// manifest signature over the raw bytes before calling.
    ///
    /// # Errors
    ///
    /// [`ResolutionError::UnsupportedSchema`],
    /// [`ResolutionError::ManifestVersionMismatch`], or a parse error surfaced
    /// as [`ResolutionError::Cache`].
    pub fn parse_and_validate(
        bytes: &[u8],
        expected_version: &str,
    ) -> Result<Self, ResolutionError> {
        // Gate the schema version FIRST, from the minimal envelope, so an
        // unrecognised higher major yields a clear diagnostic even if the rest
        // of the document reshaped under it.
        let envelope: SchemaEnvelope =
            serde_json::from_slice(bytes).map_err(|error| {
                ResolutionError::Cache {
                    path: "manifest.json".into(),
                    detail: format!("manifest is not valid JSON: {error}"),
                }
            })?;
        if envelope.schema_version > SUPPORTED_SCHEMA_VERSION {
            return Err(ResolutionError::UnsupportedSchema {
                found: envelope.schema_version,
                supported: SUPPORTED_SCHEMA_VERSION,
            });
        }

        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|error| {
                ResolutionError::Cache {
                    path: "manifest.json".into(),
                    detail: format!("manifest is not valid JSON: {error}"),
                }
            })?;
        if manifest.version != expected_version {
            return Err(ResolutionError::ManifestVersionMismatch {
                expected: expected_version.to_owned(),
                actual: manifest.version,
            });
        }
        Ok(manifest)
    }

    /// Look up a binary's entry for a platform, or `None` if absent.
    #[must_use]
    pub fn platform_entry(
        &self,
        binary: &str,
        platform: &str,
    ) -> Option<&PlatformEntry> {
        self.binaries.get(binary)?.platforms.get(platform)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::launch::core::ResolutionError;

    use super::{Manifest, SUPPORTED_SCHEMA_VERSION};

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// The shared golden contract fixture (Decisions §6), also validated
    /// structurally by the Python contract test — the two readers of the
    /// publisher↔launcher contract cannot silently diverge.
    const GOLDEN: &str =
        include_str!("../../../../tests/fixtures/manifest.example.json");

    #[test]
    fn parses_the_shared_golden_contract_fixture() -> Result<(), Box<dyn Error>>
    {
        // Parse with the fixture's OWN declared version so a plugin version bump
        // (which does not touch this contract fixture) cannot drift it into the
        // anti-rollback mismatch path — the fixture is a shape contract.
        let value: serde_json::Value = serde_json::from_str(GOLDEN)?;
        let version = value["version"].as_str().ok_or("no version")?;
        let manifest =
            Manifest::parse_and_validate(GOLDEN.as_bytes(), version)?;
        let entry = manifest
            .platform_entry("accelerator-visualiser", "darwin-arm64")
            .ok_or("missing entry")?;
        assert_eq!(entry.bare_sha256("accelerator-visualiser")?.len(), 64);
        Ok(())
    }

    fn manifest_json(version: &str, sha256: &str) -> String {
        format!(
            "{{\"schema_version\":1,\"version\":\"{version}\",\"binaries\":{{\
             \"foo\":{{\"description\":\"Bar tool\",\"platforms\":{{\
             \"darwin-arm64\":{{\"sha256\":\"{sha256}\",\"signature\":\"sig\"\
             }}}}}}}}}}"
        )
    }

    #[test]
    fn parses_a_well_formed_manifest() -> Result<(), Box<dyn Error>> {
        let json = manifest_json(VERSION, &"a".repeat(64));
        let manifest = Manifest::parse_and_validate(json.as_bytes(), VERSION)?;
        let foo = manifest.binaries.get("foo").ok_or("missing foo")?;
        assert_eq!(foo.description, "Bar tool");
        let entry = manifest
            .platform_entry("foo", "darwin-arm64")
            .ok_or("missing platform entry")?;
        assert_eq!(entry.bare_sha256("foo")?.len(), 64);
        Ok(())
    }

    #[test]
    fn strips_a_sha256_prefix_from_the_digest() -> Result<(), Box<dyn Error>> {
        let json =
            manifest_json(VERSION, &format!("sha256:{}", "b".repeat(64)));
        let manifest = Manifest::parse_and_validate(json.as_bytes(), VERSION)?;
        let entry = manifest
            .platform_entry("foo", "darwin-arm64")
            .ok_or("missing entry")?;
        assert_eq!(entry.bare_sha256("foo")?, "b".repeat(64));
        Ok(())
    }

    #[test]
    fn an_all_zeros_digest_is_the_sentinel_named_error(
    ) -> Result<(), Box<dyn Error>> {
        let json = manifest_json(VERSION, &"0".repeat(64));
        let manifest = Manifest::parse_and_validate(json.as_bytes(), VERSION)?;
        let entry = manifest
            .platform_entry("foo", "darwin-arm64")
            .ok_or("missing entry")?;
        assert!(matches!(
            entry.bare_sha256("foo"),
            Err(ResolutionError::AssetNotFound { .. })
        ));
        Ok(())
    }

    #[test]
    fn rejects_an_unsupported_higher_schema() {
        let json = format!(
            "{{\"schema_version\": {}, \"version\": \"{VERSION}\", \
             \"binaries\": {{}}}}",
            SUPPORTED_SCHEMA_VERSION + 1
        );
        assert!(matches!(
            Manifest::parse_and_validate(json.as_bytes(), VERSION),
            Err(ResolutionError::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn rejects_a_version_mismatch() {
        let json = manifest_json("9.9.9", &"a".repeat(64));
        assert!(matches!(
            Manifest::parse_and_validate(json.as_bytes(), VERSION),
            Err(ResolutionError::ManifestVersionMismatch { .. })
        ));
    }

    #[test]
    fn ignores_unknown_additive_fields() -> Result<(), Box<dyn Error>> {
        let json = format!(
            "{{\"schema_version\": 1, \"version\": \"{VERSION}\", \
             \"future_field\": 42, \"binaries\": {{}}}}"
        );
        Manifest::parse_and_validate(json.as_bytes(), VERSION)?;
        Ok(())
    }
}
