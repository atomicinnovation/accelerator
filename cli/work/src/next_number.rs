//! Next-sequential-ID allocation.

use std::collections::BTreeSet;
use std::path::Path;

use crate::id_pattern::escaped;
use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;
use crate::id_pattern::KeyError;
use crate::id_pattern::MissingKey;
use crate::id_scheme::WorkItemIdScheme;
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
        numbers_held: u64,
    },
}

impl AllocationError {
    /// The refusal as the user reads it, naming a holder inside `work_dir`.
    #[must_use]
    pub fn message(&self, pattern: &IdPattern, work_dir: &Path) -> String {
        let pattern = pattern.as_str();
        match self {
            Self::MissingKey => format!(
                "E_PATTERN_MISSING_KEY: pattern '{pattern}' references the \
                 {{key}} prefix but no value supplied — pass --project or \
                 set work.key"
            ),
            Self::ProjectUnused => format!(
                "E_PATTERN_KEY_UNUSED: --project is meaningless for pattern \
                 '{pattern}' (no {{key}} token)"
            ),
            Self::InvalidProject(error) => {
                let KeyError::BadKeyValue(value) = error;
                format!(
                    "E_WORK_KEY_INVALID: --project '{}': {error}",
                    escaped(value)
                )
            }
            Self::Overflow {
                highest,
                highest_file,
                cap,
                numbers_held,
                ..
            } => {
                let Some(highest_file) =
                    highest_file.as_deref().filter(|_| highest >= cap)
                else {
                    return format!(
                        "E_PATTERN_OVERFLOW: number space exhausted \
                         (highest={highest}, cap={cap}). Archive completed \
                         work items or widen the pattern."
                    );
                };
                if highest > cap {
                    return format!(
                        "E_PATTERN_OVERFLOW: out-of-width file '{}' has \
                         number {highest} exceeding the pattern '{pattern}' \
                         cap of {cap}. Rename the stray file or widen the \
                         pattern.",
                        escaped(highest_file)
                    );
                }
                let remedy = if numbers_held < cap {
                    "If that ID was a slip, move the item to a free lower ID \
                     by renaming its file and changing its id, H1 and \
                     references; otherwise archive completed work items or \
                     widen the pattern."
                } else {
                    "Archive completed work items or widen the pattern."
                };
                format!(
                    "E_PATTERN_OVERFLOW: number space exhausted: {} holds \
                     {highest}, the cap of pattern '{pattern}'. {remedy}",
                    escaped(&work_dir.join(highest_file).to_string_lossy())
                )
            }
        }
    }
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
    configured: Option<&IdKey>,
) -> Result<Option<IdKey>, AllocationError> {
    if !pattern.references_key() {
        return match project {
            Some(_) => Err(AllocationError::ProjectUnused),
            None => Ok(None),
        };
    }
    project.map_or_else(
        || Ok(configured.cloned()),
        |project| {
            IdKey::parse(project)
                .map(Some)
                .map_err(AllocationError::InvalidProject)
        },
    )
}

/// Allocates `count` sequential IDs following the highest number `key`
/// already holds among `filenames`, ignoring the key's letter case.
///
/// A file named by a bare legacy number counts under the scheme's own key.
///
/// # Errors
///
/// [`AllocationError::MissingKey`] when the pattern has a key token and
/// `key` is `None`; [`AllocationError::Overflow`] when `highest + count`
/// would exceed the pattern's numeric cap — the error still carries every ID
/// that fit before the boundary.
pub fn allocate(
    scheme: &WorkItemIdScheme,
    key: Option<&IdKey>,
    count: u32,
    filenames: &[String],
) -> Result<Vec<String>, AllocationError> {
    let pattern = scheme.pattern();
    if pattern.references_key() && key.is_none() {
        return Err(AllocationError::MissingKey);
    }
    let in_sequence = |parts: &IdParts| match (parts.key(), key) {
        (Some(held), Some(key)) => held.eq_ignore_ascii_case(key.as_str()),
        (held, key) => held.is_none() && key.is_none(),
    };
    let held: Vec<(u64, &String)> = filenames
        .iter()
        .filter(|filename| is_md_filename(filename))
        .filter_map(|filename| {
            scheme
                .read_held_prefix(filename)
                .map(|(parts, _)| parts)
                .filter(in_sequence)
                .map(|parts| (parts.number(), filename))
        })
        .collect();
    let (highest, highest_file) = held
        .iter()
        .max_by_key(|(number, _)| *number)
        .map_or((0, None), |(number, filename)| {
            (*number, Some((*filename).clone()))
        });
    let numbers_held = held
        .iter()
        .map(|(number, _)| *number)
        .collect::<BTreeSet<_>>()
        .len();

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
            numbers_held: u64::try_from(numbers_held).unwrap_or(u64::MAX),
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
            numbers_held: 0,
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
    use std::path::Path;

    use super::{allocate, allocate_above, allocation_key, AllocationError};
    use crate::id_pattern::{IdKey, IdPattern, KeyError};
    use crate::id_scheme::WorkItemIdScheme;

    fn pattern(text: &str) -> IdPattern {
        IdPattern::parse(text).unwrap()
    }

    fn key(text: &str) -> IdKey {
        IdKey::parse(text).unwrap()
    }

    fn scheme(text: &str, configured: Option<&str>) -> WorkItemIdScheme {
        WorkItemIdScheme::new(pattern(text), configured.map(key))
    }

    fn numeric() -> WorkItemIdScheme {
        WorkItemIdScheme::numeric()
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
            allocate(&scheme("{project}-{number:04d}", None), None, 1, &[]),
            Err(AllocationError::MissingKey)
        );
    }

    #[test]
    fn the_allocation_key_is_the_project_or_the_configured_key() {
        let keyed = pattern("{key}-{number:04d}");
        let pp = key("PP");
        let configured = Some(&pp);
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
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let filenames =
            names(&["XY-0400-a.md", "pp-0007-b.md", "PP-0003-c.MD"]);
        assert_eq!(
            allocate(&keyed, Some(&key("PP")), 1, &filenames),
            Ok(vec!["PP-0008".to_owned()])
        );
    }

    #[test]
    fn allocation_counts_a_legacy_number_under_the_configured_key() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let filenames = names(&["PP-0001-a.md", "0400-b.md"]);
        assert_eq!(
            allocate(&keyed, Some(&key("PP")), 1, &filenames),
            Ok(vec!["PP-0401".to_owned()])
        );
        assert_eq!(
            allocate(&keyed, Some(&key("XY")), 1, &filenames),
            Ok(vec!["XY-0001".to_owned()])
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
                numbers_held: 1,
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
                numbers_held: 1,
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
                numbers_held: 1,
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
            allocate_above(&pattern("{number:04d}"), None, u64::MAX, 2),
            Err(AllocationError::Overflow {
                partial: Vec::new(),
                highest: u64::MAX,
                highest_file: None,
                cap: 9999,
                numbers_held: 0,
            })
        );
    }

    fn exhausted(scheme: &WorkItemIdScheme, filenames: &[String]) -> String {
        let error = allocate(scheme, None, 1, filenames).unwrap_err();
        error.message(scheme.pattern(), Path::new("meta/work"))
    }

    #[test]
    fn an_exhausted_number_space_names_the_item_holding_the_cap() {
        let message = exhausted(
            &numeric(),
            &names(&["0001-first.md", "9999-a-slip\u{1b}.md"]),
        );

        assert!(message.starts_with("E_PATTERN_OVERFLOW: "), "{message}");
        assert!(message.contains("number space exhausted"), "{message}");
        assert!(
            message.contains("meta/work/9999-a-slipU+001B.md holds 9999"),
            "{message}"
        );
        assert!(message.contains("the cap of pattern '{number:04d}'"));
        assert!(message.contains("a free lower ID"), "{message}");
        assert!(message.contains("widen the pattern"), "{message}");
    }

    #[test]
    fn a_full_sequence_offers_no_lower_id() {
        let filenames: Vec<String> = (1..=99)
            .map(|number| format!("{number:02}-item.md"))
            .collect();

        let message = exhausted(&scheme("{number:02d}", None), &filenames);

        assert!(message.contains("number space exhausted"), "{message}");
        assert!(!message.contains("lower"), "{message}");
        assert!(message.contains("Archive completed work items"));
        assert!(message.contains("widen the pattern"), "{message}");
    }

    fn overflowed(
        scheme: &WorkItemIdScheme,
        count: u32,
        filenames: &[String],
    ) -> String {
        let error = allocate(scheme, None, count, filenames).unwrap_err();
        error.message(scheme.pattern(), Path::new("meta/work"))
    }

    #[test]
    fn a_request_crossing_the_cap_names_no_item_as_holding_it() {
        let message = overflowed(&numeric(), 3, &names(&["9997-x.md"]));

        assert_eq!(
            message,
            "E_PATTERN_OVERFLOW: number space exhausted (highest=9997, \
             cap=9999). Archive completed work items or widen the pattern."
        );
    }

    #[test]
    fn a_request_beyond_the_cap_of_an_empty_directory_names_no_file() {
        let message = overflowed(&scheme("{number:02d}", None), 100, &[]);

        assert_eq!(
            message,
            "E_PATTERN_OVERFLOW: number space exhausted (highest=0, \
             cap=99). Archive completed work items or widen the pattern."
        );
    }

    #[test]
    fn a_stray_out_of_width_file_keeps_its_message() {
        let message = exhausted(&numeric(), &names(&["19999-stray.md"]));

        assert_eq!(
            message,
            "E_PATTERN_OVERFLOW: out-of-width file '19999-stray.md' has \
             number 19999 exceeding the pattern '{number:04d}' cap of 9999. \
             Rename the stray file or widen the pattern."
        );
    }

    #[test]
    fn every_other_allocation_error_renders_its_code() {
        let keyed = pattern("{key}-{number:04d}");
        let dir = Path::new("meta/work");
        let cases = [
            (AllocationError::MissingKey, "E_PATTERN_MISSING_KEY: "),
            (AllocationError::ProjectUnused, "E_PATTERN_KEY_UNUSED: "),
            (
                AllocationError::InvalidProject(KeyError::BadKeyValue(
                    "1X".to_owned(),
                )),
                "E_WORK_KEY_INVALID: --project '1X': ",
            ),
        ];
        for (error, code) in cases {
            let message = error.message(&keyed, dir);
            assert!(message.starts_with(code), "{message}");
        }
    }
}
