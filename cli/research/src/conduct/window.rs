//! Which of a round's spawns `conduct` launches next, and what the set shows
//! that nobody was asked to write.

use std::collections::BTreeSet;

use crate::conduct::memory::RunMemory;
use crate::conduct::observed::Observed;
use crate::conduct::spawn::SpawnRef;
use crate::topic::evidence::NoteRejection;
use crate::topic::layout::lineage::Lineage;
use crate::topic::layout::note_ref::NoteRef;
use crate::topic::layout::stem::Stem;
use crate::topic::plan::OutstandingPair;
use crate::topic::plan::RoundPlan;
use crate::topic::tree::Stage;

/// A spawn attempted earlier in the run whose pair or node is still
/// outstanding, which the run does not offer again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnfinishedSpawn {
    pub spawn: SpawnRef,
    pub rejected: Option<NoteRejection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    /// The plan's outstanding pairs narrowed to the offered spawns.
    pub pairs: Vec<OutstandingPair>,
    pub offered: Vec<SpawnRef>,
    pub remaining: usize,
    pub unfinished: Vec<UnfinishedSpawn>,
    pub unexpected: Vec<SpawnRef>,
}

/// Offers the first `limit` spawns not yet attempted: shallower levels
/// before deeper ones, then by stem and lineage, compositions last.
#[must_use]
pub fn window(
    plan: &RoundPlan,
    observed: &Observed,
    limit: Option<usize>,
    memory: Option<&RunMemory>,
) -> Window {
    let mut spawns: Vec<RankedSpawn> =
        plan.pairs.iter().flat_map(spawns_of).collect();
    spawns.sort_by(|a, b| a.rank.cmp(&b.rank));
    let (repeated, fresh): (Vec<RankedSpawn>, Vec<RankedSpawn>) =
        spawns.into_iter().partition(|spawn| {
            memory.is_some_and(|m| m.attempted.contains(&spawn.spawn))
        });
    let offered_count = fresh.len().min(limit.unwrap_or(usize::MAX));
    let remaining = fresh.len() - offered_count;
    let offered: Vec<SpawnRef> = fresh
        .into_iter()
        .take(offered_count)
        .map(|spawn| spawn.spawn)
        .collect();
    Window {
        pairs: narrowed(&plan.pairs, &offered.iter().cloned().collect()),
        offered,
        remaining,
        unfinished: repeated
            .into_iter()
            .map(|spawn| UnfinishedSpawn {
                spawn: spawn.spawn,
                rejected: spawn.rejected,
            })
            .collect(),
        unexpected: memory
            .map(|memory| unexpected(observed, memory))
            .unwrap_or_default(),
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

fn spawns_of(pair: &OutstandingPair) -> Vec<RankedSpawn> {
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
        Stage::SinglePass => vec![whole(false)],
        Stage::Compose(_) => vec![whole(true)],
        Stage::ResearchNodes(nodes) => nodes
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

fn narrowed(
    pairs: &[OutstandingPair],
    offered: &BTreeSet<SpawnRef>,
) -> Vec<OutstandingPair> {
    pairs
        .iter()
        .filter_map(|pair| {
            let mut pair = pair.clone();
            if let Stage::ResearchNodes(nodes) = &mut pair.stage {
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
        .collect()
}

fn unexpected(observed: &Observed, memory: &RunMemory) -> Vec<SpawnRef> {
    let notes = observed.notes.iter().filter_map(|(at, digest)| {
        let spawn = SpawnRef::Node(at.clone());
        let unexplained = memory
            .seen
            .notes
            .get(at)
            .map_or_else(|| !memory.excuses(&spawn), |seen| seen != digest);
        unexplained.then_some(spawn)
    });
    let findings = observed
        .answered
        .iter()
        .filter(|stem| !memory.seen.answered.contains(*stem))
        .map(|stem| SpawnRef::Pair(stem.clone()))
        .filter(|spawn| !memory.excuses(spawn));
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
    use super::UnfinishedSpawn;
    use super::Window;
    use crate::conduct::memory::PendingBatch;
    use crate::conduct::memory::RunMemory;
    use crate::conduct::observed::Observed;
    use crate::conduct::spawn::SpawnRef;
    use crate::topic::claims::ClaimedIndexes;
    use crate::topic::evidence::Digest;
    use crate::topic::evidence::NoteRejection;
    use crate::topic::layout::lineage::Lineage;
    use crate::topic::layout::note_ref::NoteRef;
    use crate::topic::layout::stem::Stem;
    use crate::topic::outline::Pair;
    use crate::topic::plan::OutstandingPair;
    use crate::topic::plan::RoundPlan;
    use crate::topic::tree::MissingNode;
    use crate::topic::tree::Stage;

    const SEEN: Digest = Digest::new([1; 32]);
    const CHANGED: Digest = Digest::new([2; 32]);

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

    fn outstanding(stem_text: &str, stage: Stage) -> OutstandingPair {
        OutstandingPair {
            pair: Pair {
                question: "Q?".into(),
                profile: "web".into(),
            },
            stem: stem(stem_text),
            stage,
        }
    }

    fn single_pass(stem_text: &str) -> OutstandingPair {
        outstanding(stem_text, Stage::SinglePass)
    }

    fn compose(stem_text: &str) -> OutstandingPair {
        outstanding(stem_text, Stage::Compose(vec![Lineage::root()]))
    }

    fn research_nodes(stem_text: &str, lineages: &[&str]) -> OutstandingPair {
        let nodes = lineages
            .iter()
            .map(|at| MissingNode {
                lineage: Lineage::parse(at).expect("a lineage"),
                question: format!("{at}?"),
                known_questions: Vec::new(),
                rejected: None,
            })
            .collect();
        outstanding(stem_text, Stage::ResearchNodes(nodes))
    }

    fn plan(pairs: Vec<OutstandingPair>) -> RoundPlan {
        RoundPlan {
            items: Vec::new(),
            pairs,
            skipped: Vec::new(),
            warnings: Vec::new(),
            shallow: Vec::new(),
            trims: Vec::new(),
            claims: ClaimedIndexes::default(),
        }
    }

    fn spawns(texts: &[&str]) -> Vec<SpawnRef> {
        texts.iter().map(|text| spawn(text)).collect()
    }

    fn set(texts: &[&str]) -> BTreeSet<SpawnRef> {
        spawns(texts).into_iter().collect()
    }

    fn unwatched(plan: &RoundPlan, limit: Option<usize>) -> Window {
        window(plan, &Observed::default(), limit, None)
    }

    fn offered(plan: &RoundPlan, limit: Option<usize>) -> Vec<String> {
        unwatched(plan, limit)
            .offered
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn showing(notes: &[(&str, Digest)], answered: &[&str]) -> Observed {
        Observed {
            notes: notes
                .iter()
                .map(|(at, digest)| (note_ref(at), *digest))
                .collect(),
            answered: answered.iter().map(|text| stem(text)).collect(),
        }
    }

    fn unexpected(observed: &Observed, memory: &RunMemory) -> Vec<String> {
        window(&plan(Vec::new()), observed, None, Some(memory))
            .unexpected
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn the_window_offers_node_research_by_level_before_compositions() {
        let plan = plan(vec![
            compose("01-a"),
            research_nodes("02-b", &["2-1", "2-2"]),
        ]);
        assert_eq!(offered(&plan, None), ["02-b:2-1", "02-b:2-2", "01-a"]);
    }

    #[test]
    fn the_window_orders_by_level_before_stem_across_pairs() {
        let plan = plan(vec![
            research_nodes("01-a", &["3-1-1"]),
            research_nodes("02-b", &["2-1"]),
            research_nodes("03-c", &["2-1"]),
        ]);
        assert_eq!(
            offered(&plan, None),
            ["02-b:2-1", "03-c:2-1", "01-a:3-1-1"]
        );
    }

    #[test]
    fn a_single_pass_pair_ranks_as_level_one_beside_node_research() {
        let plan = plan(vec![
            research_nodes("01-a", &["2-1"]),
            single_pass("02-b"),
            research_nodes("03-c", &["1"]),
        ]);
        assert_eq!(offered(&plan, None), ["02-b", "03-c:1", "01-a:2-1"]);
    }

    #[test]
    fn the_window_holds_back_spawns_past_its_limit_and_counts_them() {
        let window = unwatched(
            &plan(vec![
                single_pass("01-a"),
                single_pass("02-b"),
                single_pass("03-c"),
            ]),
            Some(2),
        );
        assert_eq!(window.offered, spawns(&["01-a", "02-b"]));
        assert_eq!(window.remaining, 1);
        assert_eq!(
            window.pairs,
            vec![single_pass("01-a"), single_pass("02-b")]
        );
    }

    #[test]
    fn a_limit_splitting_a_pairs_nodes_keeps_its_first_nodes() {
        let window = unwatched(
            &plan(vec![research_nodes("01-a", &["2-1", "2-2", "2-3"])]),
            Some(2),
        );
        assert_eq!(window.pairs, vec![research_nodes("01-a", &["2-1", "2-2"])]);
        assert_eq!(window.remaining, 1);
    }

    #[test]
    fn a_pair_whose_nodes_all_fall_past_the_limit_is_not_offered() {
        let window = unwatched(
            &plan(vec![research_nodes("02-b", &["2-1"]), single_pass("01-a")]),
            Some(1),
        );
        assert_eq!(window.pairs, vec![single_pass("01-a")]);
        assert_eq!(window.remaining, 1);
    }

    #[test]
    fn an_attempted_pair_still_outstanding_is_unfinished_without_a_rejection() {
        let memory = RunMemory {
            attempted: set(&["01-a"]),
            ..RunMemory::default()
        };
        let window = window(
            &plan(vec![single_pass("01-a")]),
            &Observed::default(),
            None,
            Some(&memory),
        );
        assert!(window.offered.is_empty());
        assert!(window.pairs.is_empty());
        assert_eq!(window.remaining, 0);
        assert_eq!(
            window.unfinished,
            vec![UnfinishedSpawn {
                spawn: spawn("01-a"),
                rejected: None
            }]
        );
    }

    #[test]
    fn an_attempted_spawn_is_never_offered_again() {
        let memory = RunMemory {
            attempted: set(&["01-a:1"]),
            ..RunMemory::default()
        };
        let window = window(
            &plan(vec![research_nodes("01-a", &["1"]), single_pass("02-b")]),
            &Observed::default(),
            None,
            Some(&memory),
        );
        assert_eq!(window.offered, spawns(&["02-b"]));
        assert_eq!(window.pairs, vec![single_pass("02-b")]);
    }

    #[test]
    fn an_attempted_spawn_still_outstanding_is_unfinished_with_its_rejection() {
        let mut refused = research_nodes("01-a", &["2-1"]);
        if let Stage::ResearchNodes(nodes) = &mut refused.stage {
            nodes[0].rejected = Some(NoteRejection::WrongKind);
        }
        let memory = RunMemory {
            attempted: set(&["01-a:2-1"]),
            ..RunMemory::default()
        };
        let window = window(
            &plan(vec![refused]),
            &Observed::default(),
            None,
            Some(&memory),
        );
        assert_eq!(
            window.unfinished,
            vec![UnfinishedSpawn {
                spawn: spawn("01-a:2-1"),
                rejected: Some(NoteRejection::WrongKind)
            }]
        );
    }

    #[test]
    fn a_window_without_memory_or_limit_offers_every_pair() {
        let planned = plan(vec![
            compose("02-b"),
            research_nodes("01-a", &["2-1"]),
            single_pass("03-c"),
        ]);
        let window =
            window(&planned, &showing(&[("04-d:1", SEEN)], &[]), None, None);
        assert_eq!(window.pairs, planned.pairs);
        assert_eq!(window.remaining, 0);
        assert!(window.unfinished.is_empty());
        assert!(window.unexpected.is_empty());
    }

    #[test]
    fn a_note_neither_seen_nor_attempted_is_unexpected() {
        assert_eq!(
            unexpected(
                &showing(&[("01-a:2-1", SEEN)], &[]),
                &RunMemory::default()
            ),
            ["01-a:2-1"]
        );
    }

    #[test]
    fn a_note_seen_at_start_or_attempted_is_never_unexpected() {
        let memory = RunMemory {
            seen: showing(&[("01-a:1", SEEN)], &[]),
            just_acknowledged: set(&["01-a:2-1"]),
            attempted: set(&["01-a:2-1"]),
            ..RunMemory::default()
        };
        let observed = showing(&[("01-a:1", SEEN), ("01-a:2-1", SEEN)], &[]);
        assert!(unexpected(&observed, &memory).is_empty());
    }

    #[test]
    fn an_accepted_note_whose_digest_changes_is_unexpected() {
        let memory = RunMemory {
            seen: showing(&[("01-a:1", SEEN)], &[]),
            just_acknowledged: set(&["01-a:1"]),
            ..RunMemory::default()
        };
        assert_eq!(
            unexpected(&showing(&[("01-a:1", CHANGED)], &[]), &memory),
            ["01-a:1"]
        );
    }

    #[test]
    fn a_note_refused_at_start_then_replaced_by_an_accepted_forgery_is_unexpected(
    ) {
        assert_eq!(
            unexpected(
                &showing(&[("01-a:1", CHANGED)], &[]),
                &RunMemory::default()
            ),
            ["01-a:1"]
        );
    }

    #[test]
    fn a_re_researched_note_that_answered_another_question_is_not_unexpected() {
        let memory = RunMemory {
            attempted: set(&["01-a:2-1"]),
            just_acknowledged: set(&["01-a:2-1"]),
            ..RunMemory::default()
        };
        assert!(unexpected(&showing(&[("01-a:2-1", CHANGED)], &[]), &memory)
            .is_empty());
    }

    #[test]
    fn a_note_appearing_later_for_a_failed_attempt_is_unexpected() {
        let memory = RunMemory {
            attempted: set(&["01-a:2-1"]),
            ..RunMemory::default()
        };
        assert_eq!(
            unexpected(&showing(&[("01-a:2-1", SEEN)], &[]), &memory),
            ["01-a:2-1"]
        );
    }

    #[test]
    fn writes_by_a_pending_unacknowledged_batch_are_not_unexpected() {
        let memory = RunMemory {
            pending: PendingBatch::new(0, spawns(&["01-a:2-1", "02-b"])),
            ..RunMemory::default()
        };
        let observed = showing(&[("01-a:2-1", SEEN)], &["02-b"]);
        assert!(unexpected(&observed, &memory).is_empty());
    }

    #[test]
    fn a_forged_finding_for_an_unattempted_pair_is_unexpected() {
        let memory = RunMemory {
            seen: showing(&[], &["02-b"]),
            ..RunMemory::default()
        };
        assert_eq!(
            unexpected(&showing(&[], &["01-a", "02-b"]), &memory),
            ["01-a"]
        );
    }

    #[test]
    fn a_seen_note_unchanged_is_never_unexpected() {
        let memory = RunMemory {
            seen: Observed {
                notes: BTreeMap::from([(note_ref("01-a:1"), SEEN)]),
                ..Observed::default()
            },
            ..RunMemory::default()
        };
        assert!(
            unexpected(&showing(&[("01-a:1", SEEN)], &[]), &memory).is_empty()
        );
    }
}
