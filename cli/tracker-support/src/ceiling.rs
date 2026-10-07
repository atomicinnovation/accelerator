//! The one conversion from a written ceiling string to a [`Ceiling`].
//!
//! Config resolution and the CLI value parser route through it. Structural
//! config validation lives in `config`, which cannot name [`Ceiling`]; a test
//! pins that the two accept the same tokens.

use tracker::Ceiling;
use tracker::UNLIMITED_TOKEN;

/// Interprets a written ceiling token into a [`Ceiling`], or `None` when it is
/// not a valid bound.
///
/// `allow_zero` admits `0` — the `max_items` refuse-all — which a transport
/// page cap forbids, since a zero-page loop returns a silent complete-empty
/// result. Rejects a float, a negative, or any non-numeric token other than
/// the [`UNLIMITED_TOKEN`] sentinel.
#[must_use]
pub fn from_token(token: &str, allow_zero: bool) -> Option<Ceiling> {
    if token == UNLIMITED_TOKEN {
        return Some(Ceiling::Unlimited);
    }
    match token.parse::<usize>() {
        Ok(0) if !allow_zero => None,
        Ok(bound) => Some(Ceiling::Bounded(bound)),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::from_token;
    use tracker::Ceiling;

    #[test]
    fn a_positive_integer_is_a_bounded_ceiling() {
        assert_eq!(from_token("25", true), Some(Ceiling::Bounded(25)));
        assert_eq!(from_token("1", false), Some(Ceiling::Bounded(1)));
    }

    #[test]
    fn the_unlimited_sentinel_is_the_unbounded_ceiling() {
        assert_eq!(from_token("unlimited", true), Some(Ceiling::Unlimited));
        assert_eq!(from_token("unlimited", false), Some(Ceiling::Unlimited));
    }

    #[test]
    fn zero_is_a_bound_only_when_allowed() {
        assert_eq!(from_token("0", true), Some(Ceiling::Bounded(0)));
        assert_eq!(from_token("0", false), None);
    }

    #[test]
    fn a_float_a_negative_or_text_is_rejected() {
        assert_eq!(from_token("2.5", true), None);
        assert_eq!(from_token("-1", true), None);
        assert_eq!(from_token("lots", true), None);
        assert_eq!(from_token("", true), None);
    }

    #[test]
    fn interpretation_accepts_exactly_what_config_validation_accepts() {
        assert_eq!(config::tracker_block::UNLIMITED, tracker::UNLIMITED_TOKEN);
        for token in
            ["0", "1", "25", "unlimited", "2.5", "-1", "lots", "", " 3"]
        {
            for allow_zero in [true, false] {
                assert_eq!(
                    from_token(token, allow_zero).is_some(),
                    config::tracker_block::is_valid_ceiling(token, allow_zero),
                    "{token:?} allow_zero={allow_zero}"
                );
            }
        }
    }
}
