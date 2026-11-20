//! The statuses a set moves through, from `brief` to `finalise`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetStatus {
    Briefed,
    Outlined,
    Researching,
    Synthesised,
    Complete,
}

impl SetStatus {
    pub const ALL: [Self; 5] = [
        Self::Briefed,
        Self::Outlined,
        Self::Researching,
        Self::Synthesised,
        Self::Complete,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Briefed => "briefed",
            Self::Outlined => "outlined",
            Self::Researching => "researching",
            Self::Synthesised => "synthesised",
            Self::Complete => "complete",
        }
    }

    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|status| status.name() == name)
    }

    #[must_use]
    pub const fn is_consultable(self) -> bool {
        matches!(self, Self::Synthesised | Self::Complete)
    }
}

#[cfg(test)]
mod tests {
    use super::SetStatus;

    #[test]
    fn only_a_synthesised_or_complete_set_is_consultable() {
        let consultable: Vec<&str> = SetStatus::ALL
            .into_iter()
            .filter(|status| status.is_consultable())
            .map(SetStatus::name)
            .collect();
        assert_eq!(consultable, ["synthesised", "complete"]);
    }

    #[test]
    fn a_status_is_named_as_the_manifest_records_it() {
        for status in SetStatus::ALL {
            assert_eq!(SetStatus::named(status.name()), Some(status));
        }
        assert_eq!(SetStatus::named("drafted"), None);
    }
}
