//! Which of a round's spawns `conduct` launches next, and what the round
//! shows that nobody was asked to write.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use crate::lineage::Lineage;
use crate::round::Pair;
use crate::round::Round;
use crate::round::Stage;
use crate::stem::Stem;
use crate::tree::Digest;
use crate::tree::NoteRef;
use crate::tree::NoteRejection;

/// One agent `conduct` spawns: a pair's researcher or composer, or the
/// researcher of one node in a pair's tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpawnRef {
    Pair(Stem),
    Node(NoteRef),
}

impl SpawnRef {
    /// Reads `<stem>` or `<stem>:<lineage>`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.split_once(':') {
            Some((stem, lineage)) => Some(Self::Node(NoteRef {
                stem: Stem::parse(stem)?,
                lineage: Lineage::parse(lineage)?,
            })),
            None => Stem::parse(text).map(Self::Pair),
        }
    }
}

impl fmt::Display for SpawnRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pair(stem) => write!(f, "{stem}"),
            Self::Node(at) => write!(f, "{at}"),
        }
    }
}

/// What a run has spawned and seen so far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attempts {
    pub attempted: BTreeSet<SpawnRef>,
    pub just_acknowledged: BTreeSet<SpawnRef>,
    pub notes_seen: BTreeMap<NoteRef, Digest>,
    pub answered_seen: BTreeSet<Stem>,
    /// The batch offered but not yet acknowledged as spawned.
    pub pending: BTreeSet<SpawnRef>,
}

/// A spawn that ran and left its pair or node still outstanding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unaccepted {
    pub spawn: SpawnRef,
    pub rejected: Option<NoteRejection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    /// The planned round narrowed to the offered spawns.
    pub round: Round,
    pub offered: Vec<SpawnRef>,
    pub remaining: usize,
    pub unaccepted: Vec<Unaccepted>,
    pub unexpected: Vec<SpawnRef>,
}

/// Offers the first `limit` spawns not yet attempted: shallower levels
/// before deeper ones, then by stem and lineage, compositions last.
#[must_use]
pub fn window(
    round: Round,
    limit: Option<usize>,
    attempts: Option<&Attempts>,
) -> Window {
    let mut spawns: Vec<RankedSpawn> =
        round.pairs.iter().flat_map(spawns_of).collect();
    spawns.sort_by(|a, b| a.rank.cmp(&b.rank));
    let (repeated, fresh): (Vec<RankedSpawn>, Vec<RankedSpawn>) =
        spawns.into_iter().partition(|spawn| {
            attempts.is_some_and(|a| a.attempted.contains(&spawn.spawn))
        });
    let offered_count = fresh.len().min(limit.unwrap_or(usize::MAX));
    let remaining = fresh.len() - offered_count;
    let offered: Vec<SpawnRef> = fresh
        .into_iter()
        .take(offered_count)
        .map(|spawn| spawn.spawn)
        .collect();
    let unexpected = attempts
        .map(|attempts| unexpected(&round, attempts))
        .unwrap_or_default();
    Window {
        round: narrowed(round, &offered.iter().cloned().collect()),
        offered,
        remaining,
        unaccepted: repeated
            .into_iter()
            .map(|spawn| Unaccepted {
                spawn: spawn.spawn,
                rejected: spawn.rejected,
            })
            .collect(),
        unexpected,
    }
}

/// Compositions unlock nothing, so they rank after every research spawn.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    composes: bool,
    level: u32,
    stem: Stem,
    lineage: Option<Lineage>,
}

struct RankedSpawn {
    rank: Rank,
    spawn: SpawnRef,
    rejected: Option<NoteRejection>,
}

fn spawns_of(pair: &Pair) -> Vec<RankedSpawn> {
    let whole = |composes: bool| RankedSpawn {
        rank: Rank {
            composes,
            level: 1,
            stem: pair.stem.clone(),
            lineage: None,
        },
        spawn: SpawnRef::Pair(pair.stem.clone()),
        rejected: None,
    };
    match &pair.stage {
        Stage::Research => vec![whole(false)],
        Stage::Compose(_) => vec![whole(true)],
        Stage::Deepen(nodes) => nodes
            .iter()
            .map(|node| RankedSpawn {
                rank: Rank {
                    composes: false,
                    level: node.lineage.level(),
                    stem: pair.stem.clone(),
                    lineage: Some(node.lineage.clone()),
                },
                spawn: SpawnRef::Node(NoteRef {
                    stem: pair.stem.clone(),
                    lineage: node.lineage.clone(),
                }),
                rejected: node.rejected.clone(),
            })
            .collect(),
    }
}

fn narrowed(mut round: Round, offered: &BTreeSet<SpawnRef>) -> Round {
    round.pairs = round
        .pairs
        .into_iter()
        .filter_map(|mut pair| {
            if let Stage::Deepen(nodes) = &mut pair.stage {
                nodes.retain(|node| {
                    offered.contains(&SpawnRef::Node(NoteRef {
                        stem: pair.stem.clone(),
                        lineage: node.lineage.clone(),
                    }))
                });
                return (!nodes.is_empty()).then_some(pair);
            }
            offered
                .contains(&SpawnRef::Pair(pair.stem.clone()))
                .then_some(pair)
        })
        .collect();
    round
}

fn unexpected(round: &Round, attempts: &Attempts) -> Vec<SpawnRef> {
    let excused = |spawn: &SpawnRef| {
        attempts.just_acknowledged.contains(spawn)
            || attempts.pending.contains(spawn)
    };
    let notes = round.accepted_notes.iter().filter_map(|(at, digest)| {
        let spawn = SpawnRef::Node(at.clone());
        let unexplained = attempts
            .notes_seen
            .get(at)
            .map_or_else(|| !excused(&spawn), |seen| seen != digest);
        unexplained.then_some(spawn)
    });
    let findings = round
        .answered
        .iter()
        .filter(|stem| !attempts.answered_seen.contains(*stem))
        .map(|stem| SpawnRef::Pair(stem.clone()))
        .filter(|spawn| !excused(spawn));
    notes
        .chain(findings)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::BTreeSet;

    use super::window;
    use super::Attempts;
    use super::SpawnRef;
    use super::Unaccepted;
    use crate::lineage::Lineage;
    use crate::pinned_indexes::PinnedIndexes;
    use crate::round::Pair;
    use crate::round::Round;
    use crate::round::Stage;
    use crate::stem::Stem;
    use crate::tree::Digest;
    use crate::tree::Node;
    use crate::tree::NoteRef;
    use crate::tree::NoteRejection;

    fn spawn(text: &str) -> SpawnRef {
        SpawnRef::parse(text).expect("a spawn ref")
    }

    fn stem(text: &str) -> Stem {
        Stem::parse(text).expect("a stem")
    }

    fn note_ref(text: &str) -> NoteRef {
        let (stem_text, at) = text.split_once(':').expect("a note ref");
        NoteRef {
            stem: stem(stem_text),
            lineage: Lineage::parse(at).expect("a lineage"),
        }
    }

    fn pair(stem_text: &str, stage: Stage) -> Pair {
        Pair {
            question: "Q?".into(),
            profile: "web".into(),
            stem: stem(stem_text),
            path: format!("findings/{stem_text}.md"),
            stage,
        }
    }

    fn research(stem_text: &str) -> Pair {
        pair(stem_text, Stage::Research)
    }

    fn compose(stem_text: &str) -> Pair {
        pair(stem_text, Stage::Compose(vec![Lineage::root()]))
    }

    fn deepen(stem_text: &str, lineages: &[&str]) -> Pair {
        let nodes = lineages
            .iter()
            .map(|at| Node {
                lineage: Lineage::parse(at).expect("a lineage"),
                question: format!("{at}?"),
                cap: 1,
                known_questions: Vec::new(),
                rejected: None,
            })
            .collect();
        pair(stem_text, Stage::Deepen(nodes))
    }

    fn round(pairs: Vec<Pair>) -> Round {
        Round {
            items: Vec::new(),
            pairs,
            skipped: Vec::new(),
            warnings: Vec::new(),
            accepted_notes: BTreeMap::new(),
            answered: BTreeSet::new(),
            shallower: Vec::new(),
            trims: Vec::new(),
            indexes: PinnedIndexes::default(),
        }
    }

    fn spawns(texts: &[&str]) -> Vec<SpawnRef> {
        texts.iter().map(|text| spawn(text)).collect()
    }

    fn set(texts: &[&str]) -> BTreeSet<SpawnRef> {
        spawns(texts).into_iter().collect()
    }

    fn offered(round: Round, limit: Option<usize>) -> Vec<String> {
        window(round, limit, None)
            .offered
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    const SEEN: Digest = Digest::new([1; 32]);
    const CHANGED: Digest = Digest::new([2; 32]);

    fn with_note(mut round: Round, at: &str, digest: Digest) -> Round {
        round.accepted_notes.insert(note_ref(at), digest);
        round
    }

    fn unexpected(round: Round, attempts: &Attempts) -> Vec<String> {
        window(round, None, Some(attempts))
            .unexpected
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn a_spawn_ref_round_trips_for_a_pair_and_a_node() {
        for text in ["03-a-web", "03-a-web:1", "03-a-web:3-2-1"] {
            assert_eq!(
                SpawnRef::parse(text).map(|s| s.to_string()).as_deref(),
                Some(text)
            );
        }
        assert!(matches!(
            SpawnRef::parse("03-a-web"),
            Some(SpawnRef::Pair(_))
        ));
        assert!(matches!(
            SpawnRef::parse("03-a-web:2-1"),
            Some(SpawnRef::Node(_))
        ));
    }

    #[test]
    fn a_spawn_ref_is_refused_when_malformed() {
        for text in ["03-a:2-0", ":1", "03-A", "", "03-a:", "03-a:1:1"] {
            assert_eq!(SpawnRef::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn the_window_offers_deepen_nodes_by_level_before_compositions() {
        let round =
            round(vec![compose("01-a"), deepen("02-b", &["2-1", "2-2"])]);
        assert_eq!(offered(round, None), ["02-b:2-1", "02-b:2-2", "01-a"]);
    }

    #[test]
    fn the_window_orders_by_level_before_stem_across_pairs() {
        let round = round(vec![
            deepen("01-a", &["3-1-1"]),
            deepen("02-b", &["2-1"]),
            deepen("03-c", &["2-1"]),
        ]);
        assert_eq!(
            offered(round, None),
            ["02-b:2-1", "03-c:2-1", "01-a:3-1-1"]
        );
    }

    #[test]
    fn a_research_pair_ranks_as_level_one_beside_a_deepen_pair() {
        let round = round(vec![
            deepen("01-a", &["2-1"]),
            research("02-b"),
            deepen("03-c", &["1"]),
        ]);
        assert_eq!(offered(round, None), ["02-b", "03-c:1", "01-a:2-1"]);
    }

    #[test]
    fn the_window_holds_back_spawns_past_its_limit_and_counts_them() {
        let window = window(
            round(vec![research("01-a"), research("02-b"), research("03-c")]),
            Some(2),
            None,
        );
        assert_eq!(window.offered, spawns(&["01-a", "02-b"]));
        assert_eq!(window.remaining, 1);
        assert_eq!(
            window.round.pairs,
            vec![research("01-a"), research("02-b")]
        );
    }

    #[test]
    fn a_limit_splitting_a_deepen_pair_keeps_its_first_nodes() {
        let window = window(
            round(vec![deepen("01-a", &["2-1", "2-2", "2-3"])]),
            Some(2),
            None,
        );
        assert_eq!(window.round.pairs, vec![deepen("01-a", &["2-1", "2-2"])]);
        assert_eq!(window.remaining, 1);
    }

    #[test]
    fn a_deepen_pair_whose_nodes_all_fall_past_the_limit_is_not_offered() {
        let window = window(
            round(vec![deepen("02-b", &["2-1"]), research("01-a")]),
            Some(1),
            None,
        );
        assert_eq!(window.round.pairs, vec![research("01-a")]);
        assert_eq!(window.remaining, 1);
    }

    #[test]
    fn an_attempted_pair_still_pending_is_unaccepted_without_a_rejection() {
        let attempts = Attempts {
            attempted: set(&["01-a"]),
            ..Attempts::default()
        };
        let window =
            window(round(vec![research("01-a")]), None, Some(&attempts));
        assert!(window.offered.is_empty());
        assert!(window.round.pairs.is_empty());
        assert_eq!(window.remaining, 0);
        assert_eq!(
            window.unaccepted,
            vec![Unaccepted {
                spawn: spawn("01-a"),
                rejected: None
            }]
        );
    }

    #[test]
    fn an_attempted_spawn_is_never_offered_again() {
        let attempts = Attempts {
            attempted: set(&["01-a:1"]),
            ..Attempts::default()
        };
        let window = window(
            round(vec![deepen("01-a", &["1"]), research("02-b")]),
            None,
            Some(&attempts),
        );
        assert_eq!(window.offered, spawns(&["02-b"]));
        assert_eq!(window.round.pairs, vec![research("02-b")]);
    }

    #[test]
    fn an_attempted_spawn_still_pending_is_unaccepted_with_its_rejection() {
        let mut refused = deepen("01-a", &["2-1"]);
        if let Stage::Deepen(nodes) = &mut refused.stage {
            nodes[0].rejected = Some(NoteRejection::WrongKind);
        }
        let attempts = Attempts {
            attempted: set(&["01-a:2-1"]),
            ..Attempts::default()
        };
        let window = window(round(vec![refused]), None, Some(&attempts));
        assert_eq!(
            window.unaccepted,
            vec![Unaccepted {
                spawn: spawn("01-a:2-1"),
                rejected: Some(NoteRejection::WrongKind)
            }]
        );
    }

    #[test]
    fn a_window_without_attempts_or_limit_leaves_the_round_unchanged() {
        let planned = with_note(
            round(vec![
                compose("02-b"),
                deepen("01-a", &["2-1"]),
                research("03-c"),
            ]),
            "04-d:1",
            SEEN,
        );
        let window = window(planned.clone(), None, None);
        assert_eq!(window.round, planned);
        assert_eq!(window.remaining, 0);
        assert!(window.unaccepted.is_empty());
        assert!(window.unexpected.is_empty());
    }

    #[test]
    fn a_note_neither_seen_nor_attempted_is_unexpected() {
        let planned = with_note(round(Vec::new()), "01-a:2-1", SEEN);
        assert_eq!(unexpected(planned, &Attempts::default()), ["01-a:2-1"]);
    }

    #[test]
    fn a_note_seen_at_start_or_attempted_is_never_unexpected() {
        let planned = with_note(
            with_note(round(Vec::new()), "01-a:1", SEEN),
            "01-a:2-1",
            SEEN,
        );
        let attempts = Attempts {
            notes_seen: BTreeMap::from([(note_ref("01-a:1"), SEEN)]),
            just_acknowledged: set(&["01-a:2-1"]),
            attempted: set(&["01-a:2-1"]),
            ..Attempts::default()
        };
        assert!(unexpected(planned, &attempts).is_empty());
    }

    #[test]
    fn an_accepted_note_whose_digest_changes_is_unexpected() {
        let planned = with_note(round(Vec::new()), "01-a:1", CHANGED);
        let attempts = Attempts {
            notes_seen: BTreeMap::from([(note_ref("01-a:1"), SEEN)]),
            just_acknowledged: set(&["01-a:1"]),
            ..Attempts::default()
        };
        assert_eq!(unexpected(planned, &attempts), ["01-a:1"]);
    }

    #[test]
    fn a_note_refused_at_start_then_replaced_by_an_accepted_forgery_is_unexpected(
    ) {
        let started = Attempts::default();
        let planned = with_note(round(Vec::new()), "01-a:1", CHANGED);
        assert_eq!(unexpected(planned, &started), ["01-a:1"]);
    }

    #[test]
    fn a_re_researched_note_that_answered_another_question_is_not_unexpected() {
        let planned = with_note(round(Vec::new()), "01-a:2-1", CHANGED);
        let attempts = Attempts {
            attempted: set(&["01-a:2-1"]),
            just_acknowledged: set(&["01-a:2-1"]),
            ..Attempts::default()
        };
        assert!(unexpected(planned, &attempts).is_empty());
    }

    #[test]
    fn a_note_appearing_later_for_a_failed_attempt_is_unexpected() {
        let planned = with_note(round(Vec::new()), "01-a:2-1", SEEN);
        let attempts = Attempts {
            attempted: set(&["01-a:2-1"]),
            ..Attempts::default()
        };
        assert_eq!(unexpected(planned, &attempts), ["01-a:2-1"]);
    }

    #[test]
    fn writes_by_a_pending_unacknowledged_batch_are_not_unexpected() {
        let mut planned = with_note(round(Vec::new()), "01-a:2-1", SEEN);
        planned.answered.insert(stem("02-b"));
        let attempts = Attempts {
            pending: set(&["01-a:2-1", "02-b"]),
            ..Attempts::default()
        };
        assert!(unexpected(planned, &attempts).is_empty());
    }

    #[test]
    fn a_forged_finding_for_an_unattempted_pair_is_unexpected() {
        let mut planned = round(Vec::new());
        planned.answered.insert(stem("01-a"));
        planned.answered.insert(stem("02-b"));
        let attempts = Attempts {
            answered_seen: BTreeSet::from([stem("02-b")]),
            ..Attempts::default()
        };
        assert_eq!(unexpected(planned, &attempts), ["01-a"]);
    }
}
