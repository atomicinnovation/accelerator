//! The real-filesystem implementation of `corpus::scan`'s ports.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use corpus::scan::DirReader;
use corpus::scan::FileReader;

/// The real-filesystem adapter for [`DirReader`] and [`FileReader`],
/// composed once at a `corpus-cli` command's dispatch site and injected into
/// every command.
pub struct RealFs;

impl DirReader for RealFs {
    fn list(&self, dir: &Path) -> Result<Option<Vec<String>>, kernel::Error> {
        match fs::read_dir(dir) {
            Ok(entries) => {
                let mut names = Vec::new();
                for entry in entries {
                    let entry = entry.map_err(|error| {
                        kernel::Error::Failed(format!(
                            "reading {}: {error}",
                            dir.display()
                        ))
                    })?;
                    names
                        .push(entry.file_name().to_string_lossy().into_owned());
                }
                Ok(Some(names))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(kernel::Error::Failed(format!(
                "reading {}: {error}",
                dir.display()
            ))),
        }
    }
}

impl FileReader for RealFs {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        if !path.is_file() {
            return Ok(None);
        }
        fs::read_to_string(path).map(Some).map_err(|error| {
            kernel::Error::Failed(format!(
                "reading {}: {error}",
                path.display()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use corpus::scan::DirReader;
    use corpus::scan::FileReader;

    use super::RealFs;

    #[test]
    fn list_is_none_for_a_missing_directory() -> Result<(), kernel::Error> {
        assert_eq!(RealFs.list(Path::new("/does/not/exist"))?, None);
        Ok(())
    }

    #[test]
    fn read_is_none_for_a_missing_file() -> Result<(), kernel::Error> {
        assert_eq!(RealFs.read(Path::new("/does/not/exist.md"))?, None);
        Ok(())
    }

    #[test]
    fn read_is_none_for_a_directory() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        assert_eq!(RealFs.read(dir.path())?, None);
        Ok(())
    }

    #[test]
    fn list_and_read_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.md"), "hello")?;
        let names = RealFs.list(dir.path())?.ok_or("expected Some")?;
        assert_eq!(names, vec!["a.md".to_owned()]);
        let content = RealFs.read(&dir.path().join("a.md"))?;
        assert_eq!(content, Some("hello".to_owned()));
        Ok(())
    }
}
