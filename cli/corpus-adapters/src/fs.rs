//! The real-filesystem implementations of `corpus`'s `scan` and `resolve`
//! ports.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

use corpus::resolve::DirectoryLister;
use corpus::resolve::TypeShape;
use corpus::scan::CorpusWalker;
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

impl CorpusWalker for RealFs {
    fn walk_markdown(
        &self,
        roots: &[PathBuf],
    ) -> Result<Vec<PathBuf>, kernel::Error> {
        let mut files = Vec::new();
        for root in roots {
            walk_markdown_into(root, &mut files)?;
        }
        Ok(files)
    }
}

fn walk_markdown_into(
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), kernel::Error> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(kernel::Error::Failed(format!(
                "reading {}: {error}",
                dir.display()
            )))
        }
    };
    let mut children: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            kernel::Error::Failed(format!("reading {}: {error}", dir.display()))
        })?;
        children.push(entry.path());
    }
    children.sort();
    for path in children {
        let file_type = fs::symlink_metadata(&path)
            .map_err(|error| {
                kernel::Error::Failed(format!(
                    "reading {}: {error}",
                    path.display()
                ))
            })?
            .file_type();
        if file_type.is_dir() {
            walk_markdown_into(&path, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|ext| ext.to_str()) == Some("md")
        {
            files.push(path);
        }
    }
    Ok(())
}

/// The real-filesystem adapter for [`DirectoryLister`]: lists a type
/// directory's resolvable entries — `.md` files for a flat type, immediate
/// subdirectories for a nested-manifest type.
pub struct TypeDirectoryLister {
    dir: PathBuf,
    shape: TypeShape,
}

impl TypeDirectoryLister {
    #[must_use]
    pub const fn new(dir: PathBuf, shape: TypeShape) -> Self {
        Self { dir, shape }
    }
}

impl DirectoryLister for TypeDirectoryLister {
    fn entries(&self) -> Vec<String> {
        let Ok(reader) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        reader
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_type = entry.file_type().ok()?;
                let name = entry.file_name().to_str()?.to_owned();
                let keep = match self.shape {
                    TypeShape::Flat => {
                        file_type.is_file() && is_markdown(&name)
                    }
                    TypeShape::NestedManifest => file_type.is_dir(),
                };
                keep.then_some(name)
            })
            .collect()
    }
}

fn is_markdown(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use corpus::resolve::DirectoryLister;
    use corpus::resolve::TypeShape;
    use corpus::scan::CorpusWalker;
    use corpus::scan::DirReader;
    use corpus::scan::FileReader;

    use super::RealFs;
    use super::TypeDirectoryLister;

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

    #[test]
    fn walk_markdown_finds_files_recursively_and_ignores_non_markdown(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let nested = dir.path().join("nested");
        std::fs::create_dir_all(&nested)?;
        std::fs::write(dir.path().join("a.md"), "")?;
        std::fs::write(dir.path().join("b.txt"), "")?;
        std::fs::write(nested.join("c.md"), "")?;

        let mut found = RealFs.walk_markdown(&[dir.path().to_path_buf()])?;
        found.sort();
        assert_eq!(found, vec![dir.path().join("a.md"), nested.join("c.md")]);
        Ok(())
    }

    #[test]
    fn walk_markdown_is_silent_over_a_missing_root(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let missing = PathBuf::from("/does/not/exist/at/all");
        assert_eq!(RealFs.walk_markdown(&[missing])?, Vec::<PathBuf>::new());
        Ok(())
    }

    #[test]
    fn a_flat_type_lists_markdown_files_only(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("a.md"), "")?;
        std::fs::write(dir.path().join("b.txt"), "")?;
        std::fs::create_dir(dir.path().join("sub"))?;

        let lister =
            TypeDirectoryLister::new(dir.path().to_path_buf(), TypeShape::Flat);
        let mut entries = lister.entries();
        entries.sort();

        assert_eq!(entries, vec!["a.md".to_owned()]);
        Ok(())
    }

    #[test]
    fn a_flat_type_matches_the_markdown_extension_case_insensitively(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("shouty.MD"), "")?;

        let lister =
            TypeDirectoryLister::new(dir.path().to_path_buf(), TypeShape::Flat);

        assert_eq!(lister.entries(), vec!["shouty.MD".to_owned()]);
        Ok(())
    }

    #[test]
    fn a_nested_manifest_type_lists_subdirectories_only(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("set-one"))?;
        std::fs::create_dir(dir.path().join("set-two"))?;
        std::fs::write(dir.path().join("stray.md"), "")?;

        let lister = TypeDirectoryLister::new(
            dir.path().to_path_buf(),
            TypeShape::NestedManifest,
        );
        let mut entries = lister.entries();
        entries.sort();

        assert_eq!(entries, vec!["set-one".to_owned(), "set-two".to_owned()]);
        Ok(())
    }

    #[test]
    fn a_missing_directory_lists_no_entries() {
        let lister = TypeDirectoryLister::new(
            PathBuf::from("/does/not/exist"),
            TypeShape::Flat,
        );

        assert!(lister.entries().is_empty());
    }
}
