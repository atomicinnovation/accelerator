//! The concrete tree resolver: `acquire`/`query`/`materialise`/`verify` over
//! the leaf modules.

use std::collections::BTreeSet;
use std::io::Read as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::launch::core::tree::{
    AcquireSealedTree, AcquiredTree, Clock, Discrepancy, MaterialiseTree,
    SealedTree, TreeError, TreeReport, VerifyTree,
};
use crate::launch::core::tree_entry::{EntryKind, ExtractionLimits};

use super::super::cache_root;
use super::super::fetcher::Fetcher;
use super::super::keys::TrustedKeys;
use super::super::manifest::Manifest;
use super::attestation::Attestation;
use super::layout::{
    generation_name, generation_prefix, is_privately_owned,
    validated_generation, TreePaths, LAYOUT_VERSION,
};
use super::lease::{hold_shared_lease, take_single_flight};
use super::table::{FileTable, TABLE_NAME};
use super::{download, extract, pins, reap, seal};

/// Where the launcher's expected `(artifact, platform) -> digest` map comes
/// from.
///
/// Production always uses [`ExpectedDigests::Compiled`] — the rollback defence
/// is that the digest is baked into the binary from the reviewed anchor. The
/// injected variant is a test seam only, the same shape as `Fetcher`'s
/// backoff-injecting constructor, so an end-to-end test can pin a digest it
/// controls without a real release.
pub enum ExpectedDigests {
    /// Read the compiled-in map.
    Compiled,
    /// A test-supplied `(artifact, platform) -> digest` map.
    Fixed(std::collections::BTreeMap<(String, String), String>),
}

impl ExpectedDigests {
    fn digest_for(&self, artifact: &str, platform: &str) -> Option<String> {
        match self {
            Self::Compiled => {
                pins::expected_digest_on(artifact, platform).map(str::to_owned)
            }
            Self::Fixed(map) => map
                .get(&(artifact.to_owned(), platform.to_owned()))
                .cloned(),
        }
    }
}

/// Everything the resolver needs to reach the release and the cache root.
pub struct TreeResolver<'a> {
    pub cache_root: PathBuf,
    pub base_url: String,
    pub platform: String,
    pub expected_version: String,
    pub keys: &'a TrustedKeys,
    pub fetcher: &'a Fetcher,
    pub clock: &'a dyn Clock,
    /// A per-install identity for the retention claim, so two installs sharing
    /// a cache root each write their own claim file.
    pub launcher_id: String,
    /// Production is always [`ExpectedDigests::Compiled`]; the injected variant
    /// is a test seam.
    pub expected_digests: ExpectedDigests,
}

impl TreeResolver<'_> {
    fn paths(&self) -> TreePaths {
        TreePaths::under(&self.cache_root)
    }

    /// The digest this launcher expects for `artifact`, or a miss-shaped `None`
    /// when this platform publishes none.
    fn expected_digest(&self, artifact: &str) -> Option<String> {
        self.expected_digests.digest_for(artifact, &self.platform)
    }

    /// Steps 1-5: the side-effect-free query behind both `query` and `acquire`.
    ///
    /// Returns the validated generation name and the verified attestation, or
    /// `None` for any miss. No I/O error propagates: within design a broken
    /// cache path is a miss, not a failure, so the crawl re-materialises rather
    /// than aborting.
    fn locate(&self, artifact: &str, digest: &str) -> Option<Located> {
        let paths = self.paths();

        // 1. The pointer: owned by us, not a symlink, not group/world-writable,
        //    before its contents are read.
        let pointer = paths.pointer(artifact, &self.platform, digest);
        if !is_privately_owned(&pointer) {
            return None;
        }
        let contents = std::fs::read_to_string(&pointer).ok()?;

        // 2. The contents become a path only after the full grammar validates.
        let generation = validated_generation(
            &contents,
            artifact,
            &self.platform,
            digest,
            LAYOUT_VERSION,
        )
        .ok()?;

        // 3. The generation directory: a real directory, owned, not writable by
        //    others, and not a symlink pointing at one.
        let generation_dir = paths.generation(&generation);
        let identity = directory_identity(&generation_dir)?;

        // 4-5. The attestation, verified under the embedded key and checked
        //      field by field against what is being resolved.
        let document = std::fs::read(paths.attestation(&generation)).ok()?;
        let signature =
            std::fs::read_to_string(paths.attestation_signature(&generation))
                .ok()?;
        let attestation =
            Attestation::verified(&document, &signature, self.keys).ok()?;
        attestation.matches(artifact, &self.platform, digest).ok()?;

        Some(Located {
            generation,
            generation_dir,
            identity,
            attestation,
        })
    }

    /// Materialise into a fresh generation under the single-flight lock, which
    /// the caller already holds.
    fn materialise_locked(
        &self,
        artifact: &str,
        digest: &str,
    ) -> Result<SealedTree, TreeError> {
        let paths = self.paths();

        // 1. Reuse scan, before any network access: an existing generation at
        //    this digest and layout that still verifies is a hit.
        if let Some(existing) = self.reuse_scan(artifact, digest) {
            self.publish_pointer(&paths, artifact, digest, &existing)?;
            self.write_claim(&paths, digest);
            return Ok(self.sealed(&paths, artifact, &existing, digest));
        }

        // 2. The manifest, whose digest for this artifact must be the one this
        //    launcher expects — a disagreement is a refusal, not an instruction
        //    to fetch something else.
        let manifest = self.load_manifest()?;
        let entry = manifest
            .artifact_platform_entry(artifact, &self.platform)
            .ok_or_else(|| TreeError::Attestation {
                detail: format!(
                    "the manifest publishes no {artifact} for {}",
                    self.platform
                ),
            })?;
        if entry.sha256 != digest {
            return Err(TreeError::UnexpectedDigest {
                artifact: artifact.to_owned(),
                expected: digest.to_owned(),
                found: entry.sha256.clone(),
            });
        }

        // 3. Free-space precheck against archive + extracted copy.
        self.check_free_space(
            &paths,
            entry.archive_size + entry.uncompressed_size,
        )?;

        // 4. Stream the archive to a digest-named temp file.
        let temp_archive = paths.temp_archive(artifact, &self.platform, digest);
        let archive_url = format!(
            "{}/{}",
            self.base_url,
            asset_name(artifact, &self.platform)
        );
        let streamed = download::stream_archive(
            self.fetcher,
            &archive_url,
            &temp_archive,
            entry.archive_size,
        )?;

        // 5. The attestation and the archive verification.
        let attestation =
            self.fetch_and_check_attestation(artifact, digest, &manifest)?;
        download::verify_archive_file(
            &temp_archive,
            &streamed,
            digest,
            &entry.signature,
            self.keys,
        )?;

        // 6. Extract into a fresh temp generation, verifying each member as it
        //    is written.
        let suffix = generation_suffix();
        let generation = generation_name(
            artifact,
            &self.platform,
            digest,
            LAYOUT_VERSION,
            &suffix,
        );
        let temp_generation = paths.temp_generation(&generation);
        std::fs::create_dir_all(&temp_generation).map_err(|error| {
            TreeError::Extraction {
                detail: format!("cannot create the temp generation: {error}"),
            }
        })?;
        let archive = std::fs::File::open(&temp_archive).map_err(|error| {
            TreeError::Extraction {
                detail: format!(
                    "cannot reopen the archive to extract: {error}"
                ),
            }
        })?;
        extract::extract_archive(
            flate2::read::GzDecoder::new(archive),
            &temp_generation,
            &ExtractionLimits {
                uncompressed_size: attestation.uncompressed_size,
                entry_count: attestation.entry_count,
            },
        )?;

        // 7. Seal, then 8. write the sidecars from the release's own bytes.
        seal::seal_tree(&temp_generation)?;
        write_private(
            &paths.attestation(&generation),
            &self.reread(&paths, artifact, digest)?,
        )?;

        // The attestation document and signature are the release's bytes, not
        // locally synthesised.
        self.write_sidecars(&paths, &generation, artifact, digest)?;

        // 9. Hold the lease before the rename, so the 9->10 window cannot look
        //    like crash residue to a concurrent prune.
        let _lease = hold_shared_lease(&paths.lease(&generation))?;

        // 10. Rename into place — fresh by construction, so a collision is an
        //     internal error rather than a merge.
        let final_generation = paths.generation(&generation);
        std::fs::rename(&temp_generation, &final_generation).map_err(
            |error| TreeError::Extraction {
                detail: format!("cannot publish the generation: {error}"),
            },
        )?;
        let _ = std::fs::remove_file(&temp_archive);

        // 11. Publish the pointer, last, so a crash before here leaves only
        //     reclaimable garbage.
        self.publish_pointer(&paths, artifact, digest, &generation)?;
        self.write_claim(&paths, digest);

        Ok(self.sealed(&paths, artifact, &generation, digest))
    }

    fn reuse_scan(&self, artifact: &str, digest: &str) -> Option<String> {
        let paths = self.paths();
        let prefix =
            generation_prefix(artifact, &self.platform, digest, LAYOUT_VERSION);
        let entries = std::fs::read_dir(paths.root()).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.starts_with(&prefix) || has_sidecar_suffix(name) {
                continue;
            }
            if self.locate(artifact, digest).is_some() {
                return Some(name.to_owned());
            }
        }
        None
    }

    fn load_manifest(&self) -> Result<Manifest, TreeError> {
        let base = &self.base_url;
        let manifest_bytes = self
            .fetcher
            .get(&format!("{base}/manifest.json"))
            .map_err(|error| TreeError::Attestation {
                detail: format!("cannot fetch the manifest: {error:?}"),
            })?;
        let signature = self
            .fetcher
            .get(&format!("{base}/manifest.minisig"))
            .map_err(|error| TreeError::Attestation {
            detail: format!("cannot fetch the manifest signature: {error:?}"),
        })?;
        let signature = String::from_utf8_lossy(&signature);
        if !self.keys.verifies(&manifest_bytes, &signature) {
            return Err(TreeError::Attestation {
                detail: "the manifest signature does not verify".to_owned(),
            });
        }
        Manifest::parse_and_validate(&manifest_bytes, &self.expected_version)
            .map_err(|error| TreeError::Attestation {
                detail: format!("the manifest is unusable: {error}"),
            })
    }

    fn fetch_and_check_attestation(
        &self,
        artifact: &str,
        digest: &str,
        _manifest: &Manifest,
    ) -> Result<Attestation, TreeError> {
        let base = &self.base_url;
        let asset = asset_name(artifact, &self.platform);
        let bytes = download::fetch_attestation(
            self.fetcher,
            &format!("{base}/{asset}.sealed"),
            &format!("{base}/{asset}.sealed.sig"),
        )?;
        let attestation = Attestation::verified(
            &bytes.document,
            &bytes.signature,
            self.keys,
        )?;
        attestation.matches(artifact, &self.platform, digest)?;
        Ok(attestation)
    }

    /// Reread the freshly fetched attestation document so its own bytes — not a
    /// locally synthesised copy — are written into the sealed tree's sidecar.
    fn reread(
        &self,
        _paths: &TreePaths,
        artifact: &str,
        _digest: &str,
    ) -> Result<Vec<u8>, TreeError> {
        let base = &self.base_url;
        let asset = asset_name(artifact, &self.platform);
        self.fetcher
            .get(&format!("{base}/{asset}.sealed"))
            .map_err(|error| TreeError::Attestation {
                detail: format!("cannot reread the attestation: {error:?}"),
            })
    }

    fn write_sidecars(
        &self,
        paths: &TreePaths,
        generation: &str,
        artifact: &str,
        _digest: &str,
    ) -> Result<(), TreeError> {
        let base = &self.base_url;
        let asset = asset_name(artifact, &self.platform);
        let signature = self
            .fetcher
            .get(&format!("{base}/{asset}.sealed.sig"))
            .map_err(|error| TreeError::Attestation {
                detail: format!(
                    "cannot reread the attestation signature: {error:?}"
                ),
            })?;
        write_private(&paths.attestation_signature(generation), &signature)
    }

    fn publish_pointer(
        &self,
        paths: &TreePaths,
        artifact: &str,
        digest: &str,
        generation: &str,
    ) -> Result<(), TreeError> {
        let pointer = paths.pointer(artifact, &self.platform, digest);
        let temp = paths.root().join(format!(".tmp-{generation}.ref"));
        write_private(&temp, generation.as_bytes())?;
        std::fs::rename(&temp, &pointer).map_err(|error| {
            let _ = std::fs::remove_file(&temp);
            TreeError::Pointer {
                detail: format!("cannot publish the pointer: {error}"),
            }
        })
    }

    fn write_claim(&self, paths: &TreePaths, digest: &str) {
        // Best-effort: a populated cache root may be read-only on a warm start,
        // and the hit path must keep working there.
        let claims = paths.claims();
        if std::fs::create_dir_all(&claims).is_err() {
            return;
        }
        let claim = claims.join(format!("{digest}.{}", self.launcher_id));
        let _ = write_private(&claim, b"");
    }

    fn check_free_space(
        &self,
        paths: &TreePaths,
        needed: u64,
    ) -> Result<(), TreeError> {
        let Some(available) = available_bytes(paths.root(), &self.cache_root)
        else {
            return Ok(());
        };
        if available < needed {
            return Err(TreeError::DiskShortfall { needed, available });
        }
        Ok(())
    }

    fn sealed(
        &self,
        paths: &TreePaths,
        artifact: &str,
        generation: &str,
        digest: &str,
    ) -> SealedTree {
        let _ = self;
        SealedTree {
            artifact: artifact.to_owned(),
            path: paths.generation(generation),
            lease_path: paths.lease(generation),
            digest: digest.to_owned(),
        }
    }
}

impl AcquireSealedTree for TreeResolver<'_> {
    fn query(&self, artifact: &str) -> Result<Option<SealedTree>, TreeError> {
        let Some(digest) = self.expected_digest(artifact) else {
            return Ok(None);
        };
        let digest = digest.as_str();
        let paths = self.paths();
        Ok(self.locate(artifact, digest).map(|located| {
            self.sealed(&paths, artifact, &located.generation, digest)
        }))
    }

    fn acquire(
        &self,
        artifact: &str,
    ) -> Result<Option<AcquiredTree>, TreeError> {
        let Some(digest) = self.expected_digest(artifact) else {
            return Ok(None);
        };
        let digest = digest.as_str();
        let paths = self.paths();

        // The lease-then-recheck is retried once, so a prune reclaiming between
        // the directory check and the lease hands back a miss rather than a
        // lease held on an unlinked inode.
        for _ in 0..2 {
            let Some(located) = self.locate(artifact, digest) else {
                return Ok(None);
            };
            let Ok(lease) =
                hold_shared_lease(&paths.lease(&located.generation))
            else {
                return Ok(None);
            };
            match directory_identity(&located.generation_dir) {
                Some(now) if now == located.identity => {
                    self.write_claim(&paths, digest);
                    return Ok(Some(AcquiredTree {
                        tree: self.sealed(
                            &paths,
                            artifact,
                            &located.generation,
                            digest,
                        ),
                        lease: Box::new(lease)
                            as Box<dyn crate::launch::core::tree::HeldLease>,
                    }));
                }
                // The generation changed under us; drop the lease and retry.
                _ => drop(lease),
            }
        }
        Ok(None)
    }
}

impl MaterialiseTree for TreeResolver<'_> {
    fn materialise(&self, artifact: &str) -> Result<SealedTree, TreeError> {
        let Some(digest) = self.expected_digest(artifact) else {
            return Err(TreeError::Attestation {
                detail: format!(
                    "this launcher publishes no {artifact} for {}",
                    self.platform
                ),
            });
        };
        let digest = digest.as_str();

        // Probe the cache root exactly once, before any cache-root write, so an
        // unwritable or full root surfaces as the intended downgrade rather than
        // an opaque lock-file error.
        cache_root::verify_writable(&self.cache_root).map_err(|error| {
            TreeError::Lease {
                detail: format!("the cache root is unusable: {error}"),
            }
        })?;
        let paths = self.paths();
        ensure_trees_dir(&paths)?;

        let _lock = take_single_flight(
            &paths.single_flight_lock(artifact, &self.platform),
        )?;

        // The loser re-runs the query and materialises only if still needed.
        if let Some(located) = self.locate(artifact, digest) {
            return Ok(self.sealed(
                &paths,
                artifact,
                &located.generation,
                digest,
            ));
        }

        let sealed = self.materialise_locked(artifact, digest)?;

        // Reap residue while holding the lock the reaper requires.
        let mut keep = BTreeSet::new();
        keep.insert(digest.to_owned());
        let _ = reap::reap_orphans(&paths, self.clock, &keep);

        Ok(sealed)
    }
}

impl VerifyTree for TreeResolver<'_> {
    fn verify(&self, artifact: &str) -> Result<TreeReport, TreeError> {
        let Some(digest) = self.expected_digest(artifact) else {
            return Err(TreeError::Attestation {
                detail: format!("unknown artifact {artifact}"),
            });
        };
        let digest = digest.as_str();
        let Some(located) = self.locate(artifact, digest) else {
            return Ok(TreeReport {
                artifact: artifact.to_owned(),
                findings: vec![(String::new(), Discrepancy::Missing)],
            });
        };

        let table = self.load_and_check_table(&located)?;
        let findings = walk_against_table(&located.generation_dir, &table);
        Ok(TreeReport {
            artifact: artifact.to_owned(),
            findings,
        })
    }
}

impl TreeResolver<'_> {
    /// Read the `.files` table and confirm its digest against the signed
    /// `table_sha256` before trusting a single row — otherwise a table edited
    /// after materialisation to match a substituted member would make every
    /// tree-side detection vacuous.
    fn load_and_check_table(
        &self,
        located: &Located,
    ) -> Result<FileTable, TreeError> {
        let _ = self;
        let table_path = located.generation_dir.join(TABLE_NAME);
        let bytes = std::fs::read(&table_path).map_err(|error| {
            TreeError::Extraction {
                detail: format!("cannot read the file table: {error}"),
            }
        })?;
        let digest = {
            let hash: [u8; 32] = Sha256::digest(&bytes).into();
            hex(&hash)
        };
        if digest != located.attestation.table_sha256 {
            return Err(TreeError::Extraction {
                detail: "the file table does not match its signed digest"
                    .to_owned(),
            });
        }
        FileTable::parse(&bytes)
    }
}

/// The verified generation, ready for the lease step or the verify walk.
struct Located {
    generation: String,
    generation_dir: PathBuf,
    identity: (u64, u64),
    attestation: Attestation,
}

fn has_sidecar_suffix(name: &str) -> bool {
    [".ref", ".sealed", ".sealed.sig", ".lease"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

fn asset_name(artifact: &str, platform: &str) -> String {
    format!("accelerator-{artifact}-{platform}.tar.gz")
}

/// A directory's `(st_dev, st_ino)`, or `None` when the path is absent, a
/// symlink, not a directory, wrongly owned, or group/world-writable.
fn directory_identity(path: &Path) -> Option<(u64, u64)> {
    if !is_privately_owned(path) {
        return None;
    }
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_dir() {
        return None;
    }
    Some((metadata.dev(), metadata.ino()))
}

fn generation_suffix() -> String {
    // 16 hex characters from the platform CSPRNG: fresh by construction, so the
    // rename never lands on an existing target and there is no already-present
    // branch to get right.
    let mut bytes = [0_u8; 8];
    rand::fill(&mut bytes);
    hex(&bytes)
}

fn ensure_trees_dir(paths: &TreePaths) -> Result<(), TreeError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::create_dir_all(paths.root()).map_err(|error| {
        TreeError::Lease {
            detail: format!("cannot create the trees directory: {error}"),
        }
    })?;
    // The launcher owns trees/ and chmods it into compliance, so the strict
    // ownership check is one it can always satisfy even under a umask-relaxed
    // cache root.
    let _ = std::fs::set_permissions(
        paths.root(),
        std::fs::Permissions::from_mode(0o700),
    );
    Ok(())
}

fn walk_against_table(
    root: &Path,
    table: &FileTable,
) -> Vec<(String, Discrepancy)> {
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    walk_dir(root, root, table, &mut findings, &mut seen);
    // Anything the table describes that the walk did not see is missing.
    for (path, _) in table.iter() {
        if !seen.contains(path) {
            findings.push((path.clone(), Discrepancy::Missing));
        }
    }
    findings
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    table: &FileTable,
    findings: &mut Vec<(String, Discrepancy)>,
    seen: &mut BTreeSet<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let Some(relative) = relative.to_str() else {
            continue;
        };
        if relative == TABLE_NAME {
            // The table is an archive member with no row describing itself.
            continue;
        }
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        seen.insert(relative.to_owned());
        match table.row(relative) {
            None => {
                findings.push((relative.to_owned(), Discrepancy::Unexpected));
            }
            Some(row) => {
                check_entry(relative, &path, &metadata, row, findings);
            }
        }
        if metadata.is_dir() {
            walk_dir(root, &path, table, findings, seen);
        }
    }
}

fn check_entry(
    relative: &str,
    path: &Path,
    metadata: &std::fs::Metadata,
    row: &super::table::TableRow,
    findings: &mut Vec<(String, Discrepancy)>,
) {
    use std::os::unix::fs::PermissionsExt as _;
    let expected_mode =
        seal::sealed_mode(row.kind == EntryKind::Directory, row.mode);
    match row.kind {
        EntryKind::Directory if !metadata.is_dir() => {
            findings.push((relative.to_owned(), Discrepancy::Unexpected));
        }
        EntryKind::Symlink => {
            let target = std::fs::read_link(path)
                .ok()
                .and_then(|target| target.to_str().map(str::to_owned));
            if target.as_deref() != row.link_target.as_deref() {
                findings.push((
                    relative.to_owned(),
                    Discrepancy::LinkTarget {
                        expected: row.link_target.clone().unwrap_or_default(),
                        found: target.unwrap_or_default(),
                    },
                ));
            }
        }
        EntryKind::File => {
            if metadata.len() != row.size {
                findings.push((
                    relative.to_owned(),
                    Discrepancy::Size {
                        expected: row.size,
                        found: metadata.len(),
                    },
                ));
            } else if file_digest(path).as_deref() != row.sha256.as_deref() {
                findings.push((relative.to_owned(), Discrepancy::Digest));
            }
            let mode = metadata.permissions().mode() & 0o777;
            if mode != expected_mode {
                findings.push((
                    relative.to_owned(),
                    Discrepancy::Mode {
                        expected: expected_mode,
                        found: mode,
                    },
                ));
            }
        }
        EntryKind::Directory => {
            let mode = metadata.permissions().mode() & 0o777;
            if mode != expected_mode {
                findings.push((
                    relative.to_owned(),
                    Discrepancy::Mode {
                        expected: expected_mode,
                        found: mode,
                    },
                ));
            }
        }
        _ => {}
    }
}

fn file_digest(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let hash: [u8; 32] = hasher.finalize().into();
    Some(hex(&hash))
}

fn available_bytes(trees_root: &Path, cache_root: &Path) -> Option<u64> {
    // Prefer the trees root, falling back to the cache root before it exists.
    let target = if trees_root.exists() {
        trees_root
    } else {
        cache_root
    };
    let stat = rustix::fs::statvfs(target).ok()?;
    Some(stat.f_bavail.saturating_mul(stat.f_frsize))
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<(), TreeError> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| TreeError::Pointer {
            detail: format!("cannot write {}: {error}", path.display()),
        })?;
    file.write_all(bytes).map_err(|error| TreeError::Pointer {
        detail: format!("cannot write {}: {error}", path.display()),
    })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}
