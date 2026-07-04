//! The real fetch → verify → cache resolver, composed from a [`Fetcher`], a
//! [`verifier`], a [`cache`] store, and a resolved cache root behind the
//! `ResolveBinary` port.
//!
//! The state machine is a thin sequence of guard clauses: cache-hit-verify →
//! (on failure) replace-in-place → else miss-fetch-verify-cache. A working entry
//! is never evicted before a verified successor exists (the fresh inode is
//! renamed over the corrupt one), and an offline re-verify failure is reported
//! as a distinct "corrupt AND re-fetch failed" diagnostic, not a plain miss.

pub mod cache;
pub mod cache_root;
pub mod fetcher;
pub mod keys;
pub mod manifest;
pub mod verifier;

use std::path::PathBuf;

use crate::launch::core::{ExternalCommand, ResolutionError, ResolveBinary};

use self::cache::CachedBinary;
use self::fetcher::{FetchError, Fetcher};
use self::keys::TrustedKeys;
use self::manifest::Manifest;

/// The host platform alias this launcher was built for — the single Rust source
/// of the triple→alias map, asserted equal to `tasks/shared/targets.py` by a
/// cross-language coherence test.
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub const HOST_PLATFORM: &str = "darwin-arm64";
#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
pub const HOST_PLATFORM: &str = "darwin-x64";
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub const HOST_PLATFORM: &str = "linux-arm64";
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub const HOST_PLATFORM: &str = "linux-x64";

/// Configuration for the resolver — injected so tests point it at a mock server
/// and a temp cache root.
pub struct ResolverConfig {
    /// The launcher's own version; the manifest must match it (anti-rollback).
    pub expected_version: String,
    /// The host platform alias (`HOST_PLATFORM` in production).
    pub platform: String,
    /// The release-download base URL (no trailing slash), pinned to the
    /// plugin's own `v{version}` tag in production.
    pub base_url: String,
    /// Where cached binaries live (resolved via `cache_root` in production).
    pub cache_root: PathBuf,
}

impl ResolverConfig {
    /// Build the production config from the base URL and resolved cache root.
    #[must_use]
    pub fn production(base_url: String, cache_root: PathBuf) -> Self {
        Self {
            expected_version: env!("CARGO_PKG_VERSION").to_owned(),
            platform: HOST_PLATFORM.to_owned(),
            base_url,
            cache_root,
        }
    }
}

/// The real `ResolveBinary` adapter: cache-hit re-verify (with replace-in-place
/// self-heal), else fetch → verify → cache.
pub struct FetchVerifyCacheResolver {
    config: ResolverConfig,
    keys: TrustedKeys,
    fetcher: Fetcher,
}

impl FetchVerifyCacheResolver {
    /// # Errors
    ///
    /// If the HTTP client cannot be built.
    pub fn new(
        config: ResolverConfig,
        keys: TrustedKeys,
    ) -> Result<Self, ResolutionError> {
        let fetcher = Fetcher::new().map_err(|detail| {
            ResolutionError::CacheRootUnavailable { detail }
        })?;
        Ok(Self {
            config,
            keys,
            fetcher,
        })
    }

    /// Construct with a caller-supplied fetcher (tests inject a tiny backoff).
    #[must_use]
    pub const fn with_fetcher(
        config: ResolverConfig,
        keys: TrustedKeys,
        fetcher: Fetcher,
    ) -> Self {
        Self {
            config,
            keys,
            fetcher,
        }
    }

    fn reverify(&self, cached: &CachedBinary) -> Result<(), ResolutionError> {
        let bytes = std::fs::read(&cached.path).map_err(|error| {
            ResolutionError::Cache {
                path: cached.path.clone(),
                detail: error.to_string(),
            }
        })?;
        let signature = std::fs::read_to_string(&cached.signature_path)
            .map_err(|error| ResolutionError::Cache {
                path: cached.signature_path.clone(),
                detail: error.to_string(),
            })?;
        verifier::verify_binary(
            &file_name(&cached.path),
            &bytes,
            &cached.sha256,
            &signature,
            &self.keys,
        )
    }

    /// Fetch, signature-verify, and version/schema-validate the release
    /// manifest — the shared front half of resolution and help synthesis.
    ///
    /// # Errors
    ///
    /// A [`ResolutionError`] on fetch/signature/version/schema failure.
    pub fn load_manifest(&self) -> Result<Manifest, ResolutionError> {
        let base = &self.config.base_url;
        let manifest_url = format!("{base}/manifest.json");
        let manifest_bytes =
            self.fetcher.get(&manifest_url).map_err(|error| {
                fetch_error(&error, &self.config.platform, &manifest_url, true)
            })?;
        let signature_url = format!("{base}/manifest.minisig");
        let signature_bytes =
            self.fetcher.get(&signature_url).map_err(|error| {
                fetch_error(&error, &self.config.platform, &signature_url, true)
            })?;
        let signature = String::from_utf8_lossy(&signature_bytes);
        // Signature over the RAW bytes first; only then parse + gate.
        verifier::verify_manifest(&manifest_bytes, &signature, &self.keys)?;
        Manifest::parse_and_validate(
            &manifest_bytes,
            &self.config.expected_version,
        )
    }

    fn fetch_verify_store(
        &self,
        name: &str,
    ) -> Result<PathBuf, ResolutionError> {
        let base = &self.config.base_url;
        let manifest = self.load_manifest()?;
        let asset_name = format!("{name}-{}", self.config.platform);
        let asset_url = format!("{base}/{asset_name}");
        let entry = manifest
            .platform_entry(name, &self.config.platform)
            .ok_or_else(|| ResolutionError::AssetNotFound {
                target: asset_name.clone(),
                url: asset_url.clone(),
            })?;
        let expected_sha = entry.bare_sha256(&asset_name)?;

        let bytes = self.fetcher.get(&asset_url).map_err(|error| {
            fetch_error(&error, &asset_name, &asset_url, false)
        })?;
        verifier::verify_binary(
            &asset_name,
            &bytes,
            expected_sha,
            &entry.signature,
            &self.keys,
        )?;

        let cached = cache::store(
            &self.config.cache_root,
            name,
            &self.config.expected_version,
            expected_sha,
            &bytes,
            &entry.signature,
        )?;
        Ok(cached.path)
    }
}

impl ResolveBinary for FetchVerifyCacheResolver {
    fn resolve(
        &self,
        command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError> {
        let name = command.name.to_str().ok_or_else(|| {
            ResolutionError::Unresolved {
                name: command.name.clone(),
            }
        })?;

        if let Some(cached) = cache::find(
            &self.config.cache_root,
            name,
            &self.config.expected_version,
        ) {
            // Re-verify before every exec, including cache hits: the cache dir
            // is user-writable, so a poisoned entry (binary + matching sha) must
            // still be caught by the signature. On failure, self-heal by
            // replacing in place — fetch+verify a clean copy and rename it over
            // the corrupt entry (never evicting before a verified successor
            // exists). If the re-fetch fails (e.g. offline), report the distinct
            // "corrupt AND re-fetch failed" diagnostic, not a plain miss.
            match self.reverify(&cached) {
                Ok(()) => return Ok(cached.path),
                Err(_) => {
                    return self.fetch_verify_store(name).map_err(|error| {
                        ResolutionError::CorruptCacheAndRefetchFailed {
                            asset: name.to_owned(),
                            detail: error.to_string(),
                        }
                    });
                }
            }
        }
        self.fetch_verify_store(name)
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn fetch_error(
    error: &FetchError,
    target: &str,
    url: &str,
    is_manifest: bool,
) -> ResolutionError {
    match error {
        FetchError::NotFound if is_manifest => {
            ResolutionError::ReleaseUnavailable {
                target: target.to_owned(),
                url: url.to_owned(),
            }
        }
        FetchError::NotFound => ResolutionError::AssetNotFound {
            target: target.to_owned(),
            url: url.to_owned(),
        },
        FetchError::Unreachable(_) => ResolutionError::Fetch {
            target: target.to_owned(),
            url: url.to_owned(),
        },
    }
}
