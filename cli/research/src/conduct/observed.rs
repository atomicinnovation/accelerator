//! What a set shows that a run checks against what it asked for: its
//! accepted level notes and the pairs its retained findings answer.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::topic::evidence::Digest;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;
use crate::topic::plan::RoundInputs;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    pub notes: BTreeMap<NoteRef, Digest>,
    pub answered: BTreeSet<Stem>,
}

impl Observed {
    #[must_use]
    pub fn of(inputs: &RoundInputs) -> Self {
        let notes = inputs
            .levels
            .iter()
            .flat_map(|directory| {
                directory.notes().iter().map(|note| {
                    let at = NoteRef {
                        stem: directory.stem().clone(),
                        lineage: note.lineage.clone(),
                    };
                    (at, note.digest)
                })
            })
            .collect();
        let answered = inputs
            .findings
            .iter()
            .filter(|finding| finding.answer().is_some())
            .filter_map(|finding| finding.stem().cloned())
            .collect();
        Self { notes, answered }
    }

    /// Adds what `other` shows, keeping the digest already seen for a note.
    pub fn absorb(&mut self, other: &Self) {
        for (at, digest) in &other.notes {
            self.notes.entry(at.clone()).or_insert(*digest);
        }
        self.answered.extend(other.answered.iter().cloned());
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeMap;

    use super::Observed;
    use crate::topic::evidence::Digest;
    use crate::topic::evidence::Finding;
    use crate::topic::evidence::LevelNote;
    use crate::topic::evidence::LevelsDirectory;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::note_ref::NoteRef;
    use crate::topic::layout::stem::Stem;
    use crate::topic::outline::Pair;
    use crate::topic::plan::RoundInputs;
    use crate::topic::question::FakeUnicode;

    const SEEN: Digest = Digest::new([1; 32]);
    const CHANGED: Digest = Digest::new([2; 32]);

    fn note_ref(text: &str) -> NoteRef {
        let (stem, at) = text.split_once(':').expect("a note ref");
        NoteRef {
            stem: Stem::parse(stem).expect("a stem"),
            lineage: Lineage::parse(at).expect("a lineage"),
        }
    }

    #[test]
    fn a_set_shows_its_accepted_notes_and_answered_stems() {
        let inputs = RoundInputs {
            findings: vec![
                Finding::retained(
                    "02-b-web.md",
                    Pair {
                        question: "B?".into(),
                        profile: "web".into(),
                    },
                    &FakeUnicode,
                ),
                Finding::invalid("03-c-web.md"),
            ],
            levels: vec![LevelsDirectory::new(
                Stem::parse("01-a-web").expect("a stem"),
                Some("A?".into()),
                vec![LevelNote {
                    lineage: Lineage::root(),
                    question: "A?".into(),
                    follow_ups: vec!["X?".into()],
                    digest: SEEN,
                }],
                BTreeMap::new(),
            )],
            ..RoundInputs::default()
        };
        let observed = Observed::of(&inputs);
        assert_eq!(
            observed.notes,
            BTreeMap::from([(note_ref("01-a-web:1"), SEEN)])
        );
        let answered: Vec<String> =
            observed.answered.iter().map(ToString::to_string).collect();
        assert_eq!(answered, ["02-b-web"]);
    }

    #[test]
    fn absorbing_keeps_a_seen_digest_and_adds_the_rest() {
        let mut seen = Observed {
            notes: BTreeMap::from([(note_ref("01-a:1"), SEEN)]),
            ..Observed::default()
        };
        seen.absorb(&Observed {
            notes: BTreeMap::from([
                (note_ref("01-a:1"), CHANGED),
                (note_ref("01-a:2-1"), CHANGED),
            ]),
            answered: [Stem::parse("02-b").expect("a stem")].into(),
        });
        assert_eq!(seen.notes.get(&note_ref("01-a:1")), Some(&SEEN));
        assert_eq!(seen.notes.get(&note_ref("01-a:2-1")), Some(&CHANGED));
        assert_eq!(seen.answered.len(), 1);
    }
}
