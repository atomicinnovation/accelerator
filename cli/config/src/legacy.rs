//! The legacy-layout guard decision as a pure predicate over filesystem facts.

/// Whether the legacy `.claude/accelerator.md` layout blocks reading: the team
/// file is absent and the legacy file is present.
#[must_use]
pub const fn is_blocked(team_present: bool, legacy_present: bool) -> bool {
    !team_present && legacy_present
}

#[cfg(test)]
mod tests {
    use super::is_blocked;

    #[test]
    fn blocks_only_when_team_absent_and_legacy_present() {
        assert!(is_blocked(false, true));
        assert!(!is_blocked(true, true));
        assert!(!is_blocked(false, false));
        assert!(!is_blocked(true, false));
    }
}
