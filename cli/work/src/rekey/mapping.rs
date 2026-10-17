//! What each item becomes under a numeric pattern.

use corpus::WorkItemIdScheme;

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
}
