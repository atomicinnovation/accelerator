//! The sequence number an `id` carries, and whether an `id` is already in the
//! shape the configured pattern renders.

// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#![allow(clippy::literal_string_with_formatting_args)]

use crate::id_pattern::IdKey;
use crate::id_scheme::WorkItemIdScheme;

/// The number an `id` carries: all of a bare numeric `id`, or the digits
/// after the last `-` of a tracker-shaped one.
#[must_use]
pub fn sequence_of(id: &str) -> Option<u64> {
    let digits = if is_digits(id) {
        id
    } else if tracker::is_tracker_key(id) {
        id.rsplit_once('-')?.1
    } else {
        return None;
    };
    digits.parse().ok()
}

/// The local project key an `id` is prefixed with: a letter, then letters or
/// digits, before `-<digits>`.
pub(crate) fn project_key_of(id: &str) -> Option<&str> {
    let (key, digits) = id.split_once('-')?;
    let well_formed = key.starts_with(|c: char| c.is_ascii_alphabetic())
        && key.chars().all(|c| c.is_ascii_alphanumeric())
        && is_digits(digits);
    well_formed.then_some(key)
}

/// Whether the pattern, rendering `id`'s own number under `work.key`, gives
/// back `id`, ignoring case; an `id` under any other project key is not in
/// shape.
#[must_use]
pub fn is_in_target_shape(id: &str, scheme: &WorkItemIdScheme) -> bool {
    sequence_of(id).is_some_and(|number| {
        rendered(scheme, scheme.key(), number).eq_ignore_ascii_case(id)
    })
}

/// `number` as the pattern renders it under `key`.
pub(crate) fn rendered(
    scheme: &WorkItemIdScheme,
    key: Option<&IdKey>,
    number: u64,
) -> String {
    scheme
        .pattern()
        .render(key, number)
        .unwrap_or_else(|_| number.to_string())
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::is_in_target_shape;
    use super::sequence_of;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::WorkItemIdScheme;

    fn keyed(key: &str) -> WorkItemIdScheme {
        WorkItemIdScheme::new(
            IdPattern::parse("{key}-{number:04d}")
                .unwrap_or_else(|error| unreachable!("{error}")),
            IdKey::parse(key).ok(),
        )
    }

    #[test]
    fn a_bare_numeric_id_yields_its_sequence_number() {
        assert_eq!(sequence_of("0042"), Some(42));
    }

    #[test]
    fn a_prefixed_id_yields_its_sequence_number() {
        assert_eq!(sequence_of("ACC-0042"), Some(42));
        assert_eq!(sequence_of("PP-760"), Some(760));
    }

    #[test]
    fn an_id_of_no_numeric_shape_yields_none() {
        assert_eq!(sequence_of("draft-k3x9mq"), None);
        assert_eq!(sequence_of("ACC-"), None);
        assert_eq!(sequence_of("-0042"), None);
        assert_eq!(sequence_of("notes"), None);
        assert_eq!(sequence_of("99999999999999999999999"), None);
    }

    #[test]
    fn an_id_is_in_target_shape_only_when_it_renders_back_to_itself_ignoring_case(
    ) {
        let scheme = keyed("ACC");
        assert!(is_in_target_shape("ACC-0042", &scheme));
        assert!(is_in_target_shape("acc-0042", &scheme));
        assert!(!is_in_target_shape("ACC-042", &scheme));
        assert!(!is_in_target_shape("0042", &scheme));
        assert!(is_in_target_shape("0042", &WorkItemIdScheme::numeric()));
        assert!(!is_in_target_shape(
            "ACC-0042",
            &WorkItemIdScheme::numeric()
        ));
        assert!(!is_in_target_shape("42", &WorkItemIdScheme::numeric()));
    }

    #[test]
    fn an_id_under_another_project_key_is_not_in_target_shape() {
        assert!(!is_in_target_shape("XYZ-0001", &keyed("ACC")));
        assert!(!is_in_target_shape("AB1-0001", &keyed("ACC")));
    }

    #[test]
    fn an_id_under_another_key_at_the_wrong_width_is_not_in_target_shape() {
        assert!(!is_in_target_shape("PP-760", &keyed("ACC")));
    }
}
