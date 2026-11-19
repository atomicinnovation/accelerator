//! What a `MigrationContext` answers for the capabilities it leaves to the
//! trait's defaults.

use std::path::Path;

use migrate::ports::CorpusIndex;
use migrate::ports::DocTypeDir;
use migrate::ports::MigrationContext;
use migrate::ports::MigrationError;

struct MinimalContext;

struct NoIndex;

impl CorpusIndex for NoIndex {
    fn target_exists(&self, _target_type: &str, _target_id: &str) -> bool {
        false
    }
}

impl MigrationContext for MinimalContext {
    fn doc_type_dirs(&self) -> Vec<DocTypeDir> {
        Vec::new()
    }

    fn corpus_index(&self) -> &dyn CorpusIndex {
        &NoIndex
    }

    fn write(
        &self,
        _path: &Path,
        _content: &str,
    ) -> Result<(), MigrationError> {
        Ok(())
    }
}

#[test]
fn a_lossy_read_defaults_to_a_missing_file() {
    assert_eq!(MinimalContext.read(Path::new("meta/x.md")), Ok(None));
}

#[test]
fn a_strict_read_refuses_rather_than_reporting_a_missing_file() {
    assert_eq!(
        MinimalContext.read_text(Path::new("meta/x.md")),
        Err(MigrationError::new(
            "read_text is not implemented by this context"
        ))
    );
}
