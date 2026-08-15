//! The carrier every design subcommand reports through.
//!
//! A domain rejection is a *verdict*, not an error. `corpus-cli`'s `Outcome`
//! cannot express one: its `main` maps every `Ok` to a success exit, and no
//! sub-binary in the workspace has a successful-outcome path that exits
//! non-zero. Modelling a rejection as `kernel::Error` instead would make the
//! most caller-actionable outcome this binary has share exit 1 with genuine
//! internal failures, leaving a caller unable to tell "the tool worked and
//! refused your input" from "the tool broke".
//!
//! `kernel::Error::Refusal` therefore keeps its documented meaning here and
//! carries usage errors to exit 2.

/// The outcome of evaluating an input: accepted, or evaluated and rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict<Reason> {
    Accepted,
    Rejected(Reason),
}

impl<Reason> Verdict<Reason> {
    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Rewrites the rejection reason, leaving an acceptance untouched.
    #[must_use]
    pub fn map_reason<Other>(
        self,
        rewrite: impl FnOnce(Reason) -> Other,
    ) -> Verdict<Other> {
        match self {
            Self::Accepted => Verdict::Accepted,
            Self::Rejected(reason) => Verdict::Rejected(rewrite(reason)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Verdict;

    #[test]
    fn acceptance_and_rejection_are_distinguishable() {
        let accepted: Verdict<String> = Verdict::Accepted;
        let rejected = Verdict::Rejected("no".to_owned());
        assert!(accepted.is_accepted());
        assert!(!rejected.is_accepted());
    }

    #[test]
    fn mapping_rewrites_only_the_rejection() {
        assert_eq!(
            Verdict::Rejected(1).map_reason(|reason| reason + 1),
            Verdict::Rejected(2)
        );
        assert_eq!(
            Verdict::<i32>::Accepted.map_reason(|reason| reason + 1),
            Verdict::Accepted
        );
    }
}
