//! The order in which the runtime crawler's preconditions are checked.
//!
//! Platform support first — decided at zero network cost, so an unsupported
//! host refuses before any fetch — then the runtime (the driver bundle), then
//! the browser. The browser hatch (`design.browser_path`) substitutes the
//! browser and never the runtime, so the driver is ensured even when the hatch
//! is set; the hatch's "no browser fetch" behaviour lives in the browser
//! resolver the caller injects, not here.
//!
//! Pure over injected outcomes: the platform verdict, and two thunks the caller
//! evaluates lazily, so an unsupported platform never triggers the runtime
//! ensure or the browser resolution. The caller decides what a `Downgrade`
//! means — code-only for the default and hybrid crawlers, a hard failure for an
//! explicit `--crawler runtime`.

use std::path::PathBuf;

use crate::runtime::downgrade::DowngradeReason;
use crate::runtime::platform::Support;

/// A resolved runtime: the driver tree and the browser executable to launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runtime {
    pub driver: PathBuf,
    pub browser_executable: PathBuf,
}

/// The outcome of ensuring the driver bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeOutcome {
    Ready(PathBuf),
    Downgrade(DowngradeReason),
}

/// The outcome of resolving the browser executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserOutcome {
    /// The bundled headless shell.
    Bundled(PathBuf),
    /// An explicitly configured `design.browser_path`.
    Hatch(PathBuf),
    Downgrade(DowngradeReason),
}

/// Whether the runtime crawler can run, or why it cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Ready(Runtime),
    Downgrade(DowngradeReason),
}

/// Resolve the runtime crawler's preconditions in order: the platform, then
/// the runtime, then the browser.
pub fn resolve(
    platform: Support,
    ensure_runtime: impl FnOnce() -> RuntimeOutcome,
    resolve_browser: impl FnOnce() -> BrowserOutcome,
) -> Resolution {
    if let Support::Unsupported(reason) = platform {
        return Resolution::Downgrade(reason);
    }
    let driver = match ensure_runtime() {
        RuntimeOutcome::Ready(driver) => driver,
        RuntimeOutcome::Downgrade(reason) => {
            return Resolution::Downgrade(reason);
        }
    };
    let browser_executable = match resolve_browser() {
        BrowserOutcome::Bundled(path) | BrowserOutcome::Hatch(path) => path,
        BrowserOutcome::Downgrade(reason) => {
            return Resolution::Downgrade(reason);
        }
    };
    Resolution::Ready(Runtime {
        driver,
        browser_executable,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::PathBuf;

    use super::resolve;
    use super::BrowserOutcome;
    use super::Resolution;
    use super::RuntimeOutcome;
    use crate::runtime::downgrade::DowngradeReason;
    use crate::runtime::platform::Support;

    #[test]
    fn a_musl_host_downgrades_without_resolving_a_browser_path() {
        let runtime_ran = Cell::new(false);
        let browser_ran = Cell::new(false);
        let resolution = resolve(
            Support::Unsupported(DowngradeReason::UnsupportedPlatform),
            || {
                runtime_ran.set(true);
                RuntimeOutcome::Downgrade(DowngradeReason::ArtifactUnavailable)
            },
            || {
                browser_ran.set(true);
                BrowserOutcome::Downgrade(DowngradeReason::ArtifactUnavailable)
            },
        );
        assert_eq!(
            resolution,
            Resolution::Downgrade(DowngradeReason::UnsupportedPlatform)
        );
        assert!(!runtime_ran.get(), "the runtime ensure must not run");
        assert!(!browser_ran.get(), "the browser must not be resolved");
    }

    #[test]
    fn a_runtime_failure_downgrades_without_resolving_a_browser() {
        let browser_ran = Cell::new(false);
        let resolution = resolve(
            Support::Supported,
            || RuntimeOutcome::Downgrade(DowngradeReason::ArtifactUnavailable),
            || {
                browser_ran.set(true);
                BrowserOutcome::Downgrade(DowngradeReason::ArtifactUnavailable)
            },
        );
        assert_eq!(
            resolution,
            Resolution::Downgrade(DowngradeReason::ArtifactUnavailable)
        );
        assert!(!browser_ran.get(), "the browser must not be resolved");
    }

    #[test]
    fn the_bundled_browser_is_used_when_the_hatch_is_unset() {
        let shell = PathBuf::from("/driver/../browser/shell");
        let resolution = resolve(
            Support::Supported,
            || RuntimeOutcome::Ready(PathBuf::from("/driver")),
            || BrowserOutcome::Bundled(shell.clone()),
        );
        assert_eq!(
            resolution,
            Resolution::Ready(super::Runtime {
                driver: PathBuf::from("/driver"),
                browser_executable: PathBuf::from("/driver/../browser/shell"),
            })
        );
    }

    #[test]
    fn the_hatch_browser_is_used_while_the_driver_is_still_ensured() {
        let driver_ensured = Cell::new(false);
        let resolution = resolve(
            Support::Supported,
            || {
                driver_ensured.set(true);
                RuntimeOutcome::Ready(PathBuf::from("/driver"))
            },
            || BrowserOutcome::Hatch(PathBuf::from("/usr/bin/chromium")),
        );
        assert!(driver_ensured.get(), "the driver ensure must run");
        assert_eq!(
            resolution,
            Resolution::Ready(super::Runtime {
                driver: PathBuf::from("/driver"),
                browser_executable: PathBuf::from("/usr/bin/chromium"),
            })
        );
    }

    #[test]
    fn a_browser_failure_downgrades() {
        let resolution = resolve(
            Support::Supported,
            || RuntimeOutcome::Ready(PathBuf::from("/driver")),
            || BrowserOutcome::Downgrade(DowngradeReason::ArtifactUnavailable),
        );
        assert_eq!(
            resolution,
            Resolution::Downgrade(DowngradeReason::ArtifactUnavailable)
        );
    }
}
