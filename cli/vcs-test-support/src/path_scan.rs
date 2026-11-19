//! Finding where a fixture tree records the directory it was built in.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

use walkdir::WalkDir;

use crate::Error;

/// Every regular file under `tree` whose bytes name `needle`, and every
/// symlink whose target does, in path order.
///
/// The needle matches as given and as its canonical path, so a needle spelled
/// through `/var` also catches a file naming `/private/var`.
///
/// # Errors
///
/// When `needle` is not UTF-8, or `tree` cannot be walked or read.
pub fn files_naming(tree: &Path, needle: &Path) -> Result<Vec<PathBuf>, Error> {
    let spellings = Spellings::of(needle)?;
    let mut naming = Vec::new();
    for entry in WalkDir::new(tree).sort_by_file_name() {
        let entry = entry.map_err(|error| Error::message(error.to_string()))?;
        let file_type = entry.file_type();
        let contents = if file_type.is_symlink() {
            fs::read_link(entry.path())?
                .into_os_string()
                .into_encoded_bytes()
        } else if file_type.is_file() {
            fs::read(entry.path())?
        } else {
            continue;
        };
        if spellings.are_named_in(&contents) {
            naming.push(entry.into_path());
        }
    }
    Ok(naming)
}

struct Spellings(Vec<String>);

impl Spellings {
    fn of(needle: &Path) -> Result<Self, Error> {
        let mut spellings = vec![utf8(needle)?];
        if let Ok(canonical) = needle.canonicalize() {
            let canonical = utf8(&canonical)?;
            if !spellings.contains(&canonical) {
                spellings.push(canonical);
            }
        }
        Ok(Self(spellings))
    }

    fn are_named_in(&self, contents: &[u8]) -> bool {
        self.0.iter().any(|spelling| {
            contents
                .windows(spelling.len())
                .any(|window| window == spelling.as_bytes())
        })
    }
}

fn utf8(path: &Path) -> Result<String, Error> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        Error::message(format!("non-utf8 path: {}", path.display()))
    })
}
