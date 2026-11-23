//! Whether a jj repository shares its working copy with a git repository.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JjColocation {
    Pure,
    Colocated,
}

impl JjColocation {
    pub const ALL: [Self; 2] = [Self::Pure, Self::Colocated];

    #[must_use]
    pub const fn flag(self) -> &'static str {
        match self {
            Self::Pure => "--no-colocate",
            Self::Colocated => "--colocate",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JjColocation;

    #[test]
    fn a_pure_repository_is_initialised_without_colocating() {
        assert_eq!(JjColocation::Pure.flag(), "--no-colocate");
    }

    #[test]
    fn a_colocated_repository_is_initialised_colocating() {
        assert_eq!(JjColocation::Colocated.flag(), "--colocate");
    }
}
