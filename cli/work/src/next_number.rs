//! Next-sequential-ID allocation.

use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;
use crate::id_pattern::KeyError;
use crate::id_pattern::MissingKey;
use crate::work_item_files::is_md_filename;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationError {
    MissingKey,
    ProjectUnused,
    InvalidProject(KeyError),
    Overflow {
        partial: Vec<String>,
        highest: u64,
        highest_file: Option<String>,
        cap: u64,
    },
}

impl From<MissingKey> for AllocationError {
    fn from(_: MissingKey) -> Self {
        Self::MissingKey
    }
}

/// The key allocation renders under: `project` when given, otherwise the
/// `configured` one, or none under a keyless pattern.
///
/// # Errors
///
/// [`AllocationError::ProjectUnused`] when `project` is given to a pattern
/// with no key token, checked before `project` is parsed;
/// [`AllocationError::InvalidProject`] when the key to render under is not a
/// valid key.
pub fn allocation_key(
    pattern: &IdPattern,
    project: Option<&str>,
    configured: Option<&str>,
) -> Result<Option<IdKey>, AllocationError> {
    if !pattern.references_key() {
        return match project {
            Some(_) => Err(AllocationError::ProjectUnused),
            None => Ok(None),
        };
    }
    project
        .or(configured)
        .map(IdKey::parse)
        .transpose()
        .map_err(AllocationError::InvalidProject)
}

/// Allocates `count` sequential IDs following the highest number `key`
/// already holds among `filenames`, ignoring the key's letter case.
///
/// # Errors
///
/// [`AllocationError::MissingKey`] when the pattern has a key token and
/// `key` is `None`; [`AllocationError::Overflow`] when `highest + count`
/// would exceed the pattern's numeric cap — the error still carries every ID
/// that fit before the boundary.
pub fn allocate(
    pattern: &IdPattern,
    key: Option<&IdKey>,
    count: u32,
    filenames: &[String],
) -> Result<Vec<String>, AllocationError> {
    if pattern.references_key() && key.is_none() {
        return Err(AllocationError::MissingKey);
    }
    let in_sequence = |parts: &IdParts| match (parts.key(), key) {
        (Some(held), Some(key)) => held.eq_ignore_ascii_case(key.as_str()),
        (held, key) => held.is_none() && key.is_none(),
    };
    let (highest, highest_file) = filenames
        .iter()
        .filter(|filename| is_md_filename(filename))
        .filter_map(|filename| {
            IdParts::parse_prefix(pattern, filename)
                .map(|(parts, _)| parts)
                .filter(in_sequence)
                .map(|parts| (parts.number(), Some(filename.clone())))
        })
        .max_by_key(|(number, _)| *number)
        .unwrap_or((0, None));

    allocate_above(pattern, key, highest, count).map_err(|error| match error {
        AllocationError::Overflow {
            partial,
            highest,
            cap,
            ..
        } => AllocationError::Overflow {
            partial,
            highest,
            highest_file,
            cap,
        },
        other => other,
    })
}

fn allocate_above(
    pattern: &IdPattern,
    key: Option<&IdKey>,
    floor: u64,
    count: u32,
) -> Result<Vec<String>, AllocationError> {
    let cap = pattern.cap().unwrap_or(u64::MAX);
    let render = |number: u64| pattern.render(key, number);

    if floor.saturating_add(u64::from(count)) > cap {
        let partial = (1..=u64::from(count))
            .map_while(|i| floor.checked_add(i))
            .take_while(|&next| next <= cap)
            .map(render)
            .collect::<Result<_, _>>()?;
        return Err(AllocationError::Overflow {
            partial,
            highest: floor,
            highest_file: None,
            cap,
        });
    }

    Ok((1..=u64::from(count))
        .map(|i| render(floor + i))
        .collect::<Result<_, _>>()?)
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::unwrap_used, clippy::literal_string_with_formatting_args)]
mod tests {
    use super::{allocate, allocate_above, allocation_key, AllocationError};
    use crate::id_pattern::{IdKey, IdPattern, KeyError};

    fn pattern(text: &str) -> IdPattern {
        IdPattern::parse(text).unwrap()
    }

    fn key(text: &str) -> IdKey {
        IdKey::parse(text).unwrap()
    }

    fn numeric() -> IdPattern {
        pattern("{number:04d}")
    }

    fn names(filenames: &[&str]) -> Vec<String> {
        filenames.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn allocates_the_next_sequential_id_from_an_empty_corpus() {
        let result = allocate(&numeric(), None, 1, &[]);
        assert_eq!(result, Ok(vec!["0001".to_owned()]));
    }

    #[test]
    fn allocates_after_the_highest_existing_number() {
        let result = allocate(&numeric(), None, 1, &names(&["0001-foo.md"]));
        assert_eq!(result, Ok(vec!["0002".to_owned()]));
    }

    #[test]
    fn allocates_a_sequential_batch() {
        let result = allocate(&numeric(), None, 3, &names(&["0001-first.md"]));
        assert_eq!(
            result,
            Ok(vec![
                "0002".to_owned(),
                "0003".to_owned(),
                "0004".to_owned()
            ])
        );
    }

    #[test]
    fn missing_key_is_rejected() {
        assert_eq!(
            allocate(&pattern("{project}-{number:04d}"), None, 1, &[]),
            Err(AllocationError::MissingKey)
        );
    }

    #[test]
    fn the_allocation_key_is_the_project_or_the_configured_key() {
        let keyed = pattern("{key}-{number:04d}");
        let configured = Some("PP");
        assert_eq!(
            allocation_key(&keyed, None, configured),
            Ok(Some(key("PP")))
        );
        assert_eq!(
            allocation_key(&keyed, Some("XY"), configured),
            Ok(Some(key("XY")))
        );
        assert_eq!(
            allocation_key(&keyed, Some("1X"), configured),
            Err(AllocationError::InvalidProject(KeyError::BadKeyValue(
                "1X".to_owned()
            )))
        );
        for keyless in ["{number:04d}", "{tracker}"] {
            assert_eq!(
                allocation_key(&pattern(keyless), Some("1X"), configured),
                Err(AllocationError::ProjectUnused),
                "{keyless}"
            );
            assert_eq!(
                allocation_key(&pattern(keyless), None, configured),
                Ok(None),
                "{keyless}"
            );
        }
    }

    #[test]
    fn a_holder_under_another_key_is_not_in_the_sequence() {
        let keyed = pattern("{key}-{number:04d}");
        let filenames =
            names(&["XY-0400-a.md", "pp-0007-b.md", "PP-0003-c.MD"]);
        assert_eq!(
            allocate(&keyed, Some(&key("PP")), 1, &filenames),
            Ok(vec!["PP-0008".to_owned()])
        );
    }

    #[test]
    fn overflow_from_a_stray_out_of_width_file() {
        let result = allocate(&numeric(), None, 1, &names(&["19999-stray.md"]));
        assert_eq!(
            result,
            Err(AllocationError::Overflow {
                partial: vec![],
                highest: 19999,
                highest_file: Some("19999-stray.md".to_owned()),
                cap: 9999,
            })
        );
    }

    #[test]
    fn overflow_from_ordinary_number_space_exhaustion() {
        let result = allocate(&numeric(), None, 3, &names(&["9999-last.md"]));
        assert_eq!(
            result,
            Err(AllocationError::Overflow {
                partial: vec![],
                highest: 9999,
                highest_file: Some("9999-last.md".to_owned()),
                cap: 9999,
            })
        );
    }

    #[test]
    fn overflow_partial_emission_before_the_boundary() {
        let result = allocate(&numeric(), None, 3, &names(&["9998-near.md"]));
        assert_eq!(
            result,
            Err(AllocationError::Overflow {
                partial: vec!["9999".to_owned()],
                highest: 9998,
                highest_file: Some("9998-near.md".to_owned()),
                cap: 9999,
            })
        );
    }

    #[test]
    fn allocation_above_a_floor_starts_after_it() {
        assert_eq!(
            allocate_above(
                &pattern("{key}-{number:04d}"),
                Some(&key("ACC")),
                41,
                2
            ),
            Ok(vec!["ACC-0042".to_owned(), "ACC-0043".to_owned()])
        );
    }

    #[test]
    fn allocation_above_the_largest_floor_overflows_with_nothing_allocated() {
        assert_eq!(
            allocate_above(&numeric(), None, u64::MAX, 2),
            Err(AllocationError::Overflow {
                partial: Vec::new(),
                highest: u64::MAX,
                highest_file: None,
                cap: 9999,
            })
        );
    }
}
