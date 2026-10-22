//! The grammar of the names the plugin writes at the top of a cache root.
//!
//! A name is recognised only by its full shape, and anything else is kept, so
//! a newer release's names and a user's own files are never mistaken for
//! residue.

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use kernel::TEMP_PREFIX;

use super::super::filesystem::CacheFilesystem;
use super::super::prune_report::{PruneStep, PruneSteps};
use super::super::KNOWN_PLATFORMS;
use super::residue::parse_pid;

const SIGNATURE_SUFFIX: &str = ".minisig";
const LAUNCHER_PREFIX: &str = "accelerator-launcher-";
const SHIM_PREFIX: &str = "accelerator-verify-";
const SHIM_STAGING_INFIX: &str = ".staging.";
const LAUNCHER_STAGING_PREFIX: &str = "launcher-";
const PROBE_PREFIX: &str = ".accelerator-probe-";
const BOOTSTRAP_LOCK_PREFIX: &str = ".accelerator-lock-";
const DIGEST_LENGTH: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootEntry<'a> {
    SubBinary { stem: &'a str },
    LauncherEntry { version: &'a str },
    ShimCopy { platform: &'a str, digest: &'a str },
    ShimStaging { pid: i32 },
    StoreStaging,
    LauncherStaging { pid: i32 },
    Probe,
    BootstrapLock,
    Unrecognised,
}

/// One listing of a cache root's top level, which single-file prune and the
/// residue sweep both act on; a name that is not UTF-8 is no name the plugin
/// writes, so it is left out and kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RootListing {
    names: Vec<String>,
}

impl RootListing {
    /// # Errors
    ///
    /// If the root cannot be listed.
    pub fn read(
        root: &Path,
        filesystem: &dyn CacheFilesystem,
        steps: &dyn PruneSteps,
    ) -> io::Result<Self> {
        steps.reached(PruneStep::AboutToList);
        let names = filesystem.list(root)?;
        steps.reached(PruneStep::Listed);
        Ok(Self {
            names: names
                .into_iter()
                .filter_map(|name| name.into_string().ok())
                .collect(),
        })
    }

    /// Every listed name beside what it is.
    pub fn classified(&self) -> Vec<(&str, RootEntry<'_>)> {
        let listed: BTreeSet<&str> =
            self.names.iter().map(String::as_str).collect();
        self.names
            .iter()
            .map(|name| (name.as_str(), classify(name, &listed)))
            .collect()
    }
}

/// What `name` is, given every name `listed` beside it.
///
/// An entry with a signature is recognised only while its `.minisig` is
/// listed too, and a `.minisig` is never recognised alone: it goes with its
/// entry.
#[must_use]
pub fn classify<'a>(name: &'a str, listed: &BTreeSet<&str>) -> RootEntry<'a> {
    shim_entry(name)
        .or_else(|| launcher_entry(name, listed))
        .or_else(|| sub_binary(name, listed))
        .or_else(|| staging(name))
        .or_else(|| bootstrap_residue(name))
        .unwrap_or(RootEntry::Unrecognised)
}

fn shim_entry(name: &str) -> Option<RootEntry<'_>> {
    let rest = name.strip_prefix(SHIM_PREFIX)?;
    let (copy, pid) = match rest.split_once(SHIM_STAGING_INFIX) {
        Some((copy, pid)) => (copy, Some(staging_pid(pid)?)),
        None => (rest, None),
    };
    let (platform, digest) = copy.rsplit_once('-')?;
    if !is_known_platform(platform) || !is_digest(digest) {
        return None;
    }
    Some(pid.map_or(RootEntry::ShimCopy { platform, digest }, |pid| {
        RootEntry::ShimStaging { pid }
    }))
}

fn launcher_entry<'a>(
    name: &'a str,
    listed: &BTreeSet<&str>,
) -> Option<RootEntry<'a>> {
    let rest = name.strip_prefix(LAUNCHER_PREFIX)?;
    let version = KNOWN_PLATFORMS
        .iter()
        .find_map(|platform| rest.strip_suffix(platform)?.strip_suffix('-'))?;
    (!version.is_empty() && has_signature(name, listed))
        .then_some(RootEntry::LauncherEntry { version })
}

fn sub_binary<'a>(
    name: &'a str,
    listed: &BTreeSet<&str>,
) -> Option<RootEntry<'a>> {
    (is_digest_suffixed_stem(name)
        && !name.starts_with('.')
        && has_signature(name, listed))
    .then_some(RootEntry::SubBinary { stem: name })
}

fn staging(name: &str) -> Option<RootEntry<'_>> {
    let rest = name.strip_prefix(TEMP_PREFIX)?;
    let unsigned = rest.strip_suffix(SIGNATURE_SUFFIX).unwrap_or(rest);
    if let Some(pid) = unsigned
        .strip_prefix(LAUNCHER_STAGING_PREFIX)
        .and_then(staging_pid)
    {
        return Some(RootEntry::LauncherStaging { pid });
    }
    let (stem_and_pid, sequence) = unsigned.rsplit_once('-')?;
    let (stem, pid) = stem_and_pid.rsplit_once('-')?;
    (is_digits(pid) && is_digits(sequence) && is_digest_suffixed_stem(stem))
        .then_some(RootEntry::StoreStaging)
}

fn bootstrap_residue(name: &str) -> Option<RootEntry<'_>> {
    if let Some(rest) = name.strip_prefix(PROBE_PREFIX) {
        let (pid, sequence) = rest.split_once('-').unwrap_or((rest, "0"));
        return (is_digits(pid) && is_digits(sequence))
            .then_some(RootEntry::Probe);
    }
    let platform = name.strip_prefix(BOOTSTRAP_LOCK_PREFIX)?;
    is_known_platform(platform).then_some(RootEntry::BootstrapLock)
}

fn staging_pid(candidate: &str) -> Option<i32> {
    is_digits(candidate).then(|| parse_pid(candidate)).flatten()
}

fn has_signature(name: &str, listed: &BTreeSet<&str>) -> bool {
    listed.contains(format!("{name}{SIGNATURE_SUFFIX}").as_str())
}

/// `<x>-<64 lowercase hex>` with `<x>` non-empty.
fn is_digest_suffixed_stem(candidate: &str) -> bool {
    candidate
        .rsplit_once('-')
        .is_some_and(|(prefix, digest)| !prefix.is_empty() && is_digest(digest))
}

fn is_digest(candidate: &str) -> bool {
    candidate.len() == DIGEST_LENGTH
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_digits(candidate: &str) -> bool {
    !candidate.is_empty() && candidate.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_known_platform(candidate: &str) -> bool {
    KNOWN_PLATFORMS.contains(&candidate)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{classify, RootEntry};

    const DIGEST: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn alone(name: &str) -> RootEntry<'_> {
        classify(name, &BTreeSet::new())
    }

    fn signed(name: &str) -> RootEntry<'_> {
        let signature = format!("{name}.minisig");
        classify(name, &BTreeSet::from([signature.as_str()]))
    }

    #[test]
    fn top_level_names_are_fixed_across_launcher_versions() {
        let sub_binary = format!("vcs-1.24.0-pre.83-{DIGEST}");
        assert_eq!(
            signed(&sub_binary),
            RootEntry::SubBinary { stem: &sub_binary }
        );
        assert_eq!(
            signed("accelerator-launcher-1.24.0-pre.83-darwin-arm64"),
            RootEntry::LauncherEntry {
                version: "1.24.0-pre.83"
            }
        );
        assert_eq!(
            alone(&format!("accelerator-verify-linux-x64-{DIGEST}")),
            RootEntry::ShimCopy {
                platform: "linux-x64",
                digest: DIGEST
            }
        );
        assert_eq!(
            alone(&format!("accelerator-verify-linux-x64-{DIGEST}.staging.42")),
            RootEntry::ShimStaging { pid: 42 }
        );
        for name in [
            format!(".tmp-vcs-1.24.0-{DIGEST}-4242-0"),
            format!(".tmp-vcs-1.24.0-{DIGEST}-4242-17.minisig"),
        ] {
            assert_eq!(alone(&name), RootEntry::StoreStaging, "{name}");
        }
        assert_eq!(
            alone(".tmp-launcher-4242"),
            RootEntry::LauncherStaging { pid: 4242 }
        );
        assert_eq!(
            alone(".tmp-launcher-4242.minisig"),
            RootEntry::LauncherStaging { pid: 4242 }
        );
        assert_eq!(alone(".accelerator-probe-4242"), RootEntry::Probe);
        assert_eq!(alone(".accelerator-probe-4242-3"), RootEntry::Probe);
        assert_eq!(
            alone(".accelerator-lock-darwin-x64"),
            RootEntry::BootstrapLock
        );
    }

    #[test]
    fn every_known_platform_names_a_launcher_entry_shim_and_lock() {
        for platform in super::KNOWN_PLATFORMS {
            assert_eq!(
                signed(&format!("accelerator-launcher-2.0.0-{platform}")),
                RootEntry::LauncherEntry { version: "2.0.0" }
            );
            assert!(matches!(
                alone(&format!("accelerator-verify-{platform}-{DIGEST}")),
                RootEntry::ShimCopy { .. }
            ));
            assert_eq!(
                alone(&format!(".accelerator-lock-{platform}")),
                RootEntry::BootstrapLock
            );
        }
    }

    #[test]
    fn an_entry_without_its_listed_signature_is_unrecognised() {
        for name in [
            format!("vcs-1.24.0-{DIGEST}"),
            "accelerator-launcher-1.24.0-linux-x64".to_owned(),
        ] {
            assert_eq!(alone(&name), RootEntry::Unrecognised, "{name}");
        }
    }

    #[test]
    fn a_signature_is_never_recognised_alone() {
        for name in [
            format!("vcs-1.24.0-{DIGEST}.minisig"),
            "accelerator-launcher-1.24.0-linux-x64.minisig".to_owned(),
        ] {
            assert_eq!(signed(&name), RootEntry::Unrecognised, "{name}");
        }
    }

    #[test]
    fn unrecognised_shapes_are_kept() {
        let names = [
            "claims".to_owned(),
            "trees".to_owned(),
            ".accelerator-unverified.log".to_owned(),
            ".tmp-launcher-0".to_owned(),
            ".tmp-launcher--1".to_owned(),
            ".tmp-launcher-abc".to_owned(),
            ".tmp-launcher-".to_owned(),
            format!("accelerator-verify-linux-x64-{DIGEST}.staging.abc"),
            format!("accelerator-verify-linux-x64-{DIGEST}.staging.0"),
            format!("accelerator-verify-linux-x64-{}", &DIGEST[1..]),
            format!("accelerator-verify-linux-x64-{}", DIGEST.to_uppercase()),
            ".accelerator-probe-".to_owned(),
            ".accelerator-probe-abc".to_owned(),
            ".accelerator-probe-1-2-3".to_owned(),
            format!(".tmp-vcs-1.24.0-{DIGEST}-4242"),
            format!(".tmp-vcs-1.24.0-{DIGEST}-x-0"),
            format!(".tmp-{DIGEST}-1-0"),
            ".tmp-foreign".to_owned(),
        ];
        for name in names {
            assert_eq!(signed(&name), RootEntry::Unrecognised, "{name}");
        }
    }

    #[test]
    fn an_unknown_platform_launcher_entry_shim_or_lock_is_unrecognised() {
        assert_eq!(
            signed("accelerator-launcher-1.24.0-plan9-x64"),
            RootEntry::Unrecognised
        );
        assert_eq!(
            alone(&format!("accelerator-verify-plan9-x64-{DIGEST}")),
            RootEntry::Unrecognised
        );
        assert_eq!(
            alone(".accelerator-lock-plan9-x64"),
            RootEntry::Unrecognised
        );
    }

    #[test]
    fn a_sub_binary_needs_a_name_before_its_digest() {
        assert_eq!(signed(&format!("-{DIGEST}")), RootEntry::Unrecognised);
        assert_eq!(signed(DIGEST), RootEntry::Unrecognised);
    }
}
