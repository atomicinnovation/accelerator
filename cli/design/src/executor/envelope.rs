//! The launcher's own error envelopes, and the exit code each carries.
//!
//! Four asymmetries are preserved deliberately, because consumers depend on
//! them.
//!
//! Launcher-level failures go to **stderr** with a non-zero exit, while
//! daemon-side errors reach **stdout** at exit 0. The inventory skill
//! discriminates on exactly that, so collapsing it breaks the skill.
//!
//! Envelopes stay **three keys** — `error`, `message`, `category` — unlike
//! everything the daemon's own error module produces, which carries `protocol`
//! and `retryable` too.
//!
//! The `category` does **not** determine the exit code. `another-launcher-running`
//! and `daemon-start-timeout` are both `usage` yet exit 1, because both are
//! outcomes the tool evaluated and rejected rather than malformed invocations —
//! the exit-2-means-usage rule the ported subcommands follow does not hold
//! here, and the table below records that rather than leaving it inferred.
//!
//! Exit 3 keeps its meaning: the runtime still has to be bootstrapped
//! separately, so the condition it names still arises.

use std::fmt;

/// A launcher-level failure, as the caller sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherError {
    /// Invoked outside a repository, so there is no state directory to resolve.
    NoRepo,
    /// The lockhash namespace holds no installed runtime.
    PlaywrightNotInstalled { namespace_root: String },
    /// Another launcher holds the lock.
    AnotherLauncherRunning,
    /// The daemon did not publish its record within the poll window.
    DaemonStartTimeout { bootstrap_log: String },
}

impl LauncherError {
    /// The `error` key.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NoRepo => "no-repo",
            Self::PlaywrightNotInstalled { .. } => "playwright-not-installed",
            Self::AnotherLauncherRunning => "another-launcher-running",
            Self::DaemonStartTimeout { .. } => "daemon-start-timeout",
        }
    }

    /// The `category` key. Not the exit code — see the module docs.
    #[must_use]
    pub const fn category(&self) -> &'static str {
        match self {
            Self::NoRepo | Self::AnotherLauncherRunning => "usage",
            Self::PlaywrightNotInstalled { .. }
            | Self::DaemonStartTimeout { .. } => "bootstrap",
        }
    }

    /// The process exit status this envelope is reported with.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::AnotherLauncherRunning | Self::DaemonStartTimeout { .. } => 1,
            Self::NoRepo => 2,
            Self::PlaywrightNotInstalled { .. } => 3,
        }
    }

    /// The `message` key.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::NoRepo => "inventory-design must be run inside a git or jj \
                             repository (no enclosing repo found)"
                .to_owned(),
            Self::PlaywrightNotInstalled { namespace_root } => format!(
                "Playwright not installed at {namespace_root} — run \
                 ensure-playwright.sh first"
            ),
            Self::AnotherLauncherRunning => {
                "Another inventory-design launcher is running. Wait for it to \
                 finish."
                    .to_owned()
            }
            Self::DaemonStartTimeout { bootstrap_log } => format!(
                "Daemon did not start within 30s. Check {bootstrap_log} for \
                 details."
            ),
        }
    }

    /// The envelope as it reaches stderr: three keys, in a fixed order, with
    /// no trailing newline of its own.
    #[must_use]
    pub fn render(&self) -> String {
        format!(
            r#"{{"error":"{}","message":"{}","category":"{}"}}"#,
            self.code(),
            escape(&self.message()),
            self.category()
        )
    }
}

impl fmt::Display for LauncherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.render())
    }
}

/// JSON string escaping, over the only characters these messages can carry: a
/// path may hold a quote or a backslash, and nothing here is multi-line.
fn escape(raw: &str) -> String {
    raw.replace('\\', r"\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::LauncherError;

    fn every_error() -> [LauncherError; 4] {
        [
            LauncherError::NoRepo,
            LauncherError::PlaywrightNotInstalled {
                namespace_root: "/ns/root".to_owned(),
            },
            LauncherError::AnotherLauncherRunning,
            LauncherError::DaemonStartTimeout {
                bootstrap_log: "/state/server.bootstrap.log".to_owned(),
            },
        ]
    }

    /// The retired shell's own envelopes, byte for byte.
    #[test]
    fn each_envelope_reproduces_the_shell_s_json() {
        assert_eq!(
            LauncherError::NoRepo.render(),
            r#"{"error":"no-repo","message":"inventory-design must be run inside a git or jj repository (no enclosing repo found)","category":"usage"}"#
        );
        assert_eq!(
            LauncherError::AnotherLauncherRunning.render(),
            r#"{"error":"another-launcher-running","message":"Another inventory-design launcher is running. Wait for it to finish.","category":"usage"}"#
        );
        assert_eq!(
            LauncherError::PlaywrightNotInstalled {
                namespace_root: "/ns/root".to_owned()
            }
            .render(),
            r#"{"error":"playwright-not-installed","message":"Playwright not installed at /ns/root — run ensure-playwright.sh first","category":"bootstrap"}"#
        );
        assert_eq!(
            LauncherError::DaemonStartTimeout {
                bootstrap_log: "/state/server.bootstrap.log".to_owned()
            }
            .render(),
            r#"{"error":"daemon-start-timeout","message":"Daemon did not start within 30s. Check /state/server.bootstrap.log for details.","category":"bootstrap"}"#
        );
    }

    /// No `protocol`, no `retryable` — unlike everything the daemon produces.
    #[test]
    fn every_envelope_carries_exactly_three_keys() {
        for error in every_error() {
            let rendered = error.render();
            assert_eq!(rendered.matches("\":").count(), 3, "{rendered}");
            for absent in ["protocol", "retryable"] {
                assert!(!rendered.contains(absent), "{rendered}");
            }
            assert!(rendered.starts_with(r#"{"error":"#));
            assert!(rendered.ends_with('}'));
            assert!(!rendered.contains('\n'));
        }
    }

    /// The table the module docs describe, asserted rather than inferred —
    /// two `usage` envelopes exit 1, not 2.
    #[test]
    fn the_category_does_not_determine_the_exit_code() {
        assert_eq!(LauncherError::NoRepo.category(), "usage");
        assert_eq!(LauncherError::NoRepo.exit_code(), 2);
        assert_eq!(LauncherError::AnotherLauncherRunning.category(), "usage");
        assert_eq!(LauncherError::AnotherLauncherRunning.exit_code(), 1);

        let timeout = LauncherError::DaemonStartTimeout {
            bootstrap_log: "/log".to_owned(),
        };
        assert_eq!(timeout.category(), "bootstrap");
        assert_eq!(timeout.exit_code(), 1);

        let missing = LauncherError::PlaywrightNotInstalled {
            namespace_root: "/ns".to_owned(),
        };
        assert_eq!(missing.category(), "bootstrap");
        assert_eq!(missing.exit_code(), 3);
    }

    #[test]
    fn a_path_carrying_json_metacharacters_is_escaped() {
        let rendered = LauncherError::PlaywrightNotInstalled {
            namespace_root: r#"/ns/"quoted"\path"#.to_owned(),
        }
        .render();
        assert!(rendered.contains(r#"/ns/\"quoted\"\\path"#), "{rendered}");
    }

    #[test]
    fn every_code_is_distinct_and_kebab_cased() {
        let mut codes: Vec<&str> =
            every_error().iter().map(LauncherError::code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 4);
        assert!(codes.iter().all(|code| code
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-')));
    }
}
