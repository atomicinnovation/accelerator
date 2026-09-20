//! A total order over discovered ids, so a multi-scope pull reconciles the same
//! way every run: ascending by identifier prefix, then by numeric sequence
//! within a prefix.
//!
//! Ordering is a reconciliation concern, not a port one — [`ExternalId`] gains
//! no `Ord`. The decomposition is generic over any hyphenated remote id and is
//! independent of the local `work.id_pattern`, whose keyless numeric default
//! does not match a hyphenated remote key like `PP-2`.

use std::cmp::Ordering;

use tracker::ExternalId;

/// Orders discovered ids by `(prefix, numeric sequence, raw id)`.
///
/// The prefix is compared case- and whitespace-folded; the trailing digit run
/// is parsed to an integer so `PP-2` precedes `PP-10` and every `PP-*` precedes
/// every `XX-*`; the raw id is the final tie-break so the order is total over
/// distinct ids — a zero-padded pair like `PP-02` vs `PP-2` never compares
/// `Equal`.
#[must_use]
pub fn discovered_order(a: &ExternalId, b: &ExternalId) -> Ordering {
    let (a_prefix, a_sequence) = split_prefix_sequence(a.as_str());
    let (b_prefix, b_sequence) = split_prefix_sequence(b.as_str());
    a_prefix
        .cmp(&b_prefix)
        .then(a_sequence.cmp(&b_sequence))
        .then_with(|| a.as_str().cmp(b.as_str()))
}

/// Splits an id into its folded prefix and trailing numeric sequence, on the
/// *final* `-`-delimited numeric segment: everything before it is the prefix,
/// the trailing digits the sequence. A key with interior digits (`ABC2-5`)
/// keeps `ABC2` as its prefix. An id with no trailing digit run sorts with
/// sequence 0, disambiguated by the raw-id tie-break. The digit run parses
/// saturating to `usize::MAX`, so a very long sequence cannot overflow and
/// still orders after smaller numbers.
fn split_prefix_sequence(id: &str) -> (String, usize) {
    match id.rsplit_once('-') {
        Some((prefix, sequence))
            if !sequence.is_empty()
                && sequence.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            (fold(prefix), sequence.parse().unwrap_or(usize::MAX))
        }
        _ => (fold(id), 0),
    }
}

/// Folds a prefix the way `canonical_external_key` folds a whole id — strip
/// every whitespace character, upper-case the rest — so mixed-case survivors of
/// one logical prefix order by sequence rather than by ASCII case.
fn fold(prefix: &str) -> String {
    prefix
        .chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| character.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::discovered_order;
    use std::cmp::Ordering;
    use tracker::ExternalId;

    fn id(value: &str) -> ExternalId {
        ExternalId::new(value.to_owned())
    }

    fn sorted(values: &[&str]) -> Vec<String> {
        let mut ids: Vec<ExternalId> = values.iter().map(|v| id(v)).collect();
        ids.sort_by(discovered_order);
        ids.iter().map(|id| id.as_str().to_owned()).collect()
    }

    #[test]
    fn numeric_sequence_orders_within_a_prefix_and_prefixes_lexically() {
        assert_eq!(
            sorted(&["XX-3", "PP-10", "PP-2"]),
            vec!["PP-2", "PP-10", "XX-3"]
        );
    }

    #[test]
    fn a_key_with_interior_digits_keeps_its_whole_prefix() {
        // ABC2-5 sorts by prefix ABC2, not by collapsing to ABC.
        assert_eq!(
            sorted(&["ABC-9", "ABC2-5", "ABC2-1"]),
            vec!["ABC-9", "ABC2-1", "ABC2-5"]
        );
    }

    #[test]
    fn a_case_mixed_prefix_orders_by_sequence_not_ascii_case() {
        // 'E' (0x45) < 'e' (0x65), so a raw-string sort would wrongly
        // interleave; folding the prefix orders eng-2 after ENG-1 by sequence.
        assert_eq!(sorted(&["eng-2", "ENG-1"]), vec!["ENG-1", "eng-2"]);
    }

    #[test]
    fn the_order_is_total_over_degenerate_ids() {
        // A no-digit id, a zero-padded pair, and a huge sequence each order
        // deterministically, and no two distinct ids compare Equal.
        let ids = [
            id("PP-02"),
            id("PP-2"),
            id("NODIGITS"),
            id("PP-99999999999999999999999999"),
        ];
        for left in &ids {
            for right in &ids {
                if left.as_str() == right.as_str() {
                    assert_eq!(discovered_order(left, right), Ordering::Equal);
                } else {
                    assert_ne!(
                        discovered_order(left, right),
                        Ordering::Equal,
                        "{left} vs {right} must not compare Equal"
                    );
                }
            }
        }
        // The zero-padded pair is broken by the raw-id tie-break,
        // deterministically.
        assert_eq!(discovered_order(&id("PP-02"), &id("PP-2")), Ordering::Less);
    }
}
