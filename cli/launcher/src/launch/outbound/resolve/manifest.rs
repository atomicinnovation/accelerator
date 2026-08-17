//! The launcher's signed read contract. Unknown additive fields are ignored,
//! but `schema_version` is gated first (via a minimal envelope) and `version` is
//! checked for exact equality (anti-rollback).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::launch::core::ResolutionError;

/// The manifest schema the launcher understands; bumped only on a breaking shape
/// change.
pub const SUPPORTED_SCHEMA_VERSION: u64 = 1;

/// The all-zeros digest meaning "no binary published for this version".
const SENTINEL_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// The version-stable subset readable across every schema, so the schema gate
/// can run before the rest of the document is parsed.
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
    #[serde(default)]
    pub artifacts: BTreeMap<String, ArtifactEntry>,
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

/// A directory-tree artifact, published as one signed archive per platform.
///
/// A platform this release does not publish is represented by omitting its key,
/// never by the all-zeros sentinel `binaries` uses: a sentinel entry has no
/// archive, so its three sizes would have no honest value but zero — which is
/// the value the consumer must treat as a defect.
#[derive(Debug, Deserialize)]
pub struct ArtifactEntry {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub platforms: BTreeMap<String, ArtifactPlatformEntry>,
}

/// One platform's tree archive.
///
/// The three sizes are required rather than defaulted: a defaulted zero would
/// silently disable both the download cap and the decompression-bomb ceiling,
/// which is the failure mode a default exists to avoid.
#[derive(Debug, Deserialize)]
pub struct ArtifactPlatformEntry {
    pub sha256: String,
    pub signature: String,
    /// Bounds the download.
    pub archive_size: u64,
    /// With `archive_size`, bounds extraction and the free-space precheck —
    /// both copies exist on disk at once.
    pub uncompressed_size: u64,
    /// Bounds the entry count during extraction.
    pub entry_count: u64,
}

impl PlatformEntry {
    /// The bare lowercase-hex sha256, tolerating an optional `sha256:` prefix.
    ///
    /// # Errors
    ///
    /// [`ResolutionError::AssetNotFound`] if the digest is the all-zeros
    /// sentinel.
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
    /// Gate the schema version (via the envelope) then the version. The caller
    /// must have verified the manifest signature over the raw bytes first.
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
        // Gate the schema version before parsing the (possibly reshaped) rest.
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

    /// Look up a tree artifact's entry for a platform, or `None` if this
    /// release publishes none for it.
    #[must_use]
    pub fn artifact_platform_entry(
        &self,
        artifact: &str,
        platform: &str,
    ) -> Option<&ArtifactPlatformEntry> {
        self.artifacts.get(artifact)?.platforms.get(platform)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::launch::core::ResolutionError;

    use super::{Manifest, SUPPORTED_SCHEMA_VERSION};

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    // The shared golden contract fixture, also validated by the Python test.
    const GOLDEN: &str =
        include_str!("../../../../tests/fixtures/manifest.example.json");

    #[test]
    fn parses_the_shared_golden_contract_fixture() -> Result<(), Box<dyn Error>>
    {
        // Parse with the fixture's own version so a plugin bump can't drift it.
        let value: serde_json::Value = serde_json::from_str(GOLDEN)?;
        let version = value["version"].as_str().ok_or("no version")?;
        let manifest =
            Manifest::parse_and_validate(GOLDEN.as_bytes(), version)?;
        let entry = manifest
            .platform_entry("visualiser", "darwin-arm64")
            .ok_or("missing entry")?;
        assert_eq!(entry.bare_sha256("visualiser")?.len(), 64);
        Ok(())
    }

    #[test]
    fn parses_the_artifacts_map_from_the_golden_fixture(
    ) -> Result<(), Box<dyn Error>> {
        let value: serde_json::Value = serde_json::from_str(GOLDEN)?;
        let version = value["version"].as_str().ok_or("no version")?;
        let manifest =
            Manifest::parse_and_validate(GOLDEN.as_bytes(), version)?;
        let entry = manifest
            .artifact_platform_entry("browser", "linux-x64")
            .ok_or("missing artifact entry")?;
        assert_eq!(entry.sha256.len(), 64);
        assert!(entry.archive_size > 0);
        assert!(entry.uncompressed_size > entry.archive_size);
        assert!(entry.entry_count > 0);
        Ok(())
    }

    #[test]
    fn an_absent_artifact_platform_yields_none() -> Result<(), Box<dyn Error>> {
        let value: serde_json::Value = serde_json::from_str(GOLDEN)?;
        let version = value["version"].as_str().ok_or("no version")?;
        let manifest =
            Manifest::parse_and_validate(GOLDEN.as_bytes(), version)?;
        assert!(manifest
            .artifact_platform_entry("browser", "solaris-sparc")
            .is_none());
        assert!(manifest
            .artifact_platform_entry("nonesuch", "linux-x64")
            .is_none());
        Ok(())
    }

    fn artifact_manifest_json(platform_entry: &str) -> String {
        format!(
            "{{\"schema_version\":1,\"version\":\"{VERSION}\",\
             \"binaries\":{{}},\"artifacts\":{{\
             \"browser\":{{\"description\":\"Headless shell\",\
             \"platforms\":{{\"linux-x64\":{platform_entry}}}}}}}}}"
        )
    }

    #[test]
    fn an_artifact_platform_entry_missing_a_size_is_rejected() {
        for omitted in ["archive_size", "uncompressed_size", "entry_count"] {
            let mut fields = vec![
                "\"sha256\":\"aa\"".to_owned(),
                "\"signature\":\"sig\"".to_owned(),
                "\"archive_size\":10".to_owned(),
                "\"uncompressed_size\":20".to_owned(),
                "\"entry_count\":3".to_owned(),
            ];
            fields.retain(|field| !field.contains(omitted));
            let json =
                artifact_manifest_json(&format!("{{{}}}", fields.join(",")));
            assert!(
                Manifest::parse_and_validate(json.as_bytes(), VERSION).is_err(),
                "expected a parse failure with {omitted} omitted"
            );
        }
    }

    #[test]
    fn ignores_unknown_additive_fields_inside_an_artifact_entry(
    ) -> Result<(), Box<dyn Error>> {
        let json = artifact_manifest_json(
            "{\"sha256\":\"aa\",\"signature\":\"sig\",\"archive_size\":10,\
             \"uncompressed_size\":20,\"entry_count\":3,\"future_field\":42}",
        );
        let manifest = Manifest::parse_and_validate(json.as_bytes(), VERSION)?;
        assert!(manifest
            .artifact_platform_entry("browser", "linux-x64")
            .is_some());
        Ok(())
    }

    #[test]
    fn a_manifest_without_artifacts_still_resolves_binaries(
    ) -> Result<(), Box<dyn Error>> {
        let json = manifest_json(VERSION, &"a".repeat(64));
        let manifest = Manifest::parse_and_validate(json.as_bytes(), VERSION)?;
        assert!(manifest.artifacts.is_empty());
        assert!(manifest.platform_entry("foo", "darwin-arm64").is_some());
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
