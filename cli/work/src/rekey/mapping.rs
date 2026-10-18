//! What each item becomes: under a numeric pattern, a number; under
//! `{tracker}`, its issue's current key or a draft.

use std::collections::BTreeMap;

use corpus::WorkItemIdScheme;

use crate::draft_id::mint_draft_ids;
use crate::draft_id::DraftId;
use crate::draft_id::SuffixDraws;
use crate::identity::ItemIdentity;
use crate::next_number::allocate_above;
use crate::next_number::pattern_cap;
use crate::next_number::AllocationError;
use crate::rekey::classify::classify;
use crate::rekey::classify::Classification;
use crate::rekey::collision::refuse_collisions;
use crate::rekey::creation_date::CreationDate;
use crate::rekey::entry::RekeyEntry;
use crate::rekey::entry::RekeyTarget;
use crate::rekey::outcome::NotRekeyed;
use crate::rekey::sequence::is_in_target_shape;
use crate::rekey::sequence::project_key_of;
use crate::rekey::sequence::rendered;
use crate::rekey::sequence::sequence_of;
use crate::tracker_key::TrackerKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekeyCandidate {
    pub identity: ItemIdentity,
    pub created: CreationDate,
}

/// Every item off the pattern, by old `id`, with what it becomes.
///
/// A kept number is re-rendered under its own project key, or under
/// `work.key` when it has none. A tracker key or draft takes a fresh number
/// under `work.key`, in creation-date order, above every number that key
/// already holds.
///
/// # Errors
///
/// [`NotRekeyed`] when an item's `id` has no known shape, a kept or fresh
/// number does not fit the pattern, or a target collides.
pub fn numeric_mapping(
    scheme: &WorkItemIdScheme,
    candidates: &[RekeyCandidate],
) -> Result<Vec<RekeyEntry>, NotRekeyed> {
    let cap = pattern_cap(&scheme.id_pattern);
    let mut entries = Vec::new();
    let mut needing_fresh = Vec::new();
    for candidate in candidates {
        let identity = &candidate.identity;
        match classify(identity) {
            Classification::TrackerKey | Classification::Draft => {
                needing_fresh.push(candidate);
            }
            Classification::Sequenced(_)
                if is_in_target_shape(&identity.id, scheme) => {}
            Classification::Sequenced(number) if number > cap => {
                return Err(NotRekeyed::KeptNumberAboveCap {
                    item: identity.path.clone(),
                    number,
                    pattern: scheme.id_pattern.clone(),
                });
            }
            Classification::Sequenced(number) => entries.push(renumbered(
                &identity.id,
                rendered(scheme, kept_key(&identity.id, scheme), number),
            )),
            Classification::Unclassifiable => {
                return Err(NotRekeyed::Unclassifiable(identity.path.clone()));
            }
        }
    }

    let floor = default_key_floor(scheme, candidates, &entries);
    entries.extend(fresh_entries(scheme, floor, needing_fresh)?);
    entries.sort_by(|left, right| left.old.cmp(&right.old));
    let items: Vec<ItemIdentity> = candidates
        .iter()
        .map(|candidate| candidate.identity.clone())
        .collect();
    refuse_collisions(&entries, &items)?;
    Ok(entries)
}

fn renumbered(old: &str, new: String) -> RekeyEntry {
    RekeyEntry {
        old: old.to_owned(),
        target: RekeyTarget::Renumbered(new),
    }
}

fn default_key(scheme: &WorkItemIdScheme) -> Option<&str> {
    if corpus::references_key(&scheme.id_pattern) {
        scheme.key.as_deref()
    } else {
        None
    }
}

fn kept_key<'a>(id: &'a str, scheme: &'a WorkItemIdScheme) -> Option<&'a str> {
    default_key(scheme).map(|fallback| project_key_of(id).unwrap_or(fallback))
}

/// The highest number held, as an `id`, an alias or a kept target, bare or
/// under the default key.
fn default_key_floor(
    scheme: &WorkItemIdScheme,
    candidates: &[RekeyCandidate],
    kept: &[RekeyEntry],
) -> u64 {
    let under_default_key = |token: &&str| {
        project_key_of(token).is_none_or(|key| {
            default_key(scheme).is_some_and(|own| own.eq_ignore_ascii_case(key))
        })
    };
    candidates
        .iter()
        .flat_map(|candidate| {
            std::iter::once(candidate.identity.id.as_str())
                .chain(candidate.identity.aliases.iter().map(String::as_str))
        })
        .chain(kept.iter().map(|entry| entry.target.id()))
        .filter(under_default_key)
        .filter_map(sequence_of)
        .max()
        .unwrap_or(0)
}

fn fresh_entries(
    scheme: &WorkItemIdScheme,
    floor: u64,
    mut needing_fresh: Vec<&RekeyCandidate>,
) -> Result<Vec<RekeyEntry>, NotRekeyed> {
    if needing_fresh.is_empty() {
        return Ok(Vec::new());
    }
    needing_fresh.sort_by(|left, right| {
        left.created
            .cmp(&right.created)
            .then_with(|| left.identity.id.cmp(&right.identity.id))
    });
    let needed = u32::try_from(needing_fresh.len()).unwrap_or(u32::MAX);
    let numbers =
        allocate_above(scheme, floor, needed).map_err(|error| match error {
            AllocationError::Overflow { highest, cap, .. } => {
                NotRekeyed::FreshNumbersExhausted {
                    needed: u64::from(needed),
                    remaining: cap.saturating_sub(highest),
                    pattern: scheme.id_pattern.clone(),
                }
            }
            AllocationError::MissingKey | AllocationError::ProjectUnused => {
                unreachable!("allocate_above checks no key")
            }
        })?;
    Ok(needing_fresh
        .into_iter()
        .zip(numbers)
        .map(|(candidate, number)| renumbered(&candidate.identity.id, number))
        .collect())
}

/// The key each issue answers to now, by the key an item stores for it,
/// for every issue a refresh found moved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshedKeys(BTreeMap<String, TrackerKey>);

impl RefreshedKeys {
    /// `stored`'s current key; a key the refresh found unmoved or not at
    /// all answers as itself.
    #[must_use]
    pub fn current_key_for(&self, stored: &TrackerKey) -> TrackerKey {
        self.0
            .get(stored.as_str())
            .cloned()
            .unwrap_or_else(|| stored.clone())
    }
}

impl FromIterator<(String, TrackerKey)> for RefreshedKeys {
    fn from_iter<I: IntoIterator<Item = (String, TrackerKey)>>(
        moves: I,
    ) -> Self {
        Self(moves.into_iter().collect())
    }
}

/// The stored key of every synced item [`tracker_mapping`] may move, for a
/// refresh to read the current key of.
///
/// # Errors
///
/// [`NotRekeyed::Unclassifiable`] naming the first item whose
/// `external_id` is not shaped like a tracker key, so it never reaches a
/// tracker that cannot read it.
pub fn keys_to_refresh(
    candidates: &[RekeyCandidate],
) -> Result<Vec<TrackerKey>, NotRekeyed> {
    candidates
        .iter()
        .map(|candidate| &candidate.identity)
        .filter(|identity| DraftId::parse(&identity.id).is_none())
        .filter_map(|identity| {
            identity.external_id.as_deref().map(|stored| {
                TrackerKey::parse(stored).ok_or_else(|| {
                    NotRekeyed::Unclassifiable(identity.path.clone())
                })
            })
        })
        .collect()
}

/// Every item off `{tracker}`, by old `id`, with what it becomes.
///
/// A synced item takes its issue's current key; an unsynced item becomes a
/// draft under a freshly minted ID; a draft is left as it is.
///
/// # Errors
///
/// [`NotRekeyed`] when an `external_id` is not shaped like a tracker key,
/// no free draft ID can be drawn, or a target collides.
pub fn tracker_mapping(
    candidates: &[RekeyCandidate],
    refreshed: &RefreshedKeys,
    draws: &mut dyn SuffixDraws,
) -> Result<Vec<RekeyEntry>, NotRekeyed> {
    let items: Vec<ItemIdentity> = candidates
        .iter()
        .map(|candidate| candidate.identity.clone())
        .collect();
    let mut entries = Vec::new();
    let mut unsynced = Vec::new();
    for identity in items
        .iter()
        .filter(|identity| DraftId::parse(&identity.id).is_none())
    {
        let Some(stored) = identity.external_id.as_deref() else {
            unsynced.push(identity.id.clone());
            continue;
        };
        let stored = TrackerKey::parse(stored)
            .ok_or_else(|| NotRekeyed::Unclassifiable(identity.path.clone()))?;
        let current = refreshed.current_key_for(&stored);
        if !current.as_str().eq_ignore_ascii_case(&identity.id) {
            entries.push(RekeyEntry {
                old: identity.id.clone(),
                target: RekeyTarget::TrackerKey(current),
            });
        }
    }
    let drafts = mint_draft_ids(draws, &items, unsynced.len())
        .map_err(NotRekeyed::DraftIdExhausted)?;
    entries.extend(unsynced.into_iter().zip(drafts).map(|(old, draft)| {
        RekeyEntry {
            old,
            target: RekeyTarget::Draft(draft),
        }
    }));
    entries.sort_by(|left, right| left.old.cmp(&right.old));
    refuse_collisions(&entries, &items)?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    // DSL pattern strings such as "{key}" are literal token markers, not
    // format args.
    #![allow(clippy::literal_string_with_formatting_args)]

    use std::path::PathBuf;

    use corpus::WorkItemIdScheme;

    use super::numeric_mapping;
    use super::RekeyCandidate;
    use crate::rekey::creation_date::CreationDate;
    use crate::rekey::entry::RekeyEntry;
    use crate::rekey::entry::RekeyTarget;
    use crate::rekey::outcome::NotRekeyed;
    use crate::work_item_files::identity_of;
    use crate::work_item_files::WorkItemFile;

    fn keyed(key: &str) -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: "{key}-{number:04d}".to_owned(),
            key: Some(key.to_owned()),
        }
    }

    fn acc() -> WorkItemIdScheme {
        keyed("ACC")
    }

    fn path_of(id: &str) -> PathBuf {
        let directory = if id.starts_with("draft-") {
            "meta/work/drafts"
        } else {
            "meta/work"
        };
        PathBuf::from(format!("{directory}/{id}-item.md"))
    }

    /// An item read from its frontmatter, so its creation date is read the
    /// way the command reads it.
    fn candidate(frontmatter: &str) -> RekeyCandidate {
        let id = frontmatter
            .lines()
            .find_map(|line| line.strip_prefix("id: "))
            .map(|id| id.trim_matches('"'))
            .unwrap_or_default();
        let file = WorkItemFile {
            path: path_of(id),
            content: format!("---\n{frontmatter}\n---\n"),
        };
        RekeyCandidate {
            identity: identity_of(&file)
                .unwrap_or_else(|| unreachable!("an identifiable item")),
            created: CreationDate::of(&file),
        }
    }

    fn local(id: &str) -> RekeyCandidate {
        candidate(&format!("id: \"{id}\""))
    }

    fn synced(key: &str, date: &str) -> RekeyCandidate {
        candidate(&format!(
            "id: \"{key}\"\nexternal_id: \"{key}\"\ndate: \"{date}\""
        ))
    }

    fn draft(id: &str, date: &str) -> RekeyCandidate {
        candidate(&format!("id: \"{id}\"\ndate: \"{date}\""))
    }

    fn renumbered(old: &str, new: &str) -> RekeyEntry {
        RekeyEntry {
            old: old.to_owned(),
            target: RekeyTarget::Renumbered(new.to_owned()),
        }
    }

    fn mapped(
        scheme: &WorkItemIdScheme,
        candidates: &[RekeyCandidate],
    ) -> Vec<(String, String)> {
        numeric_mapping(scheme, candidates)
            .unwrap_or_else(|refusal| unreachable!("{refusal}"))
            .into_iter()
            .map(|entry| (entry.old.clone(), entry.target.id().to_owned()))
            .collect()
    }

    fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
        expected
            .iter()
            .map(|(old, new)| ((*old).to_owned(), (*new).to_owned()))
            .collect()
    }

    #[test]
    fn a_legacy_numeric_id_is_rerendered_under_a_key() {
        assert_eq!(
            mapped(&acc(), &[local("0042")]),
            pairs(&[("0042", "ACC-0042")])
        );
    }

    #[test]
    fn a_prefixed_id_is_rerendered_bare() {
        assert_eq!(
            mapped(&WorkItemIdScheme::numeric(), &[local("ACC-0042")]),
            pairs(&[("ACC-0042", "0042")])
        );
    }

    #[test]
    fn fresh_numbers_start_above_every_kept_number() {
        let corpus = [
            local("ACC-0041"),
            local("0042"),
            synced("PP-760", "2026-01-01"),
        ];
        assert_eq!(
            mapped(&acc(), &corpus),
            pairs(&[("0042", "ACC-0042"), ("PP-760", "ACC-0043")])
        );
    }

    #[test]
    fn fresh_numbers_start_above_every_id_and_alias_in_target_shape() {
        let corpus = [
            candidate("id: \"ACC-0041\"\naliases: [\"ACC-0050\"]"),
            local("ACC-0047"),
            synced("PP-760", "2026-01-01"),
        ];
        assert_eq!(mapped(&acc(), &corpus), pairs(&[("PP-760", "ACC-0051")]));
    }

    #[test]
    fn fresh_numbers_start_above_a_bare_alias_that_canonicalises_into_target_shape(
    ) {
        let corpus = [
            candidate("id: \"ACC-0041\"\naliases: [\"0045\"]"),
            synced("PP-760", "2026-01-01"),
        ];
        assert_eq!(mapped(&acc(), &corpus), pairs(&[("PP-760", "ACC-0046")]));
    }

    #[test]
    fn fresh_numbers_follow_date_order() {
        let corpus = [
            synced("PP-760", "2026-02-01T00:00:00+00:00"),
            draft("draft-k3x9mq", "2026-01-01T00:00:00+00:00"),
        ];
        assert_eq!(
            mapped(&acc(), &corpus),
            pairs(&[("PP-760", "ACC-0002"), ("draft-k3x9mq", "ACC-0001")])
        );
    }

    #[test]
    fn equal_dates_fall_back_to_old_id_byte_order() {
        let corpus = [
            draft("draft-a1b2cd", "2026-01-01T00:00:00+00:00"),
            synced("PP-761", "2026-01-01T00:00:00+00:00"),
        ];
        assert_eq!(
            mapped(&acc(), &corpus),
            pairs(&[("PP-761", "ACC-0001"), ("draft-a1b2cd", "ACC-0002")])
        );
    }

    #[test]
    fn undated_items_take_fresh_numbers_after_dated_ones() {
        let corpus = [
            candidate("id: \"draft-a1b2cd\""),
            draft("draft-k3x9mq", "2026-06-01"),
        ];
        assert_eq!(
            mapped(&acc(), &corpus),
            pairs(&[
                ("draft-a1b2cd", "ACC-0002"),
                ("draft-k3x9mq", "ACC-0001")
            ])
        );
    }

    #[test]
    fn a_mixed_corpus_converges_onto_the_pattern() {
        let corpus = [
            local("0041"),
            synced("PP-760", "2026-02-01"),
            draft("draft-k3x9mq", "2026-01-01"),
        ];
        assert_eq!(
            mapped(&acc(), &corpus),
            pairs(&[
                ("0041", "ACC-0041"),
                ("PP-760", "ACC-0043"),
                ("draft-k3x9mq", "ACC-0042"),
            ])
        );
    }

    #[test]
    fn an_item_already_on_its_target_is_not_an_entry() {
        assert_eq!(mapped(&acc(), &[local("ACC-0042")]), pairs(&[]));
    }

    #[test]
    fn an_item_differing_from_its_target_only_in_case_is_not_an_entry() {
        assert_eq!(mapped(&acc(), &[local("acc-0042")]), pairs(&[]));
    }

    #[test]
    fn fresh_numbers_past_the_cap_are_refused() {
        let corpus = [
            local("ACC-9998"),
            draft("draft-k3x9mq", "2026-01-01"),
            draft("draft-a1b2cd", "2026-01-02"),
        ];
        assert_eq!(
            numeric_mapping(&acc(), &corpus),
            Err(NotRekeyed::FreshNumbersExhausted {
                needed: 2,
                remaining: 1,
                pattern: "{key}-{number:04d}".to_owned(),
            })
        );
    }

    #[test]
    fn a_kept_number_past_the_cap_is_refused_naming_the_item() {
        assert_eq!(
            numeric_mapping(&WorkItemIdScheme::numeric(), &[local("PP-12345")]),
            Err(NotRekeyed::KeptNumberAboveCap {
                item: path_of("PP-12345"),
                number: 12_345,
                pattern: "{number:04d}".to_owned(),
            })
        );
    }

    #[test]
    fn a_renumbering_entry_carries_a_renumbered_target() {
        assert_eq!(
            numeric_mapping(&acc(), &[local("0042")]),
            Ok(vec![renumbered("0042", "ACC-0042")])
        );
    }

    #[test]
    fn a_multi_project_corpus_is_a_fixed_point() {
        let corpus = [local("ACC-0001"), local("XYZ-0001"), local("AB1-0001")];
        assert_eq!(mapped(&acc(), &corpus), pairs(&[]));
    }

    #[test]
    fn a_width_change_keeps_each_items_project_key() {
        let corpus = [local("ACC-001"), local("XYZ-001")];
        assert_eq!(
            mapped(&acc(), &corpus),
            pairs(&[("ACC-001", "ACC-0001"), ("XYZ-001", "XYZ-0001")])
        );
    }

    #[test]
    fn a_bare_kept_number_moves_to_the_default_key() {
        assert_eq!(
            mapped(&acc(), &[local("0042")]),
            pairs(&[("0042", "ACC-0042")])
        );
    }

    #[test]
    fn fresh_numbers_go_to_the_default_key_above_its_own_floor() {
        let corpus = [
            local("ACC-0003"),
            local("XYZ-0009"),
            draft("draft-k3x9mq", "2026-01-01"),
        ];
        assert_eq!(
            mapped(&acc(), &corpus),
            pairs(&[("draft-k3x9mq", "ACC-0004")])
        );
    }

    #[test]
    fn an_unsynced_tracker_shaped_id_keeps_its_own_key_under_a_key_pattern() {
        assert_eq!(
            mapped(&acc(), &[local("PP-760")]),
            pairs(&[("PP-760", "PP-0760")])
        );
    }

    #[test]
    fn a_draft_id_takes_a_fresh_number() {
        assert_eq!(
            mapped(&acc(), &[draft("draft-k3x9mq", "2026-01-01")]),
            pairs(&[("draft-k3x9mq", "ACC-0001")])
        );
    }

    #[test]
    fn a_tracker_key_in_target_shape_still_takes_a_fresh_number() {
        assert_eq!(
            mapped(&acc(), &[synced("PP-7600", "2026-01-01")]),
            pairs(&[("PP-7600", "ACC-0001")])
        );
    }

    #[test]
    fn an_unclassifiable_id_is_refused_naming_the_item() {
        assert_eq!(
            numeric_mapping(&acc(), &[local("notes")]),
            Err(NotRekeyed::Unclassifiable(path_of("notes")))
        );
    }

    #[test]
    fn a_colliding_target_is_refused() {
        assert!(matches!(
            numeric_mapping(&acc(), &[local("0042"), local("ACC-0042")]),
            Err(NotRekeyed::Collision { .. })
        ));
    }

    mod tracker {
        use std::collections::VecDeque;

        use super::candidate;
        use super::path_of;
        use super::RekeyCandidate;
        use crate::draft_id::DraftId;
        use crate::draft_id::SuffixDraws;
        use crate::draft_id::ALPHABET;
        use crate::rekey::entry::RekeyEntry;
        use crate::rekey::entry::RekeyTarget;
        use crate::rekey::mapping::keys_to_refresh;
        use crate::rekey::mapping::tracker_mapping;
        use crate::rekey::mapping::RefreshedKeys;
        use crate::rekey::outcome::NotRekeyed;
        use crate::tracker_key::TrackerKey;

        struct Scripted(VecDeque<String>);

        impl SuffixDraws for Scripted {
            fn draw(&mut self) -> [u8; 6] {
                let suffix = self.0.pop_front().unwrap_or_default();
                let mut indices = [0; 6];
                for (index, byte) in indices.iter_mut().zip(suffix.bytes()) {
                    *index = u8::try_from(
                        ALPHABET
                            .iter()
                            .position(|letter| *letter == byte)
                            .unwrap_or_default(),
                    )
                    .unwrap_or_default();
                }
                indices
            }
        }

        fn drawing(suffixes: &[&str]) -> Scripted {
            Scripted(suffixes.iter().map(|&suffix| suffix.to_owned()).collect())
        }

        fn key(raw: &str) -> TrackerKey {
            TrackerKey::parse(raw)
                .unwrap_or_else(|| unreachable!("a tracker key"))
        }

        fn linked(id: &str, external_id: &str) -> RekeyCandidate {
            candidate(&format!("id: \"{id}\"\nexternal_id: \"{external_id}\""))
        }

        fn unlinked(id: &str) -> RekeyCandidate {
            candidate(&format!("id: \"{id}\""))
        }

        fn onto(old: &str, new: &str) -> RekeyEntry {
            RekeyEntry {
                old: old.to_owned(),
                target: RekeyTarget::TrackerKey(key(new)),
            }
        }

        fn moved(stored: &str, current: &str) -> RefreshedKeys {
            std::iter::once((stored.to_owned(), key(current))).collect()
        }

        fn mapped(
            candidates: &[RekeyCandidate],
            refreshed: &RefreshedKeys,
        ) -> Result<Vec<RekeyEntry>, NotRekeyed> {
            tracker_mapping(candidates, refreshed, &mut drawing(&["k3x9mq"]))
        }

        fn collision(first: &str, second: &str, target: &str) -> NotRekeyed {
            NotRekeyed::Collision {
                first: path_of(first),
                second: path_of(second),
                target: target.to_owned(),
            }
        }

        #[test]
        fn a_synced_legacy_item_targets_its_external_id() {
            assert_eq!(
                mapped(&[linked("0042", "PP-760")], &RefreshedKeys::default()),
                Ok(vec![onto("0042", "PP-760")])
            );
        }

        #[test]
        fn a_synced_item_whose_issue_moved_targets_the_new_key_and_carries_it()
        {
            let entries = mapped(
                &[linked("PP-760", "PP-760")],
                &moved("PP-760", "ENG-42"),
            );

            assert_eq!(entries, Ok(vec![onto("PP-760", "ENG-42")]));
            let entries = entries.unwrap_or_default();
            assert_eq!(entries[0].retirement().new_external_id, Some("ENG-42"));
        }

        #[test]
        fn an_unmoved_synced_item_is_not_an_entry() {
            assert_eq!(
                mapped(
                    &[linked("PP-760", "PP-760")],
                    &RefreshedKeys::default()
                ),
                Ok(Vec::new())
            );
        }

        #[test]
        fn an_unsynced_item_becomes_a_draft_with_a_minted_id() {
            let draft = DraftId::parse("draft-k3x9mq")
                .unwrap_or_else(|| unreachable!("a draft ID"));
            assert_eq!(
                mapped(&[unlinked("0042")], &RefreshedKeys::default()),
                Ok(vec![RekeyEntry {
                    old: "0042".to_owned(),
                    target: RekeyTarget::Draft(draft),
                }])
            );
        }

        #[test]
        fn an_existing_draft_is_not_an_entry() {
            assert_eq!(
                mapped(&[unlinked("draft-a1b2cd")], &RefreshedKeys::default()),
                Ok(Vec::new())
            );
        }

        #[test]
        fn an_id_equal_to_its_key_ignoring_case_is_not_an_entry() {
            assert_eq!(
                mapped(
                    &[linked("pp-760", "PP-760")],
                    &RefreshedKeys::default()
                ),
                Ok(Vec::new())
            );
        }

        #[test]
        fn a_tracker_target_held_as_another_items_id_is_refused_naming_both() {
            let corpus = [linked("0042", "PP-760"), linked("PP-760", "ENG-1")];
            assert_eq!(
                mapped(&corpus, &RefreshedKeys::default()),
                Err(collision("0042", "PP-760", "PP-760"))
            );
        }

        #[test]
        fn a_tracker_target_held_as_another_items_alias_is_refused_naming_both()
        {
            let corpus = [
                linked("0042", "PP-760"),
                candidate(
                    "id: \"ENG-1\"\nexternal_id: \"ENG-1\"\naliases: [\"PP-760\"]",
                ),
            ];
            assert_eq!(
                mapped(&corpus, &RefreshedKeys::default()),
                Err(collision("0042", "ENG-1", "PP-760"))
            );
        }

        #[test]
        fn two_items_sharing_an_external_id_are_refused_naming_both() {
            let corpus = [linked("0041", "PP-760"), linked("0042", "PP-760")];
            assert_eq!(
                mapped(&corpus, &RefreshedKeys::default()),
                Err(collision("0041", "0042", "PP-760"))
            );
        }

        #[test]
        fn a_moved_key_equal_to_another_items_id_is_refused_naming_both() {
            let corpus =
                [linked("PP-760", "PP-760"), linked("ENG-42", "ENG-42")];
            assert_eq!(
                mapped(&corpus, &moved("PP-760", "ENG-42")),
                Err(collision("PP-760", "ENG-42", "ENG-42"))
            );
        }

        #[test]
        fn a_not_found_key_keeps_its_stored_value() {
            assert_eq!(
                RefreshedKeys::default().current_key_for(&key("PP-760")),
                key("PP-760")
            );
            assert_eq!(
                mapped(&[linked("0042", "PP-760")], &RefreshedKeys::default()),
                Ok(vec![onto("0042", "PP-760")])
            );
        }

        #[test]
        fn an_external_id_of_no_tracker_shape_is_unclassifiable() {
            assert_eq!(
                mapped(
                    &[linked("0042", "not a key")],
                    &RefreshedKeys::default()
                ),
                Err(NotRekeyed::Unclassifiable(path_of("0042")))
            );
        }

        #[test]
        fn the_keys_to_refresh_are_every_synced_non_draft_items_stored_key() {
            assert_eq!(
                keys_to_refresh(&[
                    linked("0042", "PP-760"),
                    unlinked("0043"),
                    linked("draft-a1b2cd", "PP-761"),
                ]),
                Ok(vec![key("PP-760")])
            );
        }

        #[test]
        fn a_key_to_refresh_of_no_tracker_shape_is_unclassifiable() {
            assert_eq!(
                keys_to_refresh(&[linked("0042", "not a key")]),
                Err(NotRekeyed::Unclassifiable(path_of("0042")))
            );
        }

        #[test]
        fn draft_draws_that_all_collide_are_refused() {
            let corpus = [unlinked("draft-k3x9mq"), unlinked("0042")];
            assert!(matches!(
                tracker_mapping(
                    &corpus,
                    &RefreshedKeys::default(),
                    &mut drawing(&["k3x9mq"; 16]),
                ),
                Err(NotRekeyed::DraftIdExhausted(_))
            ));
        }
    }
}
