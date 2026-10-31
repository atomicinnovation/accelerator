//! The number an `id` carries, and whether an `id` is already in the shape
//! the configured pattern renders.
//!
//! Re-key runs after the pattern has changed, and nothing records the
//! pattern that wrote an `id`, so a reading never assumes it.

use crate::draft_id::DraftId;
use crate::id_scheme::WorkItemIdScheme;
use crate::path_component::is_one_path_component;

/// The key an existing `id` counts under, when it names one, and its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingId {
    pub key: Option<String>,
    pub number: u64,
}

/// Why an `id` yields no number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unclassified {
    AmbiguousDigitRuns,
    NoNumber,
    NotOnePathComponent,
}

/// `id` read through the configured pattern, then as all digits or
/// `<key>-<digits>`, then as its one digit run.
///
/// # Errors
///
/// The [`Unclassified`] cause when no reading gives a number, including for
/// a draft ID, which never carries one.
pub fn read_existing_id(
    id: &str,
    scheme: &WorkItemIdScheme,
) -> Result<ExistingId, Unclassified> {
    if DraftId::parse(id).is_some() {
        return Err(Unclassified::NoNumber);
    }
    if !is_one_path_component(id) || id.chars().any(char::is_control) {
        return Err(Unclassified::NotOnePathComponent);
    }
    if let Some(parts) = scheme.read_held(id) {
        return Ok(ExistingId {
            key: parts.key().map(str::to_owned),
            number: parts.number(),
        });
    }
    if is_digits(id) {
        return number(id).map(|number| ExistingId { key: None, number });
    }
    if tracker::is_tracker_key(id) {
        let (key, digits) =
            id.rsplit_once('-').ok_or(Unclassified::NoNumber)?;
        return number(digits).map(|number| ExistingId {
            key: Some(key.to_owned()),
            number,
        });
    }
    sole_digit_run(id)
}

/// [`read_existing_id`]'s reading, when it gives a number.
#[must_use]
pub fn existing_id(id: &str, scheme: &WorkItemIdScheme) -> Option<ExistingId> {
    read_existing_id(id, scheme).ok()
}

fn sole_digit_run(id: &str) -> Result<ExistingId, Unclassified> {
    if !id.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return Err(Unclassified::NoNumber);
    }
    let mut runs = id
        .split(|c: char| !c.is_ascii_digit())
        .filter(|run| !run.is_empty());
    match (runs.next(), runs.next()) {
        (Some(run), None) => {
            number(run).map(|number| ExistingId { key: None, number })
        }
        (Some(_), Some(_)) => Err(Unclassified::AmbiguousDigitRuns),
        (None, _) => Err(Unclassified::NoNumber),
    }
}

fn number(digits: &str) -> Result<u64, Unclassified> {
    digits.parse().map_err(|_| Unclassified::NoNumber)
}

/// Whether the pattern, rendering `id`'s own number under `work.key`, gives
/// back `id`, ignoring case; an `id` under any other key is not in shape.
#[must_use]
pub fn is_in_target_shape(id: &str, scheme: &WorkItemIdScheme) -> bool {
    existing_id(id, scheme).is_some_and(|existing| {
        scheme
            .pattern()
            .render(scheme.key(), existing.number)
            .is_ok_and(|rendered| rendered.eq_ignore_ascii_case(id))
    })
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::existing_id;
    use super::is_in_target_shape;
    use super::ExistingId;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::WorkItemIdScheme;

    fn scheme(pattern: &str, key: Option<&str>) -> WorkItemIdScheme {
        WorkItemIdScheme::new(
            IdPattern::parse(pattern)
                .unwrap_or_else(|error| unreachable!("{error}")),
            key.and_then(|key| IdKey::parse(key).ok()),
        )
    }

    fn keyed(key: &str) -> WorkItemIdScheme {
        scheme("{key}-{number:04d}", Some(key))
    }

    fn number_of(id: &str) -> Option<u64> {
        existing_id(id, &keyed("ACC")).map(|existing| existing.number)
    }

    #[test]
    fn a_bare_numeric_id_yields_its_sequence_number() {
        assert_eq!(number_of("0042"), Some(42));
    }

    #[test]
    fn a_prefixed_id_yields_its_sequence_number() {
        assert_eq!(number_of("ACC-0042"), Some(42));
        assert_eq!(number_of("PP-760"), Some(760));
    }

    #[test]
    fn an_id_of_no_numeric_shape_yields_none() {
        assert_eq!(number_of("draft-k3x9mq"), None);
        assert_eq!(number_of("ACC-"), None);
        assert_eq!(number_of("-0042"), None);
        assert_eq!(number_of("notes"), None);
        assert_eq!(number_of("99999999999999999999999"), None);
    }

    #[test]
    fn an_id_the_configured_pattern_alone_reads_keeps_its_key() {
        assert_eq!(
            existing_id("X1_0001", &scheme("{key}_{number:04d}", Some("PP"))),
            Some(ExistingId {
                key: Some("X1".to_owned()),
                number: 1,
            })
        );
    }

    #[test]
    fn an_id_is_in_target_shape_only_when_it_renders_back_to_itself_ignoring_case(
    ) {
        let scheme = keyed("ACC");
        let numeric = WorkItemIdScheme::numeric();
        assert!(is_in_target_shape("ACC-0042", &scheme));
        assert!(is_in_target_shape("acc-0042", &scheme));
        assert!(!is_in_target_shape("ACC-042", &scheme));
        assert!(!is_in_target_shape("0042", &scheme));
        assert!(is_in_target_shape("0042", &numeric));
        assert!(!is_in_target_shape("ACC-0042", &numeric));
        assert!(!is_in_target_shape("42", &numeric));
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

    #[test]
    fn an_item_rendered_with_literal_text_is_in_target_shape() {
        assert!(is_in_target_shape("v001", &scheme("v{number:03d}", None)));
        assert!(is_in_target_shape(
            "PPv001",
            &scheme("{key}v{number:03d}", Some("PP"))
        ));
    }

    #[test]
    fn an_unprefixed_item_is_rekeyed_once_then_left_alone() {
        let versioned = scheme("v{number:03d}", None);
        assert!(!is_in_target_shape("001", &versioned));
        assert!(is_in_target_shape("v001", &versioned));
    }

    #[test]
    fn an_underscored_key_item_is_in_target_shape() {
        assert!(is_in_target_shape(
            "PP_0001",
            &scheme("{key}_{number:04d}", Some("PP"))
        ));
    }

    #[test]
    fn a_zero_item_stays_in_target_shape() {
        assert!(is_in_target_shape("0000", &WorkItemIdScheme::numeric()));
    }

    #[test]
    fn a_literal_case_difference_stays_in_target_shape() {
        assert!(is_in_target_shape("V001", &scheme("v{number:03d}", None)));
        assert!(is_in_target_shape("pp-0001", &keyed("PP")));
        assert!(!is_in_target_shape("XYZ-0002", &keyed("ACC")));
    }
}
