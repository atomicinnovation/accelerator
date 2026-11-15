//! Whether the work items under a directory can be trusted for their IDs.

use std::path::Path;

use work::corpus_trust::refuse_untrusted;
use work::corpus_trust::UntrustedCorpus;
use work::id_scheme::IdScheme;
use work::work_item_files::WorkItemFiles as _;

use crate::filesystem::FilesystemWorkItemFiles;

/// Refuses the work items under `work_dir` when their IDs cannot be
/// trusted. A directory that cannot be listed is left to the command's own
/// reading to report.
///
/// # Errors
///
/// [`UntrustedCorpus`] naming the first reason the corpus cannot be
/// trusted.
pub fn refuse_untrusted_under(
    scheme: &IdScheme,
    work_dir: &Path,
) -> Result<(), UntrustedCorpus> {
    let Ok(files) = FilesystemWorkItemFiles::new(work_dir).files() else {
        return Ok(());
    };
    refuse_untrusted(scheme, &files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::refuse_untrusted_under;
    use work::id_scheme::IdScheme;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_work_directory_that_cannot_be_listed_is_not_refused(
    ) -> Result<(), TestError> {
        let dir = tempfile::tempdir()?;

        assert_eq!(
            refuse_untrusted_under(
                &IdScheme::numeric(),
                &dir.path().join("missing")
            ),
            Ok(())
        );
        Ok(())
    }

    #[test]
    fn the_items_under_a_work_directory_are_checked() -> Result<(), TestError> {
        let dir = tempfile::tempdir()?;
        fs::write(
            dir.path().join("0001.md"),
            "---\nid: \"0001\"\ntitle: \"Slugless\"\n---\n",
        )?;

        let refusal =
            refuse_untrusted_under(&IdScheme::numeric(), dir.path()).err();

        assert_eq!(
            refusal.map(|refusal| refusal.code()),
            Some("E_WORK_ITEM_MALFORMED")
        );
        Ok(())
    }
}
