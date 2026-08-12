//! What a subcommand hands back to `main`.
//!
//! `corpus-cli`'s `Outcome` cannot express this: it is two `String` fields
//! whose every `Ok` maps to a success exit. A design subcommand needs a
//! successful-outcome path that exits non-zero, so the accept/reject
//! distinction is carried in the type and `main` matches on it — one
//! render-and-exit function rather than a bespoke mapping per subcommand.

/// A rendered subcommand outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// The input was evaluated and accepted. Exit 0.
    Accepted { stdout: String, stderr: String },
    /// The input was evaluated and rejected. Exit 1.
    Rejected { stderr: String },
}

impl Report {
    /// An acceptance printing nothing.
    #[must_use]
    pub const fn silent() -> Self {
        Self::Accepted {
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    /// An acceptance printing `line` and a newline to stdout.
    #[must_use]
    pub fn line(line: &str) -> Self {
        Self::Accepted {
            stdout: format!("{line}\n"),
            stderr: String::new(),
        }
    }

    /// A rejection printing `reason` to stderr, prefixed the way every
    /// migrated script prefixed its own.
    #[must_use]
    pub fn rejected(reason: &str) -> Self {
        Self::Rejected {
            stderr: format!("error: {reason}\n"),
        }
    }

    /// The same acceptance, with `warning` added to stderr.
    #[must_use]
    pub fn warning(self, warning: &str) -> Self {
        match self {
            Self::Accepted { stdout, stderr } => Self::Accepted {
                stdout,
                stderr: format!("{stderr}warning: {warning}\n"),
            },
            Self::Rejected { stderr } => Self::Rejected { stderr },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Report;

    #[test]
    fn a_line_acceptance_carries_a_trailing_newline() {
        assert_eq!(
            Report::line("header"),
            Report::Accepted {
                stdout: "header\n".to_owned(),
                stderr: String::new(),
            }
        );
    }

    #[test]
    fn a_rejection_carries_the_error_prefix_every_script_used() {
        assert_eq!(
            Report::rejected("no"),
            Report::Rejected {
                stderr: "error: no\n".to_owned()
            }
        );
    }

    #[test]
    fn a_warning_joins_an_acceptance_without_displacing_its_stdout() {
        assert_eq!(
            Report::line("header").warning("careful"),
            Report::Accepted {
                stdout: "header\n".to_owned(),
                stderr: "warning: careful\n".to_owned(),
            }
        );
    }

    #[test]
    fn a_warning_on_a_rejection_changes_nothing() {
        let rejected = Report::rejected("no");
        assert_eq!(rejected.clone().warning("careful"), rejected);
    }
}
