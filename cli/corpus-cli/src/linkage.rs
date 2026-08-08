//! The `linkage` command layer: decision logic only, generic over the
//! injected filesystem port — no `config_adapters::compose` call and no
//! direct `std::fs` access lives here.

use std::path::Path;
use std::path::PathBuf;

use corpus::scan::FileReader;
use corpus::DocTypeKey;

use crate::outcome::Outcome;

/// Runs `linkage extract`.
///
/// # Errors
///
/// A [`kernel::Error`] when the file cannot be read.
pub fn run_extract<F: FileReader>(
    file: &Path,
    source_type_override: Option<&str>,
    table: &[(DocTypeKey, PathBuf)],
    file_reader: &F,
) -> Result<Outcome, kernel::Error> {
    let Some(content) = file_reader.read(file)? else {
        return Err(kernel::Error::Failed(format!(
            "Error: File not found: {}",
            file.display()
        )));
    };
    let path = file.to_string_lossy();
    let source_type = source_type_override
        .or_else(|| corpus::linkage::type_from_path(&path, table))
        .unwrap_or("unknown");
    let mut stdout = String::new();
    for record in corpus::linkage::parse_document(source_type, &content, table)
    {
        stdout.push_str(&record.source_type);
        stdout.push('\t');
        stdout.push_str(&record.key);
        stdout.push('\t');
        stdout.push_str(&record.target_ref);
        stdout.push('\t');
        stdout.push_str(&record.anchor);
        stdout.push('\t');
        stdout.push_str(record.band.as_str());
        stdout.push('\n');
    }
    Ok(Outcome {
        stdout,
        stderr: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use corpus::scan::FileReader;
    use corpus::DocTypeKey;

    use super::run_extract;

    #[derive(Default)]
    struct StubFileReader {
        content: Option<String>,
    }

    impl FileReader for StubFileReader {
        fn read(&self, _path: &Path) -> Result<Option<String>, kernel::Error> {
            Ok(self.content.clone())
        }
    }

    fn table() -> Vec<(DocTypeKey, std::path::PathBuf)> {
        vec![(DocTypeKey::WorkItems, std::path::PathBuf::from("meta/work"))]
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let reader = StubFileReader::default();
        assert!(run_extract(
            Path::new("meta/work/0001.md"),
            None,
            &table(),
            &reader
        )
        .is_err());
    }

    #[test]
    fn infers_the_source_type_from_the_path_when_no_override_is_given(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = StubFileReader {
            content: Some("## Dependencies\n- Blocks: 0061\n".to_owned()),
        };
        let outcome = run_extract(
            Path::new("meta/work/0050-deps.md"),
            None,
            &table(),
            &reader,
        )?;
        assert_eq!(
            outcome.stdout,
            "work-item\tblocks\twork-item:0061\tbody:dependencies#0\tresolved\n"
        );
        Ok(())
    }

    #[test]
    fn an_explicit_source_type_overrides_path_inference(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = StubFileReader {
            content: Some("## Dependencies\n- Blocks: 0061\n".to_owned()),
        };
        let outcome = run_extract(
            Path::new("meta/work/0050-deps.md"),
            Some("plan"),
            &table(),
            &reader,
        )?;
        assert!(outcome.stdout.starts_with("plan\t"));
        Ok(())
    }

    #[test]
    fn an_unresolvable_path_falls_back_to_unknown(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = StubFileReader {
            content: Some("## Dependencies\n- Blocks: 0061\n".to_owned()),
        };
        let outcome =
            run_extract(Path::new("elsewhere/x.md"), None, &table(), &reader)?;
        assert!(outcome.stdout.starts_with("unknown\t"));
        Ok(())
    }

    #[test]
    fn no_records_produces_empty_stdout(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let reader = StubFileReader {
            content: Some("# Just a heading\n".to_owned()),
        };
        let outcome = run_extract(
            Path::new("meta/work/0001.md"),
            None,
            &table(),
            &reader,
        )?;
        assert_eq!(outcome.stdout, String::new());
        Ok(())
    }
}
