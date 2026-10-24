use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use super::super::filesystem::FileKind;
use super::super::prune_report::{Count, Removals, UnreadableClaims};
use super::super::retention_windows::is_aged;
use super::super::verifier::sha256_hex;
use super::ports::CachePorts;
use super::root_entries::{RootEntry, RootListing};
use super::root_lock::HeldRootLock;
use super::version_claims::{
    self, ClaimsListing, FreshVersions, ListedClaim, VERSION_CLAIMS_DIR,
};

const SIGNATURE_SUFFIX: &str = ".minisig";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Remove,
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    Protected,
    Unprotected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    Aged,
    Young,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShimSource {
    Digest(String),
    Unhashable,
}

/// The shim copy each platform's bootstraps run, even on another host
/// sharing the root; removing it would race a live bootstrap into
/// `fail_integrity`, which leaves a lasting unverified-launcher notice.
#[derive(Debug, Default)]
pub struct RunningShims {
    sources: BTreeMap<String, ShimSource>,
}

impl RunningShims {
    #[must_use]
    pub fn with(mut self, platform: &str, source: ShimSource) -> Self {
        self.sources.insert(platform.to_owned(), source);
        self
    }

    fn runs(&self, platform: &str, digest: &str) -> bool {
        match self.sources.get(platform) {
            Some(ShimSource::Digest(running)) => running == digest,
            Some(ShimSource::Unhashable) | None => true,
        }
    }
}

#[derive(Debug, Default)]
pub struct LiveBootstraps {
    pids: BTreeSet<i32>,
}

impl LiveBootstraps {
    #[must_use]
    pub fn with(mut self, pid: i32) -> Self {
        self.pids.insert(pid);
        self
    }

    fn holds(&self, pid: i32) -> bool {
        self.pids.contains(&pid)
    }
}

pub struct ProtectionContext {
    pub fresh: FreshVersions,
    pub running_shims: RunningShims,
    pub live_bootstraps: LiveBootstraps,
}

#[must_use]
pub fn protection(
    entry: &RootEntry<'_>,
    context: &ProtectionContext,
) -> Protection {
    let protected = match *entry {
        RootEntry::SubBinary { stem } => context
            .fresh
            .iter()
            .any(|version| is_stem_of_version(stem, version)),
        RootEntry::LauncherEntry { version } => context.fresh.contains(version),
        RootEntry::ShimCopy { platform, digest } => {
            context.running_shims.runs(platform, digest)
        }
        RootEntry::ShimStaging { pid } | RootEntry::LauncherStaging { pid } => {
            context.live_bootstraps.holds(pid)
        }
        RootEntry::StoreStaging
        | RootEntry::Probe
        | RootEntry::BootstrapLock
        | RootEntry::Unrecognised => false,
    };
    if protected {
        Protection::Protected
    } else {
        Protection::Unprotected
    }
}

/// Matched as a suffix of the stem and never split out of it, since both
/// names and versions may hold hyphens.
fn is_stem_of_version(stem: &str, version: &str) -> bool {
    stem.rsplit_once('-')
        .and_then(|(named, _digest)| named.strip_suffix(version))
        .is_some_and(|name| name.len() > 1 && name.ends_with('-'))
}

/// Store staging ignores protection: the launcher wrote it, so its mtime is
/// local and its age alone says it was abandoned.
#[must_use]
pub const fn disposition(
    entry: &RootEntry<'_>,
    protection: Protection,
    age: Age,
) -> Disposition {
    let removable = match entry {
        RootEntry::SubBinary { .. }
        | RootEntry::LauncherEntry { .. }
        | RootEntry::ShimCopy { .. }
        | RootEntry::ShimStaging { .. }
        | RootEntry::LauncherStaging { .. } => {
            matches!(protection, Protection::Unprotected)
                && matches!(age, Age::Aged)
        }
        RootEntry::StoreStaging => matches!(age, Age::Aged),
        RootEntry::Probe
        | RootEntry::BootstrapLock
        | RootEntry::Unrecognised => false,
    };
    if removable {
        Disposition::Remove
    } else {
        Disposition::Keep
    }
}

#[must_use]
pub const fn stale_claim_disposition(
    has_other_fresh_claim: bool,
    every_entry_removed: bool,
) -> Disposition {
    if has_other_fresh_claim || every_entry_removed {
        Disposition::Remove
    } else {
        Disposition::Keep
    }
}

/// Safe beside a concurrent `store`.
///
/// It removes only entries in a listing taken before claims are read: an
/// entry stored after the listing is never touched, and one stored before it
/// was preceded by its claim.
pub struct SingleFilePrune<'a> {
    pub root: &'a Path,
    pub running_version: &'a str,
    pub plugin_root: Option<&'a Path>,
    pub claim_window: Duration,
    pub ports: &'a CachePorts,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SingleFileOutcome {
    pub removals: Removals,
    pub unreadable_claims: Option<UnreadableClaims>,
}

struct RemovalAttempt {
    of: String,
    removed: bool,
}

#[derive(Default)]
struct VersionedRemovals {
    stems: Vec<RemovalAttempt>,
    launchers: Vec<RemovalAttempt>,
}

impl VersionedRemovals {
    fn every_entry_removed(&self, version: &str) -> bool {
        self.stems
            .iter()
            .filter(|attempt| is_stem_of_version(&attempt.of, version))
            .chain(
                self.launchers
                    .iter()
                    .filter(|attempt| attempt.of == version),
            )
            .all(|attempt| attempt.removed)
    }
}

impl SingleFilePrune<'_> {
    pub fn run(
        &self,
        listing: &RootListing,
        _held: &HeldRootLock,
    ) -> SingleFileOutcome {
        let entries = listing.classified();
        let running_shims = self.running_shims(&entries);
        let live_bootstraps = self.live_bootstraps(&entries);
        let mut outcome = SingleFileOutcome::default();
        let claims = match version_claims::list(
            self.root,
            self.ports.filesystem.as_ref(),
            self.ports.ownership.as_ref(),
        ) {
            ClaimsListing::Read(claims) => Some(claims),
            ClaimsListing::Unreadable(kind) => {
                tracing::warn!(
                    ?kind,
                    "claims unreadable; kept every versioned entry"
                );
                outcome.unreadable_claims = Some(UnreadableClaims {
                    directory: self.root.join(VERSION_CLAIMS_DIR),
                    kind,
                });
                None
            }
        };
        let claimed = claims.as_deref().map(|claims| {
            FreshVersions::within(
                claims,
                self.claim_window,
                self.ports.clock.as_ref(),
            )
        });
        let context = ProtectionContext {
            fresh: claimed
                .clone()
                .unwrap_or_default()
                .including(self.running_version),
            running_shims,
            live_bootstraps,
        };
        let versioned = self.remove_entries(
            &entries,
            &context,
            claims.is_some(),
            &mut outcome,
        );
        if let (Some(claims), Some(claimed)) = (claims, claimed) {
            self.remove_stale_claims(
                &claims,
                &claimed,
                &versioned,
                &mut outcome,
            );
        }
        outcome
    }

    fn running_shims(&self, entries: &[(&str, RootEntry<'_>)]) -> RunningShims {
        let platforms: BTreeSet<&str> = entries
            .iter()
            .filter_map(|(_, entry)| match entry {
                RootEntry::ShimCopy { platform, .. } => Some(*platform),
                _ => None,
            })
            .collect();
        platforms
            .into_iter()
            .fold(RunningShims::default(), |shims, platform| {
                shims.with(platform, self.shim_source(platform))
            })
    }

    fn shim_source(&self, platform: &str) -> ShimSource {
        self.plugin_root
            .map(|plugin_root| {
                plugin_root
                    .join("bin")
                    .join(format!("accelerator-verify-{platform}"))
            })
            .and_then(|source| self.ports.filesystem.read(&source).ok())
            .map_or(ShimSource::Unhashable, |bytes| {
                ShimSource::Digest(sha256_hex(&bytes))
            })
    }

    fn live_bootstraps(
        &self,
        entries: &[(&str, RootEntry<'_>)],
    ) -> LiveBootstraps {
        entries
            .iter()
            .filter_map(|(_, entry)| match entry {
                RootEntry::ShimStaging { pid }
                | RootEntry::LauncherStaging { pid } => Some(*pid),
                _ => None,
            })
            .collect::<BTreeSet<i32>>()
            .into_iter()
            .filter(|pid| self.ports.bootstrap_holders.is_alive(*pid))
            .fold(LiveBootstraps::default(), LiveBootstraps::with)
    }

    fn remove_entries(
        &self,
        entries: &[(&str, RootEntry<'_>)],
        context: &ProtectionContext,
        claims_read: bool,
        outcome: &mut SingleFileOutcome,
    ) -> VersionedRemovals {
        let mut versioned = VersionedRemovals::default();
        for (name, entry) in entries {
            let is_versioned = matches!(
                entry,
                RootEntry::SubBinary { .. } | RootEntry::LauncherEntry { .. }
            );
            if is_versioned && !claims_read {
                continue;
            }
            let removed =
                self.remove_if_disposable(name, entry, context, outcome);
            match *entry {
                RootEntry::SubBinary { stem } => {
                    versioned.stems.push(RemovalAttempt {
                        of: stem.to_owned(),
                        removed,
                    });
                }
                RootEntry::LauncherEntry { version } => {
                    versioned.launchers.push(RemovalAttempt {
                        of: version.to_owned(),
                        removed,
                    });
                }
                _ => {}
            }
        }
        versioned
    }

    fn remove_if_disposable(
        &self,
        name: &str,
        entry: &RootEntry<'_>,
        context: &ProtectionContext,
        outcome: &mut SingleFileOutcome,
    ) -> bool {
        let protection = protection(entry, context);
        if protection == Protection::Protected {
            return false;
        }
        let path = self.root.join(name);
        let signature = has_signature(entry)
            .then(|| self.root.join(format!("{name}{SIGNATURE_SUFFIX}")));
        let age = self.age_of(&path, signature.as_deref());
        if disposition(entry, protection, age) == Disposition::Keep {
            return false;
        }
        let filesystem = self.ports.filesystem.as_ref();
        let removals = &mut outcome.removals;
        let rule = rule_of(entry);
        if let Some(signature) = &signature {
            if !removals
                .remove_file(filesystem, signature, rule, Count::WithItsEntry)
                .left_nothing()
            {
                return false;
            }
        }
        removals
            .remove_file(filesystem, &path, rule, Count::Once)
            .left_nothing()
    }

    fn age_of(&self, path: &Path, signature: Option<&Path>) -> Age {
        let now = self.ports.clock.now_seconds();
        let aged = std::iter::once(path).chain(signature).all(|file| {
            self.ports.filesystem.entry(file).is_ok_and(|entry| {
                entry.kind == FileKind::File
                    && is_aged(entry.modified_seconds, now)
            })
        });
        if aged {
            Age::Aged
        } else {
            Age::Young
        }
    }

    fn remove_stale_claims(
        &self,
        claims: &[ListedClaim],
        claimed: &FreshVersions,
        versioned: &VersionedRemovals,
        outcome: &mut SingleFileOutcome,
    ) {
        for claim in claims.iter().filter(|claim| self.is_stale(claim)) {
            if stale_claim_disposition(
                claimed.contains(&claim.version),
                versioned.every_entry_removed(&claim.version),
            ) == Disposition::Keep
            {
                continue;
            }
            if !self.is_still_stale(&claim.path) {
                continue;
            }
            outcome.removals.remove_file(
                self.ports.filesystem.as_ref(),
                &claim.path,
                "stale version claim",
                Count::Once,
            );
        }
    }

    fn is_stale(&self, claim: &ListedClaim) -> bool {
        claim.valid
            && !FreshVersions::within(
                std::slice::from_ref(claim),
                self.claim_window,
                self.ports.clock.as_ref(),
            )
            .contains(&claim.version)
    }

    /// Re-read immediately before the unlink, so a refresh landing between
    /// the listing and here keeps its claim.
    fn is_still_stale(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };
        let Some((version, launcher_id)) =
            version_claims::parse_claim_name(name)
        else {
            return false;
        };
        self.ports.filesystem.entry(path).is_ok_and(|entry| {
            self.is_stale(&ListedClaim {
                version: version.to_owned(),
                launcher_id: launcher_id.to_owned(),
                path: path.to_path_buf(),
                valid: entry.kind == FileKind::File
                    && entry.owner == self.ports.ownership.effective_uid(),
                modified_seconds: entry.modified_seconds,
            })
        })
    }
}

const fn has_signature(entry: &RootEntry<'_>) -> bool {
    matches!(
        entry,
        RootEntry::SubBinary { .. } | RootEntry::LauncherEntry { .. }
    )
}

const fn rule_of(entry: &RootEntry<'_>) -> &'static str {
    match entry {
        RootEntry::SubBinary { .. } => "unclaimed aged sub-binary",
        RootEntry::LauncherEntry { .. } => "unclaimed aged launcher entry",
        RootEntry::ShimCopy { .. } => "aged shim copy of another release",
        RootEntry::ShimStaging { .. } | RootEntry::LauncherStaging { .. } => {
            "aged staging of a finished bootstrap"
        }
        RootEntry::StoreStaging => "aged store staging",
        RootEntry::Probe
        | RootEntry::BootstrapLock
        | RootEntry::Unrecognised => "kept",
    }
}

#[cfg(test)]
mod tests {
    use super::super::root_entries::RootEntry;
    use super::super::version_claims::FreshVersions;
    use super::{
        disposition, protection, stale_claim_disposition, Age, Disposition,
        LiveBootstraps, Protection, ProtectionContext, RunningShims,
        ShimSource,
    };

    const DIGEST: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const OTHER_DIGEST: &str =
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    fn context() -> ProtectionContext {
        ProtectionContext {
            fresh: FreshVersions::default().including("1.24.0-pre.78"),
            running_shims: RunningShims::default()
                .with("linux-x64", ShimSource::Digest(DIGEST.to_owned()))
                .with("darwin-arm64", ShimSource::Unhashable),
            live_bootstraps: LiveBootstraps::default().with(4242),
        }
    }

    fn protects(entry: &RootEntry<'_>) -> bool {
        protection(entry, &context()) == Protection::Protected
    }

    #[test]
    fn a_sub_binary_is_protected_by_a_fresh_version_suffix() {
        let kept = format!("x-1.24.0-pre.78-{DIGEST}");
        let split = format!("x-y-1.24.0-{DIGEST}");
        let near = format!("x-11.24.0-pre.78-{DIGEST}");
        assert!(protects(&RootEntry::SubBinary { stem: &kept }));
        assert!(!protects(&RootEntry::SubBinary { stem: &split }));
        assert!(!protects(&RootEntry::SubBinary { stem: &near }));
    }

    #[test]
    fn a_launcher_entry_is_protected_by_its_fresh_version() {
        assert!(protects(&RootEntry::LauncherEntry {
            version: "1.24.0-pre.78"
        }));
        assert!(!protects(&RootEntry::LauncherEntry {
            version: "1.24.0-pre.77"
        }));
    }

    #[test]
    fn a_shim_copy_is_protected_when_it_is_its_platforms_running_shim() {
        assert!(protects(&RootEntry::ShimCopy {
            platform: "linux-x64",
            digest: DIGEST
        }));
        assert!(!protects(&RootEntry::ShimCopy {
            platform: "linux-x64",
            digest: OTHER_DIGEST
        }));
    }

    #[test]
    fn every_copy_is_protected_on_a_platform_whose_source_is_unhashable() {
        assert!(protects(&RootEntry::ShimCopy {
            platform: "darwin-arm64",
            digest: OTHER_DIGEST
        }));
        assert!(protects(&RootEntry::ShimCopy {
            platform: "linux-arm64",
            digest: OTHER_DIGEST
        }));
    }

    #[test]
    fn staging_is_protected_while_its_bootstrap_lives() {
        assert!(protects(&RootEntry::ShimStaging { pid: 4242 }));
        assert!(protects(&RootEntry::LauncherStaging { pid: 4242 }));
        assert!(!protects(&RootEntry::ShimStaging { pid: 4343 }));
        assert!(!protects(&RootEntry::LauncherStaging { pid: 4343 }));
    }

    #[test]
    fn residue_and_unrecognised_names_are_never_protected() {
        for entry in [
            RootEntry::StoreStaging,
            RootEntry::Probe,
            RootEntry::BootstrapLock,
            RootEntry::Unrecognised,
        ] {
            assert!(!protects(&entry), "{entry:?}");
        }
    }

    fn removes(
        entry: &RootEntry<'_>,
        protection: Protection,
        age: Age,
    ) -> bool {
        disposition(entry, protection, age) == Disposition::Remove
    }

    #[test]
    fn a_protectable_entry_goes_only_unprotected_and_aged() {
        for entry in [
            RootEntry::SubBinary { stem: "x" },
            RootEntry::LauncherEntry { version: "1" },
            RootEntry::ShimCopy {
                platform: "linux-x64",
                digest: DIGEST,
            },
            RootEntry::ShimStaging { pid: 1 },
            RootEntry::LauncherStaging { pid: 1 },
        ] {
            assert!(removes(&entry, Protection::Unprotected, Age::Aged));
            assert!(!removes(&entry, Protection::Unprotected, Age::Young));
            assert!(!removes(&entry, Protection::Protected, Age::Aged));
            assert!(!removes(&entry, Protection::Protected, Age::Young));
        }
    }

    #[test]
    fn store_staging_goes_once_aged() {
        assert!(removes(
            &RootEntry::StoreStaging,
            Protection::Unprotected,
            Age::Aged
        ));
        assert!(!removes(
            &RootEntry::StoreStaging,
            Protection::Unprotected,
            Age::Young
        ));
    }

    #[test]
    fn the_residue_sweeps_and_unrecognised_names_are_kept() {
        for entry in [
            RootEntry::Probe,
            RootEntry::BootstrapLock,
            RootEntry::Unrecognised,
        ] {
            assert!(!removes(&entry, Protection::Unprotected, Age::Aged));
        }
    }

    #[test]
    fn a_stale_claim_goes_beside_a_fresh_one_or_after_all_its_entries() {
        let cases = [
            (true, true, Disposition::Remove),
            (true, false, Disposition::Remove),
            (false, true, Disposition::Remove),
            (false, false, Disposition::Keep),
        ];
        for (other_fresh, all_removed, expected) in cases {
            assert_eq!(
                stale_claim_disposition(other_fresh, all_removed),
                expected,
                "other fresh: {other_fresh}, all removed: {all_removed}"
            );
        }
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used, clippy::panic)]
mod run_tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use super::super::super::filesystem::testing::ReportingFilesystem;
    use super::super::super::filesystem::{
        CacheFilesystem as _, EntryMetadata, SystemFilesystem,
    };
    use super::super::super::ownership::{Ownership as _, ProcessOwnership};
    use super::super::super::retention_windows::CLAIM_WINDOW;
    use super::super::super::single_flight::LockPatience;
    use super::super::super::verifier::sha256_hex;
    use super::super::super::HOST_PLATFORM;
    use super::super::root_entries::RootListing;
    use super::super::root_lock::HeldRootLock;
    use super::super::testing::{aged, now, young, LiveHolders, Ports, Root};
    use super::super::version_claims::VERSION_CLAIMS_DIR;
    use super::{SingleFileOutcome, SingleFilePrune};

    const RUNNING: &str = "9.9.9";
    const DIGEST: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const OTHER_DIGEST: &str =
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    const LAUNCHER_ID: &str = "0123456789abcdef";
    const OTHER_ID: &str = "fedcba9876543210";
    const LIVE: i32 = 4242;
    const DEAD: i32 = 4343;
    const DAY: u64 = 24 * 60 * 60;

    fn other_platform() -> &'static str {
        if HOST_PLATFORM == "linux-x64" {
            "darwin-arm64"
        } else {
            "linux-x64"
        }
    }

    struct Seeded {
        root: Root,
        now: u64,
    }

    impl Seeded {
        fn new() -> Self {
            Self {
                root: Root::new(),
                now: now(),
            }
        }

        fn sub_binary(&self, version: &str, modified: u64) -> String {
            self.named_sub_binary(&format!("vcs-{version}-{DIGEST}"), modified)
        }

        fn named_sub_binary(&self, stem: &str, modified: u64) -> String {
            self.root.file(stem, b"binary", modified);
            self.root.file(&format!("{stem}.minisig"), b"sig", modified);
            stem.to_owned()
        }

        fn launcher(&self, version: &str, modified: u64) -> String {
            let name =
                format!("accelerator-launcher-{version}-{HOST_PLATFORM}");
            self.named_sub_binary(&name, modified)
        }

        fn claims(&self) -> PathBuf {
            self.root.join(VERSION_CLAIMS_DIR)
        }

        fn claim(&self, version: &str, id: &str, modified: u64) -> PathBuf {
            let claims = self.claims();
            if !claims.exists() {
                std::fs::create_dir(&claims).expect("claims");
                super::super::testing::set_mode(&claims, 0o700);
            }
            let path = claims.join(format!("{version}.{id}"));
            std::fs::write(&path, b"").expect("claim");
            SystemFilesystem
                .set_modified(&path, modified)
                .expect("claim mtime");
            path
        }

        fn has(&self, name: &str) -> bool {
            self.root.exists(name)
        }

        fn has_entry(&self, stem: &str) -> bool {
            let entry = self.has(stem);
            assert_eq!(
                entry,
                self.has(&format!("{stem}.minisig")),
                "{stem} and its signature diverged"
            );
            entry
        }

        fn ports(&self) -> Ports {
            Ports::at(self.now)
        }

        fn prune(&self) -> SingleFileOutcome {
            self.prune_with(self.ports(), None, CLAIM_WINDOW)
        }

        fn prune_with(
            &self,
            ports: Ports,
            plugin_root: Option<&Path>,
            claim_window: Duration,
        ) -> SingleFileOutcome {
            let ports = ports.build();
            let held = HeldRootLock::acquire(
                self.root.path(),
                LockPatience::Wait,
                &ports,
            )
            .expect("root lock")
            .expect("taken");
            let listing = RootListing::read(
                self.root.path(),
                ports.filesystem.as_ref(),
                ports.steps.as_ref(),
            )
            .expect("listed");
            SingleFilePrune {
                root: self.root.path(),
                running_version: RUNNING,
                plugin_root,
                claim_window,
                ports: &ports,
            }
            .run(&listing, &held)
        }
    }

    #[test]
    fn only_the_unclaimed_versions_entries_are_removed() {
        let root = Seeded::new();
        let a_binary = root.sub_binary("1.0.0", aged(root.now));
        let a_launcher = root.launcher("1.0.0", aged(root.now));
        let b_binary = root.sub_binary("2.0.0", aged(root.now));
        let b_launcher = root.launcher("2.0.0", aged(root.now));
        root.claim("2.0.0", LAUNCHER_ID, root.now);

        let outcome = root.prune();

        assert!(!root.has_entry(&a_binary));
        assert!(!root.has_entry(&a_launcher));
        assert!(root.has_entry(&b_binary));
        assert!(root.has_entry(&b_launcher));
        assert_eq!(outcome.removals.reclaimed, 2);
    }

    #[test]
    fn two_claimed_versions_lose_nothing() {
        let root = Seeded::new();
        let entries = [
            root.sub_binary("1.0.0", aged(root.now)),
            root.sub_binary("2.0.0", aged(root.now)),
        ];
        root.claim("1.0.0", LAUNCHER_ID, root.now);
        root.claim("2.0.0", OTHER_ID, root.now);

        assert_eq!(root.prune().removals.reclaimed, 0);
        assert!(entries.iter().all(|entry| root.has_entry(entry)));
    }

    #[test]
    fn a_symlinked_and_a_foreign_owned_claim_protect_nothing() {
        let root = Seeded::new();
        let a = root.sub_binary("1.0.0", aged(root.now));
        let b = root.sub_binary("2.0.0", aged(root.now));
        let target = root.root.file("target", b"", root.now);
        root.claim("2.0.0", LAUNCHER_ID, root.now);
        let foreign = root.claims().join(format!("2.0.0.{LAUNCHER_ID}"));
        std::os::unix::fs::symlink(
            &target,
            root.claims().join(format!("1.0.0.{OTHER_ID}")),
        )
        .expect("symlinked claim");
        let filesystem = ReportingFilesystem::over_system().reporting(
            &foreign,
            EntryMetadata {
                owner: ProcessOwnership::default().effective_uid() + 1,
                ..SystemFilesystem.entry(&foreign).expect("claim metadata")
            },
        );

        root.prune_with(
            Ports {
                filesystem: Arc::new(filesystem),
                ..root.ports()
            },
            None,
            CLAIM_WINDOW,
        );

        assert!(!root.has_entry(&a));
        assert!(!root.has_entry(&b));
        assert!(target.exists());
    }

    #[test]
    fn pre_claim_entries_younger_than_the_backstop_survive() {
        let root = Seeded::new();
        let launcher = root.launcher("3.0.0", young(root.now));
        let young_binary = root.sub_binary("3.0.0", young(root.now));
        let old_binary = root
            .named_sub_binary(&format!("git-3.0.0-{DIGEST}"), aged(root.now));

        root.prune();

        assert!(root.has_entry(&launcher));
        assert!(root.has_entry(&young_binary));
        assert!(!root.has_entry(&old_binary));
    }

    #[test]
    fn an_entry_goes_only_once_its_signature_is_aged_too() {
        let root = Seeded::new();
        let stem = root.sub_binary("1.0.0", aged(root.now));
        SystemFilesystem
            .set_modified(
                &root.root.join(&format!("{stem}.minisig")),
                young(root.now),
            )
            .expect("signature mtime");

        root.prune();

        assert!(root.has_entry(&stem));
    }

    #[test]
    fn a_version_is_matched_by_suffix_never_by_splitting_the_name() {
        let root = Seeded::new();
        let kept = root.named_sub_binary(
            &format!("x-1.24.0-pre.78-{DIGEST}"),
            aged(root.now),
        );
        let removed = root
            .named_sub_binary(&format!("x-y-1.24.0-{DIGEST}"), aged(root.now));
        root.claim("1.24.0-pre.78", LAUNCHER_ID, root.now);

        root.prune();

        assert!(root.has_entry(&kept));
        assert!(!root.has_entry(&removed));
    }

    #[test]
    fn stale_claims_go_independently_and_a_last_claim_goes_after_its_entries() {
        let root = Seeded::new();
        let stale =
            root.claim("1.0.0", LAUNCHER_ID, root.now - CLAIM_WINDOW.as_secs());
        let fresh = root.claim("1.0.0", OTHER_ID, root.now);
        let kept = root.sub_binary("1.0.0", aged(root.now));
        let last =
            root.claim("2.0.0", LAUNCHER_ID, root.now - CLAIM_WINDOW.as_secs());
        let removed = root.sub_binary("2.0.0", aged(root.now));

        root.prune();

        assert!(!stale.exists());
        assert!(fresh.exists());
        assert!(root.has_entry(&kept));
        assert!(!last.exists());
        assert!(!root.has_entry(&removed));
    }

    #[test]
    fn a_last_stale_claim_survives_an_unremoved_entry() {
        let root = Seeded::new();
        let last =
            root.claim("2.0.0", LAUNCHER_ID, root.now - CLAIM_WINDOW.as_secs());
        let young_entry = root.sub_binary("2.0.0", young(root.now));

        root.prune();

        assert!(last.exists());
        assert!(root.has_entry(&young_entry));
    }

    #[test]
    fn plugin_staging_entries_are_removed_only_once_aged() {
        let root = Seeded::new();
        let aged_names = [
            format!(".tmp-vcs-1.0.0-{DIGEST}-11-0"),
            format!(".tmp-vcs-1.0.0-{DIGEST}-11-0.minisig"),
        ];
        let young_name = format!(".tmp-vcs-1.0.0-{DIGEST}-12-0");
        for name in &aged_names {
            root.root.file(name, b"", aged(root.now));
        }
        root.root.file(&young_name, b"", young(root.now));

        assert_eq!(root.prune().removals.reclaimed, 2);
        assert!(aged_names.iter().all(|name| !root.has(name)));
        assert!(root.has(&young_name));
    }

    fn holding(root: &Seeded) -> Ports {
        Ports {
            bootstrap_holders: Arc::new(LiveHolders([LIVE].into())),
            ..root.ports()
        }
    }

    #[test]
    fn launcher_staging_with_a_live_pid_survives_however_old() {
        let root = Seeded::new();
        let live = format!(".tmp-launcher-{LIVE}");
        root.root.file(&live, b"", root.now - DAY);
        root.root
            .file(&format!("{live}.minisig"), b"", root.now - DAY);

        root.prune_with(holding(&root), None, CLAIM_WINDOW);

        assert!(root.has(&live));
        assert!(root.has(&format!("{live}.minisig")));
    }

    #[test]
    fn launcher_staging_with_a_dead_pid_goes_once_aged() {
        let root = Seeded::new();
        let aged_dead = format!(".tmp-launcher-{DEAD}");
        let young_dead = format!(".tmp-launcher-{}", DEAD + 1);
        root.root.file(&aged_dead, b"", aged(root.now));
        root.root.file(&young_dead, b"", young(root.now));

        root.prune_with(holding(&root), None, CLAIM_WINDOW);

        assert!(!root.has(&aged_dead));
        assert!(root.has(&young_dead));
    }

    fn shim_staging(platform: &str, pid: i32) -> String {
        format!("accelerator-verify-{platform}-{DIGEST}.staging.{pid}")
    }

    #[test]
    fn shim_staging_with_a_live_pid_survives_however_old() {
        let root = Seeded::new();
        let live = shim_staging(HOST_PLATFORM, LIVE);
        root.root.file(&live, b"", root.now - DAY);

        root.prune_with(holding(&root), None, CLAIM_WINDOW);

        assert!(root.has(&live));
    }

    #[test]
    fn shim_staging_with_a_dead_pid_goes_once_aged() {
        let root = Seeded::new();
        let aged_dead = shim_staging(HOST_PLATFORM, DEAD);
        let young_dead = shim_staging(HOST_PLATFORM, DEAD + 1);
        root.root.file(&aged_dead, b"", aged(root.now));
        root.root.file(&young_dead, b"", young(root.now));

        root.prune_with(holding(&root), None, CLAIM_WINDOW);

        assert!(!root.has(&aged_dead));
        assert!(root.has(&young_dead));
    }

    struct Plugin {
        dir: tempfile::TempDir,
    }

    impl Plugin {
        fn with_sources(platforms: &[&str]) -> Self {
            let dir = tempfile::tempdir().expect("plugin root");
            std::fs::create_dir(dir.path().join("bin")).expect("bin");
            for platform in platforms {
                std::fs::write(
                    Self::source_in(dir.path(), platform),
                    format!("shim for {platform}"),
                )
                .expect("shim source");
            }
            Self { dir }
        }

        fn source_in(root: &Path, platform: &str) -> PathBuf {
            root.join("bin")
                .join(format!("accelerator-verify-{platform}"))
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }

        fn digest_of(&self, platform: &str) -> String {
            sha256_hex(
                &std::fs::read(Self::source_in(self.path(), platform))
                    .expect("source"),
            )
        }
    }

    fn shim_copy(platform: &str, digest: &str) -> String {
        format!("accelerator-verify-{platform}-{digest}")
    }

    #[test]
    fn the_running_plugins_shim_copies_survive_for_every_platform() {
        let root = Seeded::new();
        let plugin = Plugin::with_sources(&[HOST_PLATFORM, other_platform()]);
        let copies = [HOST_PLATFORM, other_platform()]
            .map(|platform| shim_copy(platform, &plugin.digest_of(platform)));
        for copy in &copies {
            root.root.file(copy, b"", aged(root.now));
        }

        root.prune_with(root.ports(), Some(plugin.path()), CLAIM_WINDOW);

        assert!(copies.iter().all(|copy| root.has(copy)));
    }

    #[test]
    fn stale_shim_copies_go_on_every_platform() {
        let plugin = Plugin::with_sources(&[HOST_PLATFORM, other_platform()]);
        let stale = [
            shim_copy(HOST_PLATFORM, OTHER_DIGEST),
            shim_copy(other_platform(), OTHER_DIGEST),
            shim_copy(HOST_PLATFORM, &plugin.digest_of(other_platform())),
        ];

        let root = Seeded::new();
        for copy in &stale {
            root.root.file(copy, b"", aged(root.now));
        }
        root.prune_with(root.ports(), Some(plugin.path()), CLAIM_WINDOW);
        assert!(stale.iter().all(|copy| !root.has(copy)));

        let fresh = Seeded::new();
        for copy in &stale {
            fresh.root.file(copy, b"", young(fresh.now));
        }
        fresh.prune_with(fresh.ports(), Some(plugin.path()), CLAIM_WINDOW);
        assert!(stale.iter().all(|copy| fresh.has(copy)));
    }

    #[test]
    fn an_unreadable_shim_source_keeps_that_platforms_copies() {
        let plugin = Plugin::with_sources(&[HOST_PLATFORM]);
        let root = Seeded::new();
        let unhashable = shim_copy(other_platform(), OTHER_DIGEST);
        let hashable = shim_copy(HOST_PLATFORM, OTHER_DIGEST);
        root.root.file(&unhashable, b"", aged(root.now));
        root.root.file(&hashable, b"", aged(root.now));

        root.prune_with(root.ports(), Some(plugin.path()), CLAIM_WINDOW);

        assert!(root.has(&unhashable));
        assert!(!root.has(&hashable));

        let no_plugin_root = Seeded::new();
        no_plugin_root
            .root
            .file(&hashable, b"", aged(no_plugin_root.now));
        no_plugin_root.prune();
        assert!(no_plugin_root.has(&hashable));
    }

    #[test]
    fn the_running_version_survives_older_than_zero() {
        let root = Seeded::new();
        let entry = root.sub_binary(RUNNING, aged(root.now));
        let launcher = root.launcher(RUNNING, aged(root.now));
        let claim = root.claim(RUNNING, LAUNCHER_ID, root.now);

        root.prune_with(root.ports(), None, Duration::ZERO);

        assert!(root.has_entry(&entry));
        assert!(root.has_entry(&launcher));
        assert!(claim.exists());
    }

    #[test]
    fn the_running_version_survives_without_a_valid_claim() {
        let root = Seeded::new();
        let entry = root.sub_binary(RUNNING, aged(root.now));

        root.prune();

        assert!(root.has_entry(&entry));
    }

    #[test]
    fn older_than_narrows_the_version_claim_window() {
        for (window, survives) in
            [(Duration::from_secs(DAY), false), (CLAIM_WINDOW, true)]
        {
            let root = Seeded::new();
            let entry = root.sub_binary("1.0.0", aged(root.now));
            root.claim("1.0.0", LAUNCHER_ID, root.now - 2 * DAY);

            root.prune_with(root.ports(), None, window);

            assert_eq!(root.has_entry(&entry), survives, "{window:?}");
        }
    }

    #[test]
    fn older_than_beyond_the_window_widens_it() {
        let root = Seeded::new();
        let entry = root.sub_binary("1.0.0", aged(root.now));
        let claim = root.claim("1.0.0", LAUNCHER_ID, root.now - 20 * DAY);

        root.prune_with(root.ports(), None, Duration::from_secs(30 * DAY));

        assert!(root.has_entry(&entry));
        assert!(claim.exists());
    }

    #[test]
    fn a_claim_exactly_at_the_window_is_stale() {
        let root = Seeded::new();
        let entry = root.sub_binary("1.0.0", aged(root.now));
        let claim =
            root.claim("1.0.0", LAUNCHER_ID, root.now - CLAIM_WINDOW.as_secs());

        root.prune();

        assert!(!root.has_entry(&entry));
        assert!(!claim.exists());
    }

    #[test]
    fn an_unreadable_claims_directory_removes_no_entries() {
        let root = Seeded::new();
        let entry = root.sub_binary("1.0.0", aged(root.now));
        let staging = format!(".tmp-vcs-1.0.0-{DIGEST}-11-0");
        root.root.file(&staging, b"", aged(root.now));
        root.claim("2.0.0", LAUNCHER_ID, root.now);
        let claims = root.claims();
        let filesystem = ReportingFilesystem::over_system().reporting(
            &claims,
            EntryMetadata {
                owner: ProcessOwnership::default().effective_uid() + 1,
                ..SystemFilesystem.entry(&claims).expect("claims metadata")
            },
        );

        let outcome = root.prune_with(
            Ports {
                filesystem: Arc::new(filesystem),
                ..root.ports()
            },
            None,
            CLAIM_WINDOW,
        );

        assert!(root.has_entry(&entry));
        assert!(!root.has(&staging));
        let unreadable = outcome.unreadable_claims.expect("reported");
        assert_eq!(unreadable.directory, claims);
        assert_eq!(unreadable.kind, std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn a_held_tree_temp_generation_is_untouched_by_single_file_prune() {
        let root = Seeded::new();
        let trees = root.root.dir("trees", 0o700);
        let temp = trees.join(format!(
            ".tmp-browser-{HOST_PLATFORM}-{DIGEST}-1-0123456789abcdef"
        ));
        std::fs::create_dir(&temp).expect("temp generation");
        root.sub_binary("1.0.0", aged(root.now));

        root.prune();

        assert!(temp.exists());
    }

    #[test]
    fn the_reclaimed_count_counts_an_entry_and_its_signature_once() {
        let root = Seeded::new();
        root.sub_binary("1.0.0", aged(root.now));
        root.claim("1.0.0", LAUNCHER_ID, root.now - CLAIM_WINDOW.as_secs());

        assert_eq!(root.prune().removals.reclaimed, 2);
    }
}
