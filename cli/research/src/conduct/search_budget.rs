//! How many `WebSearch` calls each web researcher may make: per-level
//! budgets sized from the session's search cap over the worst-case tree of
//! web nodes a run may still spawn.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::topic::plan::OutstandingPair;
use crate::topic::tree::WorstCaseTree;

const SEARCHING_PROFILE: &str = "web";

const RESERVE_ONE_IN: u32 = 20;
const WEIGHT_SCALE: u64 = level_weight(1);

/// Fixed rather than derived from the follow-up cap, so budgets keep their
/// shape if the starting follow-up cap ever moves.
const fn level_weight(level: u32) -> u64 {
    match level {
        1 => 4,
        2 => 2,
        _ => 1,
    }
}

/// The most `WebSearch` calls one Claude Code session may make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchCap(NonZeroU32);

impl SearchCap {
    #[must_use]
    pub const fn new(searches: NonZeroU32) -> Self {
        Self(searches)
    }

    #[must_use]
    pub const fn searches(self) -> NonZeroU32 {
        self.0
    }

    /// Kept back for searches outside the run: the main conversation,
    /// `outline`, and whatever the session spent before `conduct` began.
    #[must_use]
    pub const fn reserve(self) -> u32 {
        self.0.get().div_ceil(RESERVE_ONE_IN)
    }

    #[must_use]
    pub const fn available(self) -> u32 {
        self.0.get() - self.reserve()
    }
}

/// Each level's `WebSearch` allowance per web node, and the searches the
/// worst-case tree would make spending every allowance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchBudgets {
    budgets: BTreeMap<u32, NonZeroU32>,
    demand: u64,
    sized_against: SearchCap,
}

impl SearchBudgets {
    /// The largest budgets, shrinking level by level with depth, whose
    /// demand fits what the cap leaves after its reserve; budgets of 1
    /// everywhere when even those do not fit.
    #[must_use]
    pub fn sized(tree: &WorstCaseTree, cap: SearchCap) -> Self {
        if tree.is_empty() {
            return Self::from_parts(BTreeMap::new(), 0, cap);
        }
        let floor = Self::floor(tree, cap);
        if floor.overruns(cap) {
            return floor;
        }
        topped_up(
            tree,
            budgets_at(tree, largest_fitting_scale(tree, cap), cap),
            cap,
        )
    }

    #[must_use]
    pub fn floor(tree: &WorstCaseTree, cap: SearchCap) -> Self {
        budgets_at(tree, 1, cap)
    }

    #[must_use]
    pub const fn from_parts(
        budgets: BTreeMap<u32, NonZeroU32>,
        demand: u64,
        sized_against: SearchCap,
    ) -> Self {
        Self {
            budgets,
            demand,
            sized_against,
        }
    }

    #[must_use]
    pub fn at(&self, level: u32) -> Option<NonZeroU32> {
        self.budgets.get(&level).copied()
    }

    pub fn unbudgeted_levels<'a>(
        &'a self,
        tree: &'a WorstCaseTree,
    ) -> impl Iterator<Item = u32> + 'a {
        tree.populated_levels()
            .map(|(level, _)| level)
            .filter(|level| !self.budgets.contains_key(level))
    }

    pub fn levels(&self) -> impl Iterator<Item = (u32, NonZeroU32)> + '_ {
        self.budgets.iter().map(|(&level, &budget)| (level, budget))
    }

    #[must_use]
    pub const fn demand(&self) -> u64 {
        self.demand
    }

    #[must_use]
    pub const fn sized_against(&self) -> SearchCap {
        self.sized_against
    }

    /// Only budgets of 1 at every level can overrun: sizing otherwise
    /// shrinks them to fit.
    #[must_use]
    pub fn overruns(&self, cap: SearchCap) -> bool {
        self.demand > u64::from(cap.available())
    }
}

#[must_use]
pub fn searches(profile: &str) -> bool {
    profile == SEARCHING_PROFILE
}

#[must_use]
pub fn unsearched_tree(pairs: &[OutstandingPair]) -> WorstCaseTree {
    pairs
        .iter()
        .filter(|outstanding| searches(&outstanding.pair.profile))
        .fold(WorstCaseTree::default(), |tree, outstanding| {
            tree.joined(&outstanding.worst_case)
        })
}

/// `scale` is a budget multiplier in units of `1 / WEIGHT_SCALE`, so scale 1
/// grants every level the floor of 1.
fn budgets_at(
    tree: &WorstCaseTree,
    scale: u64,
    cap: SearchCap,
) -> SearchBudgets {
    let mut demand = 0_u64;
    let budgets = tree
        .populated_levels()
        .map(|(level, nodes)| {
            let budget = (scale.saturating_mul(level_weight(level))
                / WEIGHT_SCALE)
                .max(1);
            demand = demand.saturating_add(nodes.saturating_mul(budget));
            (level, saturated(budget))
        })
        .collect();
    SearchBudgets::from_parts(budgets, demand, cap)
}

fn saturated(budget: u64) -> NonZeroU32 {
    NonZeroU32::new(u32::try_from(budget).unwrap_or(u32::MAX))
        .unwrap_or(NonZeroU32::MIN)
}

/// Demand only grows with the scale, and at the search's upper end the
/// heaviest level's budget alone exceeds what is available, so the upper end
/// never fits.
fn largest_fitting_scale(tree: &WorstCaseTree, cap: SearchCap) -> u64 {
    let mut fitting = 1;
    let mut overrunning =
        WEIGHT_SCALE * u64::from(cap.available()) + WEIGHT_SCALE;
    while overrunning - fitting > 1 {
        let middle = fitting + (overrunning - fitting) / 2;
        if budgets_at(tree, middle, cap).overruns(cap) {
            overrunning = middle;
        } else {
            fitting = middle;
        }
    }
    fitting
}

/// Spends what flooring each level's budget left over, one search per node
/// at a time, on the deepest level that can rise without outgrowing the
/// level above it.
fn topped_up(
    tree: &WorstCaseTree,
    mut sized: SearchBudgets,
    cap: SearchCap,
) -> SearchBudgets {
    let available = u64::from(cap.available());
    while let Some((level, nodes, raised)) =
        rising_level(tree, &sized, available)
    {
        sized.budgets.insert(level, raised);
        sized.demand += nodes;
    }
    sized
}

fn rising_level(
    tree: &WorstCaseTree,
    sized: &SearchBudgets,
    available: u64,
) -> Option<(u32, u64, NonZeroU32)> {
    let levels: Vec<(u32, NonZeroU32)> = sized.levels().collect();
    (0..levels.len()).rev().find_map(|index| {
        let (level, budget) = levels[index];
        let nodes = tree.nodes_at(level);
        let raised = budget.checked_add(1)?;
        let fits = sized.demand.saturating_add(nodes) <= available;
        let keeps_shape = index
            .checked_sub(1)
            .is_none_or(|shallower| raised <= levels[shallower].1);
        (fits && keeps_shape).then_some((level, nodes, raised))
    })
}

#[cfg(test)]
mod tests {
    use super::searches;
    use super::unsearched_tree;
    use crate::topic::layout::stem::Stem;
    use crate::topic::outline::Pair;
    use crate::topic::plan::OutstandingPair;
    use crate::topic::tree::Stage;
    use crate::topic::tree::WorstCaseTree;

    fn deep_pair(index: u32, profile: &str) -> Option<OutstandingPair> {
        Some(OutstandingPair {
            pair: Pair {
                question: format!("Q{index}?"),
                profile: profile.to_owned(),
            },
            stem: Stem::allocated(index, &format!("q{index}"), profile)?,
            stage: Stage::SinglePass,
            worst_case: WorstCaseTree::from_levels(&[(1, 1), (2, 4), (3, 8)]),
        })
    }

    #[test]
    fn only_the_web_profile_searches() {
        assert!(searches("web"));
        for profile in ["openalex", "arxiv", "Web", ""] {
            assert!(!searches(profile), "{profile}");
        }
    }

    #[test]
    fn web_pairs_sum_and_other_pairs_add_nothing() {
        let pairs: Vec<OutstandingPair> = (1..=8)
            .filter_map(|index| deep_pair(index, "web"))
            .chain((9..=10).filter_map(|index| deep_pair(index, "openalex")))
            .collect();
        assert_eq!(pairs.len(), 10);
        assert_eq!(
            unsearched_tree(&pairs),
            WorstCaseTree::from_levels(&[(1, 8), (2, 32), (3, 64)])
        );
    }

    #[test]
    fn pairs_without_web_have_an_empty_unsearched_tree() {
        let pairs: Vec<OutstandingPair> = (1..=3)
            .filter_map(|index| deep_pair(index, "arxiv"))
            .collect();
        assert!(unsearched_tree(&pairs).is_empty());
    }
}
