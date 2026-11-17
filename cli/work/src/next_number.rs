//! Next-sequential-ID allocation.

use std::collections::BTreeSet;
use std::path::Path;

use crate::id_pattern::escaped;
use crate::id_pattern::HeldPrefix;
use crate::id_pattern::IdKey;
use crate::id_pattern::IdParts;
use crate::id_pattern::IdPattern;
use crate::id_pattern::KeyError;
use crate::id_pattern::MissingKey;
use crate::id_scheme::IdScheme;
use crate::identity::ItemIdentity;
use crate::work_item_files::identity_of;
use crate::work_item_files::is_md_filename;
use crate::work_item_files::WorkItemListing;
use crate::work_item_files::DRAFTS_DIRECTORY;

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
    refuse_project_without_key_token(pattern, project)?;
    if !pattern.references_key() {
        return Ok(None);
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

/// Refuses a `--project` the pattern has no key token to render.
///
/// # Errors
///
/// [`AllocationError::ProjectUnused`] when `project` is given to a pattern
/// with no key token.
pub fn refuse_project_without_key_token(
    pattern: &IdPattern,
    project: Option<&str>,
) -> Result<(), AllocationError> {
    if project.is_some() && !pattern.references_key() {
        Err(AllocationError::ProjectUnused)
    } else {
        Ok(())
    }
}

/// Allocates `count` sequential IDs above the highest number `key` holds.
///
/// A number is held in `listing`, ignoring the key's letter case, in a
/// file's name or in its `id`, an alias or its `external_id`, so an
/// allocated ID is never one the taken rule finds held. An `external_id`
/// naming a tracker issue holds its number without lifting the sequence.
///
/// A bare number counts under the scheme's own key.
///
/// # Errors
///
/// [`AllocationError::MissingKey`] when the pattern has a key token and
/// `key` is `None`; [`AllocationError::Overflow`] when fewer than `count`
/// free numbers lie above `highest` within the pattern's numeric cap — the
/// error still carries every ID that fit before the boundary.
pub fn allocate(
    scheme: &IdScheme,
    key: Option<&IdKey>,
    count: u32,
    listing: &WorkItemListing,
) -> Result<Vec<String>, AllocationError> {
    let pattern = scheme.pattern();
    if pattern.references_key() && key.is_none() {
        return Err(AllocationError::MissingKey);
    }
    let held: Vec<(u64, String)> = held_numbers(scheme, listing)
        .into_iter()
        .filter(|(parts, _)| in_sequence_of(parts, key))
        .map(|(parts, holder)| (parts.number(), holder))
        .collect();
    let (highest, highest_file) = held
        .iter()
        .max_by_key(|(number, holder)| (*number, std::cmp::Reverse(holder)))
        .map_or((0, None), |(number, holder)| {
            (*number, Some(holder.clone()))
        });
    let numbers_held = held
        .iter()
        .map(|(number, _)| *number)
        .collect::<BTreeSet<_>>()
        .len();
    let identities: Vec<ItemIdentity> =
        listing.files.iter().filter_map(identity_of).collect();
    let linked = linked_numbers_under(scheme, key, &identities);

    allocate_above(pattern, key, highest, count, &linked).map_err(|error| {
        match error {
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
        }
    })
}

/// Every number a listed file holds, with the file as the work directory
/// names it.
fn held_numbers(
    scheme: &IdScheme,
    listing: &WorkItemListing,
) -> Vec<(IdParts, String)> {
    let paths = listing
        .files
        .iter()
        .map(|file| (file.path.as_path(), Some(file)))
        .chain(
            listing
                .unreadable
                .iter()
                .map(|file| (file.path.as_path(), None)),
        );
    let mut held = Vec::new();
    for (path, file) in paths {
        let holder = holder_name(path);
        let named = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .filter(|name| is_md_filename(name))
            .and_then(|name| scheme.read_held_prefix(&name))
            .map(HeldPrefix::into_parts);
        let fields = file.and_then(identity_of).map_or_else(Vec::new, |item| {
            let local_external_id = item
                .external_id
                .filter(|value| !scheme.names_a_tracker_issue(value));
            std::iter::once(item.id)
                .chain(item.aliases)
                .chain(local_external_id)
                .filter_map(|value| scheme.read_held(&value))
                .collect()
        });
        held.extend(
            named
                .into_iter()
                .chain(fields)
                .map(|parts| (parts, holder.clone())),
        );
    }
    held
}

fn in_sequence_of(parts: &IdParts, key: Option<&IdKey>) -> bool {
    match (parts.key(), key) {
        (Some(held), Some(key)) => held.eq_ignore_ascii_case(key.as_str()),
        (held, key) => held.is_none() && key.is_none(),
    }
}

/// Every number under `key` that an `external_id` naming a tracker issue
/// holds: it takes that number from a new item without lifting the
/// sequence past it.
#[must_use]
pub fn linked_numbers_under(
    scheme: &IdScheme,
    key: Option<&IdKey>,
    items: &[ItemIdentity],
) -> BTreeSet<u64> {
    items
        .iter()
        .filter_map(|item| item.external_id.as_deref())
        .filter(|value| scheme.names_a_tracker_issue(value))
        .filter_map(|value| scheme.read_held(value))
        .filter(|parts| in_sequence_of(parts, key))
        .map(|parts| parts.number())
        .collect()
}

/// The first `count` numbers above `floor` that `linked` leaves free,
/// within `pattern`'s cap; fewer when the cap comes first.
#[must_use]
pub fn free_numbers_above(
    pattern: &IdPattern,
    floor: u64,
    count: usize,
    linked: &BTreeSet<u64>,
) -> Vec<u64> {
    let cap = pattern.highest_number();
    std::iter::successors(floor.checked_add(1), |n| n.checked_add(1))
        .take_while(|&number| number <= cap)
        .filter(|number| !linked.contains(number))
        .take(count)
        .collect()
}

fn holder_name(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let in_drafts = path
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|parent| parent == DRAFTS_DIRECTORY);
    if in_drafts {
        format!("{DRAFTS_DIRECTORY}/{name}")
    } else {
        name
    }
}

fn allocate_above(
    pattern: &IdPattern,
    key: Option<&IdKey>,
    floor: u64,
    count: u32,
    linked: &BTreeSet<u64>,
) -> Result<Vec<String>, AllocationError> {
    let wanted = usize::try_from(count).unwrap_or(usize::MAX);
    let free: Vec<String> = free_numbers_above(pattern, floor, wanted, linked)
        .into_iter()
        .map(|number| pattern.render(key, number))
        .collect::<Result<_, _>>()?;
    if free.len() < wanted {
        return Err(AllocationError::Overflow {
            partial: free,
            highest: floor,
            highest_file: None,
            cap: pattern.highest_number(),
            numbers_held: 0,
        });
    }
    Ok(free)
}

#[cfg(test)]
// DSL pattern strings such as "{key}" are literal token markers, not format
// args.
#[allow(clippy::unwrap_used, clippy::literal_string_with_formatting_args)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use crate::work_item_files::ReadFailure;
    use crate::work_item_files::UnreadableFile;
    use crate::work_item_files::WorkItemFile;
    use crate::work_item_files::WorkItemListing;
    use std::path::Path;

    use super::refuse_project_without_key_token;
    use super::{allocate, allocate_above, allocation_key, AllocationError};
    use crate::id_pattern::{IdKey, IdPattern, KeyError};
    use crate::id_scheme::IdScheme;

    fn pattern(text: &str) -> IdPattern {
        IdPattern::parse(text).unwrap()
    }

    fn key(text: &str) -> IdKey {
        IdKey::parse(text).unwrap()
    }

    fn scheme(text: &str, configured: Option<&str>) -> IdScheme {
        IdScheme::under_any_tracker(pattern(text), configured.map(key))
    }

    fn numeric() -> IdScheme {
        IdScheme::numeric()
    }

    fn names(filenames: &[&str]) -> WorkItemListing {
        WorkItemListing {
            files: filenames
                .iter()
                .map(|name| WorkItemFile {
                    path: PathBuf::from("/work").join(name),
                    content: String::new(),
                })
                .collect(),
            unreadable: Vec::new(),
        }
    }

    fn item(path: &str, frontmatter: &str) -> WorkItemFile {
        WorkItemFile {
            path: PathBuf::from("/work").join(path),
            content: format!("---\n{frontmatter}\n---\n"),
        }
    }

    #[test]
    fn allocation_steps_over_every_number_an_item_holds() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let listing = WorkItemListing {
            files: vec![
                item("PP-0001-a.md", "id: \"PP-0001\"\naliases: [\"PP-0040\"]"),
                item(
                    "PP-0002-b.md",
                    "id: \"PP-0002\"\nexternal_id: \"PP-0050\"",
                ),
                item(
                    "drafts/draft-k7mq3x-c.md",
                    "id: \"draft-k7mq3x\"\naliases: [\"PP-0060\"]",
                ),
            ],
            unreadable: vec![UnreadableFile {
                path: PathBuf::from("/work/PP-0070-d.md"),
                reason: ReadFailure::Undecodable,
            }],
        };
        assert_eq!(
            allocate(&keyed, Some(&key("PP")), 1, &listing),
            Ok(vec!["PP-0071".to_owned()])
        );
    }

    #[test]
    fn a_tracker_key_held_as_external_id_does_not_lift_the_sequence() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let listing = WorkItemListing {
            files: vec![item(
                "PP-0001-a.md",
                "id: \"PP-0001\"\nexternal_id: \"PP-905\"",
            )],
            unreadable: Vec::new(),
        };
        assert_eq!(
            allocate(&keyed, Some(&key("PP")), 1, &listing),
            Ok(vec!["PP-0002".to_owned()])
        );
    }

    #[test]
    fn allocation_skips_a_number_a_tracker_key_holds_as_external_id() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let listing = WorkItemListing {
            files: vec![item(
                "PP-0001-a.md",
                "id: \"PP-0001\"\nexternal_id: \"PP-2\"",
            )],
            unreadable: Vec::new(),
        };
        assert_eq!(
            allocate(&keyed, Some(&key("PP")), 2, &listing),
            Ok(vec!["PP-0003".to_owned(), "PP-0004".to_owned()])
        );
    }

    #[test]
    fn allocation_skips_only_numbers_linked_under_the_allocating_key() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        let listing = WorkItemListing {
            files: vec![item(
                "PP-0001-a.md",
                "id: \"PP-0001\"\nexternal_id: \"XY-2\"",
            )],
            unreadable: Vec::new(),
        };
        assert_eq!(
            allocate(&keyed, Some(&key("PP")), 2, &listing),
            Ok(vec!["PP-0002".to_owned(), "PP-0003".to_owned()])
        );
    }

    #[test]
    fn skipping_a_tracker_key_past_the_cap_overflows() {
        let keyed = scheme("{key}-{number:02d}", Some("PP"));
        let listing = WorkItemListing {
            files: vec![item(
                "PP-98-a.md",
                "id: \"PP-98\"\nexternal_id: \"PP-99\"",
            )],
            unreadable: Vec::new(),
        };
        let result = allocate(&keyed, Some(&key("PP")), 1, &listing);
        assert!(
            matches!(
                &result,
                Err(AllocationError::Overflow { partial, cap: 99, .. })
                    if partial.is_empty()
            ),
            "{result:?}"
        );
    }

    #[test]
    fn overflow_names_a_draft_holder_under_the_drafts_directory() {
        let listing = WorkItemListing {
            files: vec![item(
                "drafts/draft-k7mq3x-c.md",
                "id: \"draft-k7mq3x\"\naliases: [\"9999\"]",
            )],
            unreadable: Vec::new(),
        };
        let result = allocate(&numeric(), None, 1, &listing);
        assert!(
            matches!(
                &result,
                Err(AllocationError::Overflow { highest_file, .. })
                    if highest_file.as_deref()
                        == Some("drafts/draft-k7mq3x-c.md")
            ),
            "{result:?}"
        );
    }

    #[test]
    fn a_project_is_refused_only_without_a_key_token() {
        for keyless in ["{number:04d}", "{tracker}"] {
            assert_eq!(
                refuse_project_without_key_token(&pattern(keyless), Some("PP")),
                Err(AllocationError::ProjectUnused),
                "{keyless}"
            );
            assert_eq!(
                refuse_project_without_key_token(&pattern(keyless), None),
                Ok(())
            );
        }
        assert_eq!(
            refuse_project_without_key_token(
                &pattern("{key}-{number:04d}"),
                Some("PP")
            ),
            Ok(())
        );
    }

    #[test]
    fn allocates_the_next_sequential_id_from_an_empty_corpus() {
        let result = allocate(&numeric(), None, 1, &names(&[]));
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
            allocate(
                &scheme("{project}-{number:04d}", None),
                None,
                1,
                &names(&[])
            ),
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
    fn overflow_names_the_first_holder_of_the_cap_in_filename_order() {
        let keyed = scheme("{key}-{number:04d}", Some("PP"));
        for filenames in [
            ["pp-9999-b.md", "PP-9999-a.md"],
            ["PP-9999-a.md", "pp-9999-b.md"],
        ] {
            let result =
                allocate(&keyed, Some(&key("PP")), 1, &names(&filenames));
            assert!(
                matches!(
                    &result,
                    Err(AllocationError::Overflow { highest_file, .. })
                        if highest_file.as_deref() == Some("PP-9999-a.md")
                ),
                "{filenames:?}: {result:?}"
            );
        }
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
                2,
                &BTreeSet::new()
            ),
            Ok(vec!["ACC-0042".to_owned(), "ACC-0043".to_owned()])
        );
    }

    #[test]
    fn allocation_above_the_largest_floor_overflows_with_nothing_allocated() {
        assert_eq!(
            allocate_above(
                &pattern("{number:04d}"),
                None,
                u64::MAX,
                2,
                &BTreeSet::new()
            ),
            Err(AllocationError::Overflow {
                partial: Vec::new(),
                highest: u64::MAX,
                highest_file: None,
                cap: 9999,
                numbers_held: 0,
            })
        );
    }

    fn exhausted(scheme: &IdScheme, filenames: &WorkItemListing) -> String {
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

        let filenames: Vec<&str> =
            filenames.iter().map(String::as_str).collect();
        let message =
            exhausted(&scheme("{number:02d}", None), &names(&filenames));

        assert!(message.contains("number space exhausted"), "{message}");
        assert!(!message.contains("lower"), "{message}");
        assert!(message.contains("Archive completed work items"));
        assert!(message.contains("widen the pattern"), "{message}");
    }

    fn overflowed(
        scheme: &IdScheme,
        count: u32,
        filenames: &WorkItemListing,
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
        let message =
            overflowed(&scheme("{number:02d}", None), 100, &names(&[]));

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
