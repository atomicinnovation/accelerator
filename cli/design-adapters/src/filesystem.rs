//! Reading the files the ported subcommands act on.

use std::path::Path;

/// What a path location turned out to be on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryCheck {
    Directory,
    /// Absent, or present but not a directory. The shell conflates the two in
    /// one message, and the port preserves that.
    NotADirectory,
}

/// Whether `path` is a directory.
#[must_use]
pub fn check_directory(path: &Path) -> DirectoryCheck {
    if path.is_dir() {
        DirectoryCheck::Directory
    } else {
        DirectoryCheck::NotADirectory
    }
}

/// Reads a document to audit or scan.
///
/// # Errors
///
/// A [`kernel::Error::Refusal`] when the path names nothing readable — a
/// malformed invocation rather than a verdict, since the argument cannot be
/// interpreted as a file to read at all.
pub fn read_document(path: &Path) -> Result<String, kernel::Error> {
    if !path.is_file() {
        return Err(kernel::Error::Refusal(format!(
            "file not found: {}",
            path.display()
        )));
    }
    std::fs::read_to_string(path).map_err(|error| {
        kernel::Error::Refusal(format!(
            "could not read {}: {error}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::check_directory;
    use super::read_document;
    use super::DirectoryCheck;

    type TestError = Box<dyn std::error::Error>;

    #[test]
    fn a_directory_is_recognised() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        assert_eq!(check_directory(work.path()), DirectoryCheck::Directory);
        Ok(())
    }

    #[test]
    fn an_absent_path_and_a_plain_file_are_both_not_a_directory(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let file = work.path().join("a.md");
        fs::write(&file, "x")?;
        assert_eq!(check_directory(&file), DirectoryCheck::NotADirectory);
        assert_eq!(
            check_directory(&work.path().join("absent")),
            DirectoryCheck::NotADirectory
        );
        Ok(())
    }

    #[test]
    fn a_document_reads_back_verbatim() -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let file = work.path().join("a.md");
        fs::write(&file, "## Alpha\n\nprose\n")?;
        assert_eq!(read_document(&file)?, "## Alpha\n\nprose\n");
        Ok(())
    }

    /// A path that names no readable file is a malformed invocation, not a
    /// verdict — so it refuses, and the command layer maps that to exit 2.
    #[test]
    fn an_unreadable_path_refuses_rather_than_returning_a_verdict(
    ) -> Result<(), TestError> {
        let work = tempfile::tempdir()?;
        let Err(error) = read_document(&work.path().join("absent.md")) else {
            return Err("expected a refusal".into());
        };
        assert!(matches!(error, kernel::Error::Refusal(_)));
        assert!(error.to_string().contains("file not found"));
        Ok(())
    }
}
