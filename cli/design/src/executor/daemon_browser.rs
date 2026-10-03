//! The browser a daemon runs, which decides the state slot it is found in.

use std::path::PathBuf;

use crate::runtime::browser_path::HatchDecision;

/// The browser a daemon was spawned with.
///
/// Each browser has its own daemon, so an executor never reuses a daemon
/// running a browser other than the one the consent policy admitted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DaemonBrowser {
    Bundled,
    /// The canonical executable the hatch admitted.
    Custom(PathBuf),
}

impl DaemonBrowser {
    #[must_use]
    pub fn chosen_by(hatch: &HatchDecision) -> Self {
        hatch.browser.clone().map_or(Self::Bundled, Self::Custom)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::DaemonBrowser;
    use crate::runtime::browser_path::HatchDecision;

    fn hatch(browser: Option<PathBuf>) -> HatchDecision {
        HatchDecision {
            browser,
            warnings: Vec::new(),
            notice: None,
        }
    }

    #[test]
    fn a_hatch_without_a_browser_chooses_the_bundled_one() {
        assert_eq!(
            DaemonBrowser::chosen_by(&hatch(None)),
            DaemonBrowser::Bundled
        );
    }

    #[test]
    fn a_hatch_with_a_browser_chooses_that_browser() {
        let chrome = PathBuf::from("/opt/chrome/chrome");
        assert_eq!(
            DaemonBrowser::chosen_by(&hatch(Some(chrome.clone()))),
            DaemonBrowser::Custom(chrome)
        );
    }
}
