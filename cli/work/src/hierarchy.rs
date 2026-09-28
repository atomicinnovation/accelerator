//! The parent/child hierarchy among work items: which items form a parent
//! cycle, and an order in which every parent precedes its children.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// One item in a hierarchy, named by a key its children's `parent` uses.
/// A `parent` naming no node in the set places no constraint on the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HierarchyNode<'a> {
    pub key: &'a str,
    pub parent: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchOrder<'a> {
    ParentsFirst(Vec<&'a str>),
    /// Every node on a parent cycle, in the order the nodes were given.
    Cycle(Vec<&'a str>),
}

/// Orders `nodes` so every parent precedes its children, keeping the given
/// order wherever the hierarchy allows, or names the cycle that makes no
/// such order possible.
#[must_use]
pub fn parents_first<'a>(nodes: &[HierarchyNode<'a>]) -> BatchOrder<'a> {
    let cyclic = cyclic_members(nodes);
    if !cyclic.is_empty() {
        return BatchOrder::Cycle(
            nodes
                .iter()
                .map(|node| node.key)
                .filter(|key| cyclic.contains(key))
                .collect(),
        );
    }
    let parents = parent_index(nodes);
    let mut ordered = Vec::with_capacity(nodes.len());
    let mut placed = BTreeSet::new();
    for node in nodes {
        let mut lineage = Vec::new();
        let mut cursor = Some(node.key);
        while let Some(key) = cursor.filter(|key| !placed.contains(key)) {
            lineage.push(key);
            cursor = parents.get(key).copied().flatten();
        }
        for key in lineage.into_iter().rev() {
            placed.insert(key);
            ordered.push(key);
        }
    }
    BatchOrder::ParentsFirst(ordered)
}

/// The nodes lying on a parent cycle. A node beneath a cycle, whose own
/// chain of parents leads into one, is not a member.
#[must_use]
pub fn cyclic_members<'a>(nodes: &[HierarchyNode<'a>]) -> BTreeSet<&'a str> {
    let parents = parent_index(nodes);
    nodes
        .iter()
        .map(|node| node.key)
        .filter(|&start| {
            let mut seen = BTreeSet::new();
            let mut cursor = parents.get(start).copied().flatten();
            while let Some(key) = cursor {
                if key == start {
                    return true;
                }
                if !seen.insert(key) {
                    return false;
                }
                cursor = parents.get(key).copied().flatten();
            }
            false
        })
        .collect()
}

/// Each node's parent, where that parent is itself a node.
fn parent_index<'a>(
    nodes: &[HierarchyNode<'a>],
) -> BTreeMap<&'a str, Option<&'a str>> {
    let keys: BTreeSet<&str> = nodes.iter().map(|node| node.key).collect();
    nodes
        .iter()
        .map(|node| {
            (node.key, node.parent.filter(|parent| keys.contains(parent)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn node<'a>(
        key: &'a str,
        parent: Option<&'a str>,
    ) -> HierarchyNode<'a> {
        HierarchyNode { key, parent }
    }

    #[test]
    fn parents_come_before_children() {
        let nodes = [
            node("story-b", Some("epic")),
            node("task", Some("story-a")),
            node("story-a", Some("epic")),
            node("epic", None),
        ];

        assert_eq!(
            parents_first(&nodes),
            BatchOrder::ParentsFirst(vec![
                "epic", "story-b", "story-a", "task"
            ])
        );
    }

    #[test]
    fn a_cycle_is_reported_with_every_member() {
        let nodes = [
            node("a", Some("c")),
            node("free", None),
            node("b", Some("a")),
            node("c", Some("b")),
            node("under-the-cycle", Some("a")),
        ];

        assert_eq!(
            parents_first(&nodes),
            BatchOrder::Cycle(vec!["a", "b", "c"])
        );
        assert_eq!(cyclic_members(&nodes), BTreeSet::from(["a", "b", "c"]));
    }

    #[test]
    fn a_node_that_is_its_own_parent_is_a_cycle() {
        let nodes = [node("a", Some("a"))];

        assert_eq!(parents_first(&nodes), BatchOrder::Cycle(vec!["a"]));
    }

    #[test]
    fn a_parent_outside_the_batch_does_not_constrain_order() {
        let nodes = [
            node("child", Some("epic")),
            node("orphan", Some("work-item:0042")),
            node("epic", Some("PP-760")),
        ];

        assert_eq!(
            parents_first(&nodes),
            BatchOrder::ParentsFirst(vec!["epic", "child", "orphan"])
        );
    }
}
