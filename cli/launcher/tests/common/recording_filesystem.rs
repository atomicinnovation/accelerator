//! A `CacheFilesystem` decorator that records every call, so a test can assert
//! a dispatch's exact filesystem sequence.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use accelerator::launch::outbound::resolve::filesystem::{
    AccessAcl, CacheFilesystem, EntryMetadata, ResolvedDirectory,
    SystemFilesystem,
};

/// A path relative to the cache root, or else to the test's outer anchor, so
/// an expected sequence is independent of where the temp directory lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedPath {
    InRoot(PathBuf),
    Outside(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesystemCall {
    List(RecordedPath),
    Exists(RecordedPath),
    Read(RecordedPath),
    ReadToString(RecordedPath),
    ReadBounded(RecordedPath),
    CreateDirAll(RecordedPath),
    WritePrivate(RecordedPath),
    MakeExecutable(RecordedPath),
    Rename {
        from: RecordedPath,
        to: RecordedPath,
    },
    RemoveFile(RecordedPath),
    Entry(RecordedPath),
    Resolve(RecordedPath),
    CreatePrivateDir(RecordedPath),
    SetModified(RecordedPath),
    AccessAcl(RecordedPath),
    IgnoresOwnership(RecordedPath),
}

impl FilesystemCall {
    pub const fn is_mutation(&self) -> bool {
        matches!(
            self,
            Self::CreateDirAll(_)
                | Self::WritePrivate(_)
                | Self::MakeExecutable(_)
                | Self::Rename { .. }
                | Self::RemoveFile(_)
                | Self::CreatePrivateDir(_)
                | Self::SetModified(_)
        )
    }

    fn touches(&self, path: &RecordedPath) -> bool {
        match self {
            Self::Rename { from, to } => from == path || to == path,
            Self::List(touched)
            | Self::Exists(touched)
            | Self::Read(touched)
            | Self::ReadToString(touched)
            | Self::ReadBounded(touched)
            | Self::CreateDirAll(touched)
            | Self::WritePrivate(touched)
            | Self::MakeExecutable(touched)
            | Self::RemoveFile(touched)
            | Self::Entry(touched)
            | Self::Resolve(touched)
            | Self::CreatePrivateDir(touched)
            | Self::SetModified(touched)
            | Self::AccessAcl(touched)
            | Self::IgnoresOwnership(touched) => touched == path,
        }
    }
}

pub fn in_root(relative: impl Into<PathBuf>) -> RecordedPath {
    RecordedPath::InRoot(relative.into())
}

pub fn outside(relative: impl Into<PathBuf>) -> RecordedPath {
    RecordedPath::Outside(relative.into())
}

/// Both spellings of a directory: on macOS a temp directory under
/// `/var/folders` is canonically `/private/var/folders`.
#[derive(Clone)]
struct Spellings {
    given: PathBuf,
    canonical: PathBuf,
}

impl Spellings {
    fn of(given: &Path) -> Self {
        Self {
            given: given.to_path_buf(),
            canonical: given
                .canonicalize()
                .unwrap_or_else(|_| given.to_path_buf()),
        }
    }

    fn relative(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(&self.given)
            .or_else(|_| path.strip_prefix(&self.canonical))
            .ok()
            .map(Path::to_path_buf)
    }
}

#[derive(Clone)]
pub struct RecordingFilesystem {
    root: Spellings,
    anchor: Spellings,
    calls: Arc<Mutex<Vec<FilesystemCall>>>,
}

impl RecordingFilesystem {
    pub fn new(root: &Path, anchor: &Path) -> Self {
        Self {
            root: Spellings::of(root),
            anchor: Spellings::of(anchor),
            calls: Arc::default(),
        }
    }

    pub fn calls(&self) -> Vec<FilesystemCall> {
        self.calls.lock().expect("recorded calls").clone()
    }

    pub fn calls_touching(&self, path: &Path) -> Vec<FilesystemCall> {
        let path = self.recorded(path);
        self.calls()
            .into_iter()
            .filter(|call| call.touches(&path))
            .collect()
    }

    pub fn recorded(&self, path: &Path) -> RecordedPath {
        self.root.relative(path).map_or_else(
            || {
                RecordedPath::Outside(
                    self.anchor
                        .relative(path)
                        .unwrap_or_else(|| path.to_path_buf()),
                )
            },
            RecordedPath::InRoot,
        )
    }

    fn record(&self, call: FilesystemCall) {
        self.calls.lock().expect("recorded calls").push(call);
    }
}

impl CacheFilesystem for RecordingFilesystem {
    fn list(&self, dir: &Path) -> io::Result<Vec<OsString>> {
        self.record(FilesystemCall::List(self.recorded(dir)));
        SystemFilesystem.list(dir)
    }

    fn exists(&self, path: &Path) -> bool {
        self.record(FilesystemCall::Exists(self.recorded(path)));
        SystemFilesystem.exists(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.record(FilesystemCall::Read(self.recorded(path)));
        SystemFilesystem.read(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.record(FilesystemCall::ReadToString(self.recorded(path)));
        SystemFilesystem.read_to_string(path)
    }

    fn read_bounded(&self, path: &Path, limit: u64) -> io::Result<Vec<u8>> {
        self.record(FilesystemCall::ReadBounded(self.recorded(path)));
        SystemFilesystem.read_bounded(path, limit)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.record(FilesystemCall::CreateDirAll(self.recorded(path)));
        SystemFilesystem.create_dir_all(path)
    }

    fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.record(FilesystemCall::WritePrivate(self.recorded(path)));
        SystemFilesystem.write_private(path, bytes)
    }

    fn make_executable(&self, path: &Path) -> io::Result<()> {
        self.record(FilesystemCall::MakeExecutable(self.recorded(path)));
        SystemFilesystem.make_executable(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.record(FilesystemCall::Rename {
            from: self.recorded(from),
            to: self.recorded(to),
        });
        SystemFilesystem.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.record(FilesystemCall::RemoveFile(self.recorded(path)));
        SystemFilesystem.remove_file(path)
    }

    fn entry(&self, path: &Path) -> io::Result<EntryMetadata> {
        self.record(FilesystemCall::Entry(self.recorded(path)));
        SystemFilesystem.entry(path)
    }

    fn resolve(&self, path: &Path) -> io::Result<ResolvedDirectory> {
        self.record(FilesystemCall::Resolve(self.recorded(path)));
        SystemFilesystem.resolve(path)
    }

    fn create_private_dir(&self, path: &Path) -> io::Result<()> {
        self.record(FilesystemCall::CreatePrivateDir(self.recorded(path)));
        SystemFilesystem.create_private_dir(path)
    }

    fn set_modified(&self, path: &Path, seconds: u64) -> io::Result<()> {
        self.record(FilesystemCall::SetModified(self.recorded(path)));
        SystemFilesystem.set_modified(path, seconds)
    }

    fn access_acl(&self, path: &Path) -> io::Result<AccessAcl> {
        self.record(FilesystemCall::AccessAcl(self.recorded(path)));
        SystemFilesystem.access_acl(path)
    }

    fn ignores_ownership(&self, path: &Path) -> io::Result<bool> {
        self.record(FilesystemCall::IgnoresOwnership(self.recorded(path)));
        SystemFilesystem.ignores_ownership(path)
    }
}

mod tests {
    use accelerator::launch::outbound::resolve::filesystem::CacheFilesystem as _;

    use super::{in_root, outside, FilesystemCall, RecordingFilesystem};

    #[test]
    fn a_path_is_recorded_relative_to_either_spelling_of_root_and_anchor() {
        let real = tempfile::tempdir().expect("real anchor");
        let links = tempfile::tempdir().expect("link directory");
        let anchor = links.path().join("anchor");
        std::os::unix::fs::symlink(real.path(), &anchor).expect("symlink");
        let root = anchor.join("cache");
        std::fs::create_dir(&root).expect("cache root");
        let canonical = anchor.canonicalize().expect("canonical anchor");
        let recorder = RecordingFilesystem::new(&root, &anchor);

        for path in [
            root.join("given"),
            canonical.join("cache/canonical"),
            anchor.join("plugin/given"),
            canonical.join("plugin/canonical"),
        ] {
            recorder.exists(&path);
        }

        assert_eq!(
            recorder.calls(),
            [
                FilesystemCall::Exists(in_root("given")),
                FilesystemCall::Exists(in_root("canonical")),
                FilesystemCall::Exists(outside("plugin/given")),
                FilesystemCall::Exists(outside("plugin/canonical")),
            ]
        );
        assert_ne!(canonical, anchor, "the spellings must differ");
        assert_eq!(
            recorder.calls_touching(&root.join("given")),
            [FilesystemCall::Exists(in_root("given"))]
        );
    }
}
