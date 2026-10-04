//! The finding indexes focus areas claim: from a file on disk that holds one
//! for its question, and for the rest of a `conduct` run once planned.

use std::collections::BTreeMap;

use crate::topic::layout::stem::Stem;
use crate::topic::question::NormalisedQuestion;
use crate::topic::question::UnicodeText;

/// The index each focus area keeps for the rest of a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaimedIndexes(BTreeMap<NormalisedQuestion, u32>);

impl ClaimedIndexes {
    #[must_use]
    pub fn index_for(&self, question: &NormalisedQuestion) -> Option<u32> {
        self.0.get(question).copied()
    }

    /// Keeps an index already claimed for `question`.
    pub fn claim(&mut self, question: NormalisedQuestion, index: u32) {
        self.0.entry(question).or_insert(index);
    }

    pub fn claim_all(&mut self, other: &Self) {
        for (question, index) in other.iter() {
            self.claim(question.clone(), index);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NormalisedQuestion, u32)> {
        self.0.iter().map(|(question, index)| (question, *index))
    }

    pub fn indexes(&self) -> impl Iterator<Item = u32> + '_ {
        self.0.values().copied()
    }
}

/// A quarantined finding or a pair's `.levels/` directory: a file that holds
/// an index, for the question it names when it names one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexClaim {
    index: u32,
    question: Option<NormalisedQuestion>,
}

impl IndexClaim {
    #[must_use]
    pub fn new(
        index: u32,
        question: Option<&str>,
        unicode: &dyn UnicodeText,
    ) -> Self {
        Self {
            index,
            question: question
                .map(|question| NormalisedQuestion::of(question, unicode)),
        }
    }

    /// The claim a quarantined `findings/` file named `name` holds, when its
    /// name leads with an index.
    #[must_use]
    pub fn quarantined(
        name: &str,
        question: Option<&str>,
        unicode: &dyn UnicodeText,
    ) -> Option<Self> {
        Some(Self::new(Stem::index_of(name)?, question, unicode))
    }

    #[must_use]
    pub const fn index(&self) -> u32 {
        self.index
    }

    #[must_use]
    pub fn is_for(&self, question: &NormalisedQuestion) -> bool {
        self.question.as_ref() == Some(question)
    }
}

#[cfg(test)]
mod tests {
    use super::ClaimedIndexes;
    use super::IndexClaim;
    use crate::topic::question::FakeUnicode;
    use crate::topic::question::NormalisedQuestion;

    fn question(text: &str) -> NormalisedQuestion {
        NormalisedQuestion::of(text, &FakeUnicode)
    }

    #[test]
    fn a_claimed_index_is_never_moved() {
        let mut claims = ClaimedIndexes::default();
        claims.claim(question("A?"), 3);
        claims.claim(question("A?"), 5);
        let mut others = ClaimedIndexes::default();
        others.claim(question("A?"), 7);
        others.claim(question("B?"), 4);
        claims.claim_all(&others);
        assert_eq!(claims.index_for(&question("A?")), Some(3));
        assert_eq!(claims.index_for(&question("B?")), Some(4));
        assert_eq!(claims.index_for(&question("C?")), None);
        assert_eq!(claims.indexes().collect::<Vec<_>>(), vec![3, 4]);
    }

    #[test]
    fn a_quarantined_file_claims_its_leading_index_for_its_question() {
        let claim = IndexClaim::quarantined(
            ".04-c-web.md.invalid",
            Some(" C? "),
            &FakeUnicode,
        );
        assert_eq!(claim.as_ref().map(IndexClaim::index), Some(4));
        assert!(claim.is_some_and(|claim| claim.is_for(&question("C?"))));
    }

    #[test]
    fn a_quarantined_file_without_a_leading_index_claims_nothing() {
        assert_eq!(
            IndexClaim::quarantined(".x.md.invalid", None, &FakeUnicode),
            None
        );
    }

    #[test]
    fn a_claim_naming_no_question_is_for_none() {
        let claim = IndexClaim::new(2, None, &FakeUnicode);
        assert!(!claim.is_for(&question("A?")));
    }
}
