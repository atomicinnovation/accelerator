//! A pair's `<nn>-<slug>-<profile>` stem, which names its finding and its
//! tree of level notes.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

/// ASCII digits, a `-`, then a non-empty rest drawn from `[a-z0-9-]`.
///
/// The alphabet is what the planner allocates from. It keeps a stem safe to
/// interpolate into warnings and to join to a lineage with `:`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stem {
    text: String,
    index: u32,
}

impl Stem {
    /// Keeps the text as written, so `parse(s)?.to_string() == s`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (digits, rest) = text.split_once('-')?;
        let well_formed = !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
            && is_allocatable(rest);
        let index = well_formed.then(|| digits.parse().ok()).flatten()?;
        Some(Self {
            text: text.to_owned(),
            index,
        })
    }

    /// Whether a stem can end in `profile`, so the planner can allocate its
    /// pairs.
    #[must_use]
    pub fn admits_profile(profile: &str) -> bool {
        is_allocatable(profile)
    }

    #[must_use]
    pub fn allocated(index: u32, slug: &str, profile: &str) -> Option<Self> {
        Self::admits_profile(profile)
            .then(|| Self::parse(&format!("{index:02}-{slug}-{profile}")))
            .flatten()
    }

    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }
}

impl Display for Stem {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

fn is_allocatable(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::Stem;

    #[test]
    fn a_stem_carries_its_index_and_an_allocated_rest() {
        let stem = Stem::parse("03-a-web");
        assert_eq!(stem.as_ref().map(Stem::index), Some(3));
        let allocated = Stem::allocated(3, "a", "web");
        assert_eq!(allocated, stem);
    }

    #[test]
    fn a_stem_outside_the_alphabet_is_refused() {
        for text in
            ["03-A", "03-a b", "03-a:1", "03-a\nb", "a-web", "03-", "-a"]
        {
            assert_eq!(Stem::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_stem_round_trips_through_display() {
        for text in ["03-a-web", "100-a-web", "3-a", "003-a"] {
            assert_eq!(
                Stem::parse(text).map(|stem| stem.to_string()).as_deref(),
                Some(text)
            );
        }
    }

    #[test]
    fn stems_with_equal_index_but_different_digits_are_distinct() {
        let short = Stem::parse("3-a");
        let padded = Stem::parse("03-a");
        assert_eq!(short.as_ref().map(Stem::index), Some(3));
        assert_eq!(padded.as_ref().map(Stem::index), Some(3));
        assert_ne!(short, padded);
    }

    #[test]
    fn a_stem_whose_index_overflows_u32_is_refused() {
        assert_eq!(Stem::parse("4294967296-a"), None);
        assert_eq!(
            Stem::parse("4294967295-a").as_ref().map(Stem::index),
            Some(u32::MAX)
        );
    }

    #[test]
    fn a_profile_outside_the_alphabet_allocates_no_stem() {
        for profile in ["Web", "we b", "web:1", ""] {
            assert!(!Stem::admits_profile(profile), "{profile:?}");
            assert_eq!(Stem::allocated(3, "a", profile), None, "{profile:?}");
        }
        assert!(Stem::admits_profile("open-alex2"));
    }
}
