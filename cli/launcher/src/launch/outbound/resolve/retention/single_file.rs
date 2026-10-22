//! Single-file prune: the sub-binary and launcher entries, shim copies and
//! staging files at the top of a relocated cache root.

use std::collections::{BTreeMap, BTreeSet};

use super::root_entries::RootEntry;
use super::version_claims::FreshVersions;

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

/// What the running plugin's own shim source hashes to, per platform.
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

    /// A platform never hashed is treated as unhashable, so its copies stay.
    fn runs(&self, platform: &str, digest: &str) -> bool {
        match self.sources.get(platform) {
            Some(ShimSource::Digest(running)) => running == digest,
            Some(ShimSource::Unhashable) | None => true,
        }
    }
}

/// The pids of bootstraps still running, by the staging names they wrote.
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

/// The one place an entry's protection is decided.
///
/// A sub-binary's version is matched as a suffix of its stem and never split
/// out of the name, since both names and versions may hold hyphens.
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

/// Whether a stale version claim goes: beside a fresh claim on the same
/// version, or once every entry of that version this prune listed is gone.
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
