//! The `adr` command layer: decision logic only, generic over the injected
//! filesystem ports — no `config_adapters::compose` call and no direct
//! `std::fs` access lives here.

use std::path::Path;

use corpus::scan::DirReader;
use corpus::scan::FileReader;

use crate::outcome::Outcome;

fn is_valid_count(count: &str) -> bool {
    let mut chars = count.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_digit() && first != '0')
        && chars.all(|c| c.is_ascii_digit())
}

fn is_adr_basename(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("ADR-") else {
        return false;
    };
    let mut chars = rest.chars();
    (0..4).all(|_| chars.next().is_some_and(|c| c.is_ascii_digit()))
}

fn invalid_count(count: &str) -> kernel::Error {
    kernel::Error::Failed(format!(
        "Error: --count requires a positive integer, got '{count}'"
    ))
}

fn render_numbers(numbers: impl IntoIterator<Item = String>) -> String {
    let mut stdout = String::new();
    for number in numbers {
        stdout.push_str(&number);
        stdout.push('\n');
    }
    stdout
}

/// Runs `adr next-number`.
///
/// # Errors
///
/// A [`kernel::Error`] when `count` is not a positive integer — including a
/// value that matches `^[1-9][0-9]*$` but overflows `u32` — or when
/// `dir_reader` fails. A missing decisions directory is a success path
/// carrying a warning, matching bash's `exit 0`.
pub fn run_next_number<D: DirReader>(
    count: &str,
    decisions_dir: &Path,
    dir_reader: &D,
) -> Result<Outcome, kernel::Error> {
    if !is_valid_count(count) {
        return Err(invalid_count(count));
    }
    let parsed: u32 = count.parse().map_err(|_| invalid_count(count))?;
    let Some(entries) = dir_reader.list(decisions_dir)? else {
        let stdout = render_numbers((1..=parsed).map(|n| format!("{n:04}")));
        return Ok(Outcome {
            stdout,
            stderr: format!(
                "Warning: decisions directory '{}' does not exist — \
                 defaulting to next number 0001\n",
                decisions_dir.display()
            ),
        });
    };
    let existing: Vec<&str> = entries
        .iter()
        .map(String::as_str)
        .filter(|name| is_adr_basename(name))
        .collect();
    let stdout = render_numbers(corpus::adr::next_numbers(&existing, parsed));
    Ok(Outcome {
        stdout,
        stderr: String::new(),
    })
}

/// Runs `adr read-status`.
///
/// # Errors
///
/// A [`kernel::Error`] when no file was given, the file cannot be read, or
/// its frontmatter carries no closed fence with a `status:` line — matching
/// bash's exit code 1 in each case.
pub fn run_read_status<F: FileReader>(
    file: Option<&Path>,
    file_reader: &F,
) -> Result<Outcome, kernel::Error> {
    let Some(file) = file else {
        return Err(kernel::Error::Failed(
            "Usage: adr-read-status.sh <adr-file-path>".to_owned(),
        ));
    };
    let Some(content) = file_reader.read(file)? else {
        return Err(kernel::Error::Failed(format!(
            "Error: File not found: {}",
            file.display()
        )));
    };
    corpus::adr::read_status(&content).map_or_else(
        || {
            let basename = file.file_name().map_or_else(
                || file.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            Err(kernel::Error::Failed(format!(
                "Error: No status field found in frontmatter of {basename}.\n\
                 Expected YAML frontmatter with 'status: \
                 proposed|accepted|rejected|superseded|deprecated' between \
                 --- delimiters."
            )))
        },
        |status| {
            Ok(Outcome {
                stdout: format!("{status}\n"),
                stderr: String::new(),
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use corpus::scan::DirReader;
    use corpus::scan::FileReader;

    use super::run_next_number;
    use super::run_read_status;
    use crate::outcome::Outcome;

    #[derive(Default)]
    struct StubDirReader {
        entries: Option<Vec<String>>,
        fails: bool,
    }

    impl DirReader for StubDirReader {
        fn list(
            &self,
            _dir: &Path,
        ) -> Result<Option<Vec<String>>, kernel::Error> {
            if self.fails {
                return Err(kernel::Error::Failed("boom".to_owned()));
            }
            Ok(self.entries.clone())
        }
    }

    #[derive(Default)]
    struct StubFileReader {
        content: Option<String>,
        fails: bool,
    }

    impl FileReader for StubFileReader {
        fn read(&self, _path: &Path) -> Result<Option<String>, kernel::Error> {
            if self.fails {
                return Err(kernel::Error::Failed("boom".to_owned()));
            }
            Ok(self.content.clone())
        }
    }

    #[test]
    fn next_number_rejects_a_non_positive_count() {
        let reader = StubDirReader::default();
        assert!(run_next_number("0", Path::new("/decisions"), &reader).is_err());
    }

    #[test]
    fn next_number_rejects_a_non_numeric_count() {
        let reader = StubDirReader::default();
        assert!(
            run_next_number("abc", Path::new("/decisions"), &reader).is_err()
        );
    }

    #[test]
    fn next_number_rejects_a_count_that_overflows_u32() {
        let reader = StubDirReader::default();
        assert!(run_next_number(
            "5000000000",
            Path::new("/decisions"),
            &reader
        )
        .is_err());
    }

    #[test]
    fn next_number_a_missing_directory_succeeds_with_a_warning(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = StubDirReader {
            entries: None,
            fails: false,
        };
        let outcome = run_next_number("2", Path::new("/decisions"), &reader)?;
        assert_eq!(outcome.stdout, "0001\n0002\n");
        assert!(outcome.stderr.contains("does not exist"));
        Ok(())
    }

    #[test]
    fn next_number_propagates_a_reader_failure() {
        let reader = StubDirReader {
            entries: None,
            fails: true,
        };
        assert!(run_next_number("1", Path::new("/decisions"), &reader).is_err());
    }

    #[test]
    fn next_number_filters_non_adr_entries(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = StubDirReader {
            entries: Some(vec![
                "ADR-0002-something.md".to_owned(),
                "README.md".to_owned(),
                "DRAFT-notes.md".to_owned(),
            ]),
            fails: false,
        };
        let outcome = run_next_number("1", Path::new("/decisions"), &reader)?;
        assert_eq!(
            outcome,
            Outcome {
                stdout: "0003\n".to_owned(),
                stderr: String::new(),
            }
        );
        Ok(())
    }

    #[test]
    fn read_status_with_no_file_is_an_error() {
        let reader = StubFileReader::default();
        assert!(run_read_status(None, &reader).is_err());
    }

    #[test]
    fn read_status_a_missing_file_is_an_error() {
        let reader = StubFileReader {
            content: None,
            fails: false,
        };
        assert!(run_read_status(Some(Path::new("/nope.md")), &reader).is_err());
    }

    #[test]
    fn read_status_propagates_a_reader_failure() {
        let reader = StubFileReader {
            content: None,
            fails: true,
        };
        assert!(run_read_status(Some(Path::new("/nope.md")), &reader).is_err());
    }

    #[test]
    fn read_status_a_file_with_no_status_key_is_an_error() {
        let reader = StubFileReader {
            content: Some("---\nadr_id: ADR-0001\n---\n".to_owned()),
            fails: false,
        };
        assert!(run_read_status(Some(Path::new("/a.md")), &reader).is_err());
    }

    #[test]
    fn read_status_a_valid_file_returns_the_status(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = StubFileReader {
            content: Some("---\nstatus: proposed\n---\n".to_owned()),
            fails: false,
        };
        let outcome = run_read_status(Some(Path::new("/a.md")), &reader)?;
        assert_eq!(
            outcome,
            Outcome {
                stdout: "proposed\n".to_owned(),
                stderr: String::new(),
            }
        );
        Ok(())
    }
}
