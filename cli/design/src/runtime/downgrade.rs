//! Why a Playwright-driven inventory fell back to the code-only crawler.
//!
//! Today's six-reason vocabulary, ported unchanged. `ensure-playwright.sh`
//! still emits five of these and passes them verbatim to `--reason`, so a new
//! vocabulary would make every real downgrade a usage error — failing the
//! graceful-degradation path on exactly the machines that need it.
//!
//! The message table lives here as a `match` rather than being loaded from
//! disk, so exhaustiveness is a compile error rather than a runtime lookup
//! miss. A drift test pins it against the on-disk fixture the shell read.

use std::fmt;
use std::str::FromStr;

/// The reasons a downgrade notice can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DowngradeReason {
    NodeMissing,
    NodeTooOld,
    BootstrapFailed,
    ExecutorPingFailed,
    CacheUnwritable,
    DiskFloorNotMet,
}

impl DowngradeReason {
    /// Every reason, so a caller can enumerate the vocabulary without
    /// restating it.
    pub const ALL: [Self; 6] = [
        Self::NodeMissing,
        Self::NodeTooOld,
        Self::BootstrapFailed,
        Self::ExecutorPingFailed,
        Self::CacheUnwritable,
        Self::DiskFloorNotMet,
    ];

    /// The key the shell passes to `--reason`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NodeMissing => "node-missing",
            Self::NodeTooOld => "node-too-old",
            Self::BootstrapFailed => "bootstrap-failed",
            Self::ExecutorPingFailed => "executor-ping-failed",
            Self::CacheUnwritable => "cache-unwritable",
            Self::DiskFloorNotMet => "disk-floor-not-met",
        }
    }

    /// The notice printed for this reason.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::NodeMissing => "inventory-design: Playwright runtime is unavailable (Node >= 20 not found). Falling back to code-only crawler. Run `ensure-playwright.sh` manually to install, or pass --crawler code to suppress this notice.",
            Self::NodeTooOld => "inventory-design: Playwright runtime is unavailable (Node version is too old; >= 20 required). Falling back to code-only crawler. Run `ensure-playwright.sh` manually after upgrading Node, or pass --crawler code to suppress this notice.",
            Self::BootstrapFailed => "inventory-design: Playwright bootstrap failed. Falling back to code-only crawler. Run `ensure-playwright.sh` manually to see the full error, or pass --crawler code to suppress this notice.",
            Self::ExecutorPingFailed => "inventory-design: Playwright executor is unhealthy. Falling back to code-only crawler. Run `run.sh ping` manually to diagnose, or pass --crawler code to suppress this notice.",
            Self::CacheUnwritable => "inventory-design: Playwright cache directory is not writable. Falling back to code-only crawler. Check permissions on $ACCELERATOR_PLAYWRIGHT_CACHE (or ~/.cache/accelerator/playwright) and retry, or pass --crawler code to suppress this notice.",
            Self::DiskFloorNotMet => "inventory-design: Playwright cache filesystem has less than 500 MB free. Falling back to code-only crawler. Free space at $ACCELERATOR_PLAYWRIGHT_CACHE and retry, or pass --crawler code to suppress this notice.",
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

    /// The shell filtered these out of text it read from a file at runtime.
    /// The table is now compiled in, so the branch is unreachable by
    /// construction — the invariant is asserted over the shipped data instead.
    #[test]
    fn every_message_is_printable_ascii_free_of_bidi_overrides() {
        for reason in DowngradeReason::ALL {
            for character in reason.message().chars() {
                assert!(
                    matches!(character, ' '..='~'),
                    "{reason}'s message carries {character:?}, which the \
                     shell's runtime filter would have stripped"
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
