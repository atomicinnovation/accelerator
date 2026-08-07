//! Next-sequential-ID allocation. Port of `work-item-next-number.sh:94-140`.
//!
//! Formatting reuses `corpus::WorkItemIdScheme::canonicalise_id` (via a
//! project-overridden clone of `scheme`) rather than
//! `corpus_adapters::compile_format_string`, and the numeric cap is
//! computed with a small local, dependency-free width parse rather than
//! `corpus_adapters::pattern_max_number` — `work`'s own import-restriction
//! rule keeps `corpus_adapters` (which needs `regex`) out of this crate,
//! the same trade-off `resolve.rs` already makes.

// DSL pattern strings such as "{project}" are literal token markers, not
// format args.
#![allow(clippy::literal_string_with_formatting_args)]

use corpus::IdScanner;
use corpus::WorkItemIdScheme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationError {
    MissingProject,
    ProjectUnused,
    Overflow {
        partial: Vec<String>,
        highest: u64,
        highest_file: Option<String>,
        cap: u64,
    },
}

fn explicit_width(pattern: &str) -> Option<usize> {
    let idx = pattern.find("{number:0")?;
    let rest = &pattern[idx + "{number:0".len()..];
    let digit_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let digits = &rest[..digit_end];
    if digits.is_empty() || digits.starts_with('0') {
        return None;
    }
    if rest[digit_end..].starts_with("d}") {
        digits.parse().ok()
    } else {
        None
    }
}

fn pattern_cap(pattern: &str) -> u64 {
    let width = explicit_width(pattern).unwrap_or(4);
    10u64
        .saturating_pow(u32::try_from(width).unwrap_or(u32::MAX))
        .saturating_sub(1)
}

fn is_md_filename(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        && name.contains('.')
}

/// Allocates `count` sequential IDs following the highest number already
/// present in `filenames`.
///
/// `project` is the already-resolved `--project`-flag-or-config-default
/// value (`None`/empty when unused), not necessarily
/// `scheme.default_project_code` — a caller-supplied `--project` overrides
/// the scheme's own configured default for this one call.
///
/// # Errors
///
/// [`AllocationError::MissingProject`] when the pattern needs `{project}`
/// but none was supplied; [`AllocationError::ProjectUnused`] when a project
/// was supplied but the pattern has no `{project}` token;
/// [`AllocationError::Overflow`] when `highest + count` would exceed the
/// pattern's numeric cap — the error still carries every ID that fit before
/// the boundary.
pub fn allocate(
    scheme: &WorkItemIdScheme,
    project: Option<&str>,
    count: u32,
    filenames: &[String],
    scanner: &dyn IdScanner,
) -> Result<Vec<String>, AllocationError> {
    let pattern_has_project = scheme.id_pattern.contains("{project}");
    let project_given = project.is_some_and(|p| !p.is_empty());

    if pattern_has_project && !project_given {
        return Err(AllocationError::MissingProject);
    }
    if !pattern_has_project && project_given {
        return Err(AllocationError::ProjectUnused);
    }

    let format_scheme = WorkItemIdScheme {
        id_pattern: scheme.id_pattern.clone(),
        default_project_code: project.map(str::to_owned),
    };
    let cap = pattern_cap(&scheme.id_pattern);

    let mut highest: u64 = 0;
    let mut highest_file: Option<String> = None;
    for filename in filenames {
        if !is_md_filename(filename) {
            continue;
        }
        if let Some(scan) = scanner.scan(filename) {
            if let Ok(number) = scan.digits.parse::<u64>() {
                if number > highest {
                    highest = number;
                    highest_file = Some(filename.clone());
                }
            }
        }
    }

    let format = |number: u64| {
        format_scheme
            .canonicalise_id(&number.to_string())
            .unwrap_or_else(|| number.to_string())
    };

    if highest.saturating_add(u64::from(count)) > cap {
        let mut partial = Vec::new();
        for i in 1..=u64::from(count) {
            let next = highest + i;
            if next > cap {
                break;
            }
            partial.push(format(next));
        }
        return Err(AllocationError::Overflow {
            partial,
            highest,
            highest_file,
            cap,
        });
    }

    Ok((1..=u64::from(count))
        .map(|i| format(highest + i))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{allocate, AllocationError};
    use corpus::{IdScan, IdScanner, WorkItemIdScheme};

    struct DigitPrefixScanner;

    impl IdScanner for DigitPrefixScanner {
        fn scan(&self, text: &str) -> Option<IdScan> {
            let digits: String =
                text.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() || !text[digits.len()..].starts_with('-') {
                return None;
            }
            let match_end = digits.len() + 1;
            Some(IdScan { digits, match_end })
        }
    }

    fn numeric() -> WorkItemIdScheme {
        WorkItemIdScheme {
            id_pattern: "{number:04d}".to_owned(),
            default_project_code: None,
        }
    }

    #[test]
    fn allocates_the_next_sequential_id_from_an_empty_corpus() {
        let result = allocate(&numeric(), None, 1, &[], &DigitPrefixScanner);
        assert_eq!(result, Ok(vec!["0001".to_owned()]));
    }

    #[test]
    fn allocates_after_the_highest_existing_number() {
        let filenames = vec!["0001-foo.md".to_owned()];
        let result =
            allocate(&numeric(), None, 1, &filenames, &DigitPrefixScanner);
        assert_eq!(result, Ok(vec!["0002".to_owned()]));
    }

    #[test]
    fn allocates_a_sequential_batch() {
        let filenames = vec!["0001-first.md".to_owned()];
        let result =
            allocate(&numeric(), None, 3, &filenames, &DigitPrefixScanner);
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
    fn missing_project_is_rejected() {
        let scheme = WorkItemIdScheme {
            id_pattern: "{project}-{number:04d}".to_owned(),
            default_project_code: None,
        };
        assert_eq!(
            allocate(&scheme, None, 1, &[], &DigitPrefixScanner),
            Err(AllocationError::MissingProject)
        );
    }

    #[test]
    fn project_given_to_a_project_less_pattern_is_rejected() {
        assert_eq!(
            allocate(&numeric(), Some("PROJ"), 1, &[], &DigitPrefixScanner),
            Err(AllocationError::ProjectUnused)
        );
    }

    #[test]
    fn overflow_from_a_stray_out_of_width_file() {
        let filenames = vec!["19999-stray.md".to_owned()];
        let result =
            allocate(&numeric(), None, 1, &filenames, &DigitPrefixScanner);
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
        let filenames = vec!["9999-last.md".to_owned()];
        let result =
            allocate(&numeric(), None, 3, &filenames, &DigitPrefixScanner);
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
        let filenames = vec!["9998-near.md".to_owned()];
        let result =
            allocate(&numeric(), None, 3, &filenames, &DigitPrefixScanner);
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
}
