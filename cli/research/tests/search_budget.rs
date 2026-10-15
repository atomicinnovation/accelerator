#![allow(clippy::expect_used)]

use std::num::NonZeroU32;

use research::conduct::search_budget::SearchBudgets;
use research::conduct::search_budget::SearchCap;
use research::topic::tree::WorstCaseTree;

fn tree(levels: &[(u32, u64)]) -> WorstCaseTree {
    WorstCaseTree::from_levels(levels)
}

const fn cap(searches: u32) -> SearchCap {
    SearchCap::new(NonZeroU32::new(searches).expect("a non-zero cap"))
}

fn shallowest_first(nodes: &str) -> Vec<(u32, u64)> {
    (1..)
        .zip(nodes.split('/'))
        .filter_map(|(level, count)| Some((level, count.parse().ok()?)))
        .collect()
}

fn budgets(sized: &SearchBudgets) -> Vec<(u32, u32)> {
    sized
        .levels()
        .map(|(level, budget)| (level, budget.get()))
        .collect()
}

fn per_level(budgets: &str) -> Vec<(u32, u32)> {
    shallowest_first(budgets)
        .into_iter()
        .map(|(level, budget)| {
            (level, u32::try_from(budget).expect("a u32 budget"))
        })
        .collect()
}

fn assert_sized(searches: u32, nodes: &str, expected: &str, demand: u64) {
    let sized =
        SearchBudgets::sized(&tree(&shallowest_first(nodes)), cap(searches));
    let case = format!("cap {searches}, tree {nodes}");
    assert_eq!(budgets(&sized), per_level(expected), "{case}");
    assert_eq!(sized.demand(), demand, "{case}");
}

#[test]
fn the_reserve_is_a_twentieth_of_the_cap_rounded_up() {
    for (searches, reserve, available) in [
        (200, 10, 190),
        (192, 10, 182),
        (201, 11, 190),
        (60, 3, 57),
        (1, 1, 0),
    ] {
        assert_eq!(cap(searches).reserve(), reserve, "cap {searches}");
        assert_eq!(cap(searches).available(), available, "cap {searches}");
    }
}

#[test]
fn budgets_fill_the_cap_less_its_reserve() {
    for (searches, nodes, expected, demand) in [
        (200, "10/40/80", "3/2/1", 190),
        (200, "3/12/24", "15/6/3", 189),
        (500, "10/40/80", "11/5/2", 470),
        (500, "10/40/80/80/80", "7/4/1/1/1", 470),
        (200, "10/40", "7/3", 190),
        (200, "2/8/16/16/16", "11/5/3/3/2", 190),
        (192, "14/56/112", "1/1/1", 182),
        (200, "8/32/64", "7/2/1", 184),
        (200, "1/4/8", "42/19/9", 190),
        (201, "1/4/8", "42/19/9", 190),
        (60, "1/4/8", "13/5/3", 57),
        (200, "10", "19", 190),
        (200, "–/2/7", "–/35/17", 189),
    ] {
        assert_sized(searches, nodes, expected, demand);
        let sized = SearchBudgets::sized(
            &tree(&shallowest_first(nodes)),
            cap(searches),
        );
        assert!(!sized.overruns(cap(searches)), "cap {searches}, {nodes}");
    }
}

#[test]
fn budgets_of_one_that_overrun_stay_at_one() {
    for (nodes, demand) in [("20/80/160", 260), ("15/60/120", 195)] {
        assert_sized(200, nodes, "1/1/1", demand);
        let sized =
            SearchBudgets::sized(&tree(&shallowest_first(nodes)), cap(200));
        assert!(sized.overruns(cap(200)), "{nodes}");
    }
}

#[test]
fn a_resized_remainder_would_grant_more() {
    assert_sized(200, "–/10/20", "–/9/5", 190);
}

#[test]
fn a_level_without_remaining_nodes_takes_no_budget() {
    let sized = SearchBudgets::sized(&tree(&[(2, 2), (3, 7)]), cap(200));
    let levels: Vec<u32> = sized.levels().map(|(level, _)| level).collect();
    assert_eq!(levels, [2, 3]);
}

#[test]
fn an_empty_tree_sizes_no_budgets_and_no_demand() {
    let sized = SearchBudgets::sized(&WorstCaseTree::default(), cap(200));
    assert_eq!(budgets(&sized), []);
    assert_eq!(sized.demand(), 0);
    assert!(!sized.overruns(cap(200)));
}

#[test]
fn a_level_without_a_budget_is_budgeted_one() {
    let sized = SearchBudgets::sized(&tree(&[(2, 2), (3, 7)]), cap(200));
    assert_eq!(sized.at(1), NonZeroU32::MIN);
    assert_eq!(sized.at(4), NonZeroU32::MIN);
}

fn could_rise(
    sized: &SearchBudgets,
    tree: &WorstCaseTree,
    cap: SearchCap,
) -> Option<u32> {
    let levels: Vec<(u32, u32)> = budgets(sized);
    levels
        .iter()
        .enumerate()
        .find_map(|(index, &(level, budget))| {
            let fits = sized.demand() + tree.nodes_at(level)
                <= u64::from(cap.available());
            let keeps_shape = index == 0 || budget < levels[index - 1].1;
            (fits && keeps_shape).then_some(level)
        })
}

fn assert_well_sized(tree: &WorstCaseTree, searches: SearchCap) {
    let sized = SearchBudgets::sized(tree, searches);
    let levels = budgets(&sized);
    let case = format!("{tree:?} at {searches:?}");
    let every_budget_is_one = levels.iter().all(|&(_, budget)| budget == 1);
    assert!(
        sized.demand() <= u64::from(searches.available())
            || every_budget_is_one,
        "{case} overruns with budgets {levels:?}"
    );
    assert!(
        levels.windows(2).all(|pair| pair[0].1 >= pair[1].1),
        "{case} grows with depth: {levels:?}"
    );
    assert_eq!(
        could_rise(&sized, tree, searches),
        None,
        "{case} is not maximal: {levels:?}"
    );
}

#[test]
fn sized_budgets_are_maximal_shaped_and_within_the_cap() {
    const GROWTH: [u64; 4] = [1, 4, 8, 8];
    for shallow in shallow_node_counts(4, 0..=3) {
        let levels: Vec<(u32, u64)> = (1..)
            .zip(shallow.iter().zip(GROWTH))
            .map(|(level, (&nodes, growth))| (level, nodes * growth))
            .collect();
        let tree = tree(&levels);
        if tree.is_empty() {
            continue;
        }
        for searches in 1..=260 {
            assert_well_sized(&tree, cap(searches));
        }
    }
}

fn shallow_node_counts(
    length: usize,
    values: std::ops::RangeInclusive<u64>,
) -> Vec<Vec<u64>> {
    (0..length).fold(vec![Vec::new()], |prefixes, _| {
        prefixes
            .into_iter()
            .flat_map(|prefix| {
                values.clone().map(move |value| {
                    let mut extended = prefix.clone();
                    extended.push(value);
                    extended
                })
            })
            .collect()
    })
}

#[test]
fn a_huge_cap_sizes_without_overflow() {
    let tree = tree(&[(1, 1), (2, 4), (3, 8)]);
    let searches = SearchCap::new(NonZeroU32::MAX);
    assert_well_sized(&tree, searches);
    assert!(!SearchBudgets::sized(&tree, searches).overruns(searches));
}
