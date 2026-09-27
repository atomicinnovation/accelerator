//! The index each focus area holds for the rest of a `conduct` run.

use std::collections::BTreeMap;

use crate::question::NormalisedQuestion;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PinnedIndexes(BTreeMap<NormalisedQuestion, u32>);

impl PinnedIndexes {
    #[must_use]
    pub fn index_for(&self, question: &NormalisedQuestion) -> Option<u32> {
        self.0.get(question).copied()
    }

    /// Keeps an index already pinned for `question`.
    pub fn pin(&mut self, question: NormalisedQuestion, index: u32) {
        self.0.entry(question).or_insert(index);
    }

    pub fn pin_all(&mut self, other: &Self) {
        for (question, index) in other.iter() {
            self.pin(question.clone(), index);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NormalisedQuestion, u32)> {
        self.0.iter().map(|(question, index)| (question, *index))
    }

    pub fn indexes(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.values().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::PinnedIndexes;
    use crate::question::FakeUnicode;
    use crate::question::NormalisedQuestion;

    fn question(text: &str) -> NormalisedQuestion {
        NormalisedQuestion::of(text, &FakeUnicode)
    }

    #[test]
    fn a_pinned_index_is_never_moved() {
        let mut pins = PinnedIndexes::default();
        pins.pin(question("A?"), 3);
        pins.pin(question("A?"), 5);
        let mut others = PinnedIndexes::default();
        others.pin(question("A?"), 7);
        others.pin(question("B?"), 4);
        pins.pin_all(&others);
        assert_eq!(pins.index_for(&question("A?")), Some(3));
        assert_eq!(pins.index_for(&question("B?")), Some(4));
        assert_eq!(pins.index_for(&question("C?")), None);
        assert_eq!(pins.indexes().collect::<Vec<_>>(), vec![3, 4]);
    }
}
