//! A node's place in its pair's research tree.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::num::NonZeroU32;

/// A level followed by one position per level below the root, such as
/// `3-2-1`.
///
/// The derived order compares the level first, then the positions left to
/// right, numerically.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lineage {
    level: u32,
    positions: Vec<u32>,
}

impl Lineage {
    #[must_use]
    pub const fn root() -> Self {
        Self {
            level: 1,
            positions: Vec::new(),
        }
    }

    /// Accepts only the canonical form, so `parse(s)?.to_string() == s`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('-').map(canonical_positive);
        let level = parts.next()??;
        let positions = parts.collect::<Option<Vec<u32>>>()?;
        let expected = usize::try_from(level - 1).ok()?;
        (positions.len() == expected).then_some(Self { level, positions })
    }

    #[must_use]
    pub const fn level(&self) -> u32 {
        self.level
    }

    #[must_use]
    pub fn child(&self, position: NonZeroU32) -> Self {
        let mut positions = self.positions.clone();
        positions.push(position.get());
        Self {
            level: self.level + 1,
            positions,
        }
    }

    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let (_, positions) = self.positions.split_last()?;
        Some(Self {
            level: self.level - 1,
            positions: positions.to_vec(),
        })
    }
}

impl Display for Lineage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.level)?;
        self.positions
            .iter()
            .try_for_each(|position| write!(formatter, "-{position}"))
    }
}

fn canonical_positive(part: &str) -> Option<u32> {
    let canonical =
        part.bytes().all(|b| b.is_ascii_digit()) && !part.starts_with('0');
    canonical.then(|| part.parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::Lineage;

    fn parsed(text: &str) -> Option<Lineage> {
        Lineage::parse(text)
    }

    fn position(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap_or(NonZeroU32::MIN)
    }

    #[test]
    fn the_root_is_level_one_with_no_positions() {
        assert_eq!(Lineage::root().level(), 1);
        assert_eq!(Lineage::root().to_string(), "1");
        assert_eq!(parsed("1"), Some(Lineage::root()));
    }

    #[test]
    fn a_lineage_carries_one_fewer_position_than_its_level() {
        let lineage = parsed("3-2-1");
        assert_eq!(lineage.as_ref().map(Lineage::level), Some(3));
        assert_eq!(lineage.map(|l| l.to_string()).as_deref(), Some("3-2-1"));
        assert_eq!(parsed("2"), None);
        assert_eq!(parsed("2-1-1"), None);
    }

    #[test]
    fn positions_are_positive_canonical_integers() {
        for text in ["2-0", "2-a", "0", "2-01", "01", "2-", "-1", "", "2--1"] {
            assert_eq!(parsed(text), None, "{text:?}");
        }
    }

    #[test]
    fn lineages_order_by_level_then_numeric_positions() {
        let ordered: Vec<Lineage> = ["2-2", "2-10", "3-1-1"]
            .iter()
            .filter_map(|text| parsed(text))
            .collect();
        assert_eq!(ordered.len(), 3);
        assert!(ordered[0] < ordered[1]);
        assert!(ordered[1] < ordered[2]);
        assert!(Lineage::root() < ordered[0]);
    }

    #[test]
    fn a_child_extends_its_parent() {
        let child = Lineage::root().child(position(2));
        assert_eq!(child.to_string(), "2-2");
        assert_eq!(child.level(), 2);
        assert_eq!(child.parent(), Some(Lineage::root()));
        let grandchild = child.child(position(1));
        assert_eq!(grandchild.to_string(), "3-2-1");
        assert_eq!(grandchild.parent(), Some(child));
        assert_eq!(Lineage::root().parent(), None);
    }
}
