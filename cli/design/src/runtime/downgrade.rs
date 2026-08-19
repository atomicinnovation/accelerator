//! Why a Playwright-driven inventory fell back to the code-only crawler.
//!
//! The reasons are emitted by the executor's availability check — the platform
//! probe, the runtime materialisation, and the bootstrap-log classification —
//! and rendered to the user unfiltered. The message table lives here as a
//! `match` rather than being loaded from disk, so exhaustiveness is a compile
//! error rather than a runtime lookup miss, and a golden per reason pins the
//! exact text.

use std::fmt;
use std::str::FromStr;

/// The reasons a downgrade notice can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DowngradeReason {
    UnsupportedPlatform,
    LoaderUnresolvable,
    GlibcTooOld,
    RuntimeLibrariesMissing,
    ArtifactUnavailable,
    MaterialisationInProgress,
    ExecutorPingFailed,
    CacheUnwritable,
    DiskFloorNotMet,
}

impl DowngradeReason {
    /// Every reason, so a caller can enumerate the vocabulary without
    /// restating it.
    pub const ALL: [Self; 9] = [
        Self::UnsupportedPlatform,
        Self::LoaderUnresolvable,
        Self::GlibcTooOld,
        Self::RuntimeLibrariesMissing,
        Self::ArtifactUnavailable,
        Self::MaterialisationInProgress,
        Self::ExecutorPingFailed,
        Self::CacheUnwritable,
        Self::DiskFloorNotMet,
    ];

    /// The key a caller passes to `--reason`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "unsupported-platform",
            Self::LoaderUnresolvable => "loader-unresolvable",
            Self::GlibcTooOld => "glibc-too-old",
            Self::RuntimeLibrariesMissing => "runtime-libraries-missing",
            Self::ArtifactUnavailable => "artifact-unavailable",
            Self::MaterialisationInProgress => "materialisation-in-progress",
            Self::ExecutorPingFailed => "executor-ping-failed",
            Self::CacheUnwritable => "cache-unwritable",
            Self::DiskFloorNotMet => "disk-floor-not-met",
        }
    }

    /// The notice printed for this reason.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::UnsupportedPlatform => "inventory-design: Playwright runtime is unavailable (this platform's libc is not supported). Falling back to code-only crawler. Pass --crawler code to suppress this notice.",
            Self::LoaderUnresolvable => "inventory-design: Playwright runtime is unavailable (the dynamic loader it needs is missing or relocated). Falling back to code-only crawler. Install nix-ld or set design.browser_path to a working browser, or pass --crawler code to suppress this notice.",
            Self::GlibcTooOld => "inventory-design: Playwright runtime is unavailable (the system glibc is too old for the bundled browser). Falling back to code-only crawler. Upgrade the distribution, or pass --crawler code to suppress this notice.",
            Self::RuntimeLibrariesMissing => "inventory-design: Playwright runtime is unavailable (a shared library the browser needs is missing). Falling back to code-only crawler. Install the package providing the named library, or pass --crawler code to suppress this notice.",
            Self::ArtifactUnavailable => "inventory-design: Playwright runtime is unavailable (the runtime artifacts could not be materialised). Falling back to code-only crawler. Run `accelerator cache repair` to re-materialise, or pass --crawler code to suppress this notice.",
            Self::MaterialisationInProgress => "inventory-design: Playwright runtime is being materialised by another process. Falling back to code-only crawler for this invocation; it retries on the next. Pass --crawler code to suppress this notice.",
            Self::ExecutorPingFailed => "inventory-design: Playwright executor is unhealthy. Falling back to code-only crawler. Run `accelerator design executor ping` manually to diagnose, or pass --crawler code to suppress this notice.",
            Self::CacheUnwritable => "inventory-design: Playwright cache directory is not writable. Falling back to code-only crawler. Check permissions on the cache directory (ACCELERATOR_CACHE_DIR, or the default under the plugin) and retry, or pass --crawler code to suppress this notice.",
            Self::DiskFloorNotMet => "inventory-design: Playwright cache filesystem has insufficient free space for the runtime artifacts. Falling back to code-only crawler. Free space and retry, or pass --crawler code to suppress this notice.",
        }
    }
}

impl fmt::Display for DowngradeReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key())
    }
}

/// The `--reason` value named no known key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownReason(pub String);

impl std::error::Error for UnknownReason {}

impl fmt::Display for UnknownReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let valid: Vec<&str> =
            DowngradeReason::ALL.iter().map(|r| r.key()).collect();
        write!(
            formatter,
            "unknown --reason '{}'\n       Valid values: {}",
            self.0,
            valid.join(", ")
        )
    }
}

impl FromStr for DowngradeReason {
    type Err = UnknownReason;

    fn from_str(key: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|reason| reason.key() == key)
            .ok_or_else(|| UnknownReason(key.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::DowngradeReason;

    #[test]
    fn every_reason_round_trips_through_its_key() {
        for reason in DowngradeReason::ALL {
            assert_eq!(
                DowngradeReason::from_str(reason.key()),
                Ok(reason),
                "{reason}"
            );
        }
    }

    #[test]
    fn an_unknown_reason_lists_the_whole_vocabulary() -> Result<(), String> {
        let Err(error) = DowngradeReason::from_str("nonsense") else {
            return Err("expected a refusal".to_owned());
        };
        let text = error.to_string();
        assert!(text.contains("unknown --reason 'nonsense'"));
        for reason in DowngradeReason::ALL {
            assert!(text.contains(reason.key()), "{reason} must be listed");
        }
        Ok(())
    }

    /// The messages reach a terminal unfiltered, so the invariant is asserted
    /// over the shipped data rather than enforced at print time.
    #[test]
    fn every_message_is_printable_ascii_free_of_bidi_overrides() {
        for reason in DowngradeReason::ALL {
            for character in reason.message().chars() {
                assert!(
                    matches!(character, ' '..='~'),
                    "{reason}'s message carries {character:?}, which is not \
                     printable ASCII"
                );
            }
        }
    }

    #[test]
    fn every_reason_carries_a_distinct_non_empty_message() {
        let mut messages: Vec<&str> =
            DowngradeReason::ALL.iter().map(|r| r.message()).collect();
        messages.sort_unstable();
        messages.dedup();
        assert_eq!(messages.len(), DowngradeReason::ALL.len());
        assert!(messages.iter().all(|message| !message.is_empty()));
    }

    #[test]
    fn every_message_names_the_suppression_flag() {
        for reason in DowngradeReason::ALL {
            assert!(
                reason.message().contains("--crawler code"),
                "{reason} must tell the caller how to suppress the notice"
            );
        }
    }
}
