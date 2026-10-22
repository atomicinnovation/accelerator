//! A `CacheFilesystem` decorator over the recorder that fails, or runs an
//! action before, the nth call of a given kind on a given path.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use accelerator::launch::outbound::resolve::filesystem::{
    AccessAcl, CacheFilesystem, EntryMetadata, ResolvedDirectory,
};

use super::recording_filesystem::{
    CallKind, FilesystemCall, RecordedPath, RecordingFilesystem, ScriptedEvent,
};

type Action = Box<dyn Fn() + Send + Sync>;

enum Response {
    Fail(io::ErrorKind),
    RunBefore(Action),
}

struct Script {
    kind: CallKind,
    path: RecordedPath,
    nth: usize,
    response: Response,
}

pub struct ScriptedFilesystem {
    recorder: RecordingFilesystem,
    scripts: Vec<Script>,
    seen: Mutex<HashMap<(CallKind, RecordedPath), usize>>,
    fired: Mutex<Vec<ScriptedEvent>>,
}

impl ScriptedFilesystem {
    pub fn over(recorder: RecordingFilesystem) -> Self {
        Self {
            recorder,
            scripts: Vec::new(),
            seen: Mutex::default(),
            fired: Mutex::default(),
        }
    }

    pub fn failing(
        mut self,
        call: CallKind,
        path: &Path,
        nth: usize,
        error: io::ErrorKind,
    ) -> Self {
        self.script(call, path, nth, Response::Fail(error));
        self
    }

    pub fn before(
        mut self,
        call: CallKind,
        path: &Path,
        nth: usize,
        action: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        self.script(call, path, nth, Response::RunBefore(Box::new(action)));
        self
    }

    pub fn fired(&self) -> Vec<ScriptedEvent> {
        self.fired.lock().expect("fired").clone()
    }

    fn script(
        &mut self,
        kind: CallKind,
        path: &Path,
        nth: usize,
        response: Response,
    ) {
        self.scripts.push(Script {
            kind,
            path: self.recorder.recorded(path),
            nth,
            response,
        });
    }

    /// The error to answer with instead of making `call`, if any.
    fn intercept(&self, call: &FilesystemCall) -> Option<io::Error> {
        let nth = {
            let mut seen = self.seen.lock().expect("seen");
            let count = seen
                .entry((call.kind(), primary_path(call).clone()))
                .or_insert(0);
            *count += 1;
            let nth = *count;
            drop(seen);
            nth
        };
        let mut failure = None;
        for script in self.scripts.iter().filter(|script| {
            script.kind == call.kind()
                && &script.path == primary_path(call)
                && script.nth == nth
        }) {
            let event = match &script.response {
                Response::RunBefore(action) => {
                    action();
                    ScriptedEvent::Ran {
                        before: call.clone(),
                        nth,
                    }
                }
                Response::Fail(kind) => {
                    failure = Some(io::Error::from(*kind));
                    ScriptedEvent::Failed {
                        call: call.clone(),
                        nth,
                    }
                }
            };
            self.fired.lock().expect("fired").push(event.clone());
            self.recorder.note(event);
        }
        failure
    }

    fn intercepted<T>(
        &self,
        call: &FilesystemCall,
        make: impl FnOnce() -> io::Result<T>,
    ) -> io::Result<T> {
        self.intercept(call).map_or_else(make, Err)
    }

    fn recorded(&self, path: &Path) -> RecordedPath {
        self.recorder.recorded(path)
    }
}

const fn primary_path(call: &FilesystemCall) -> &RecordedPath {
    match call {
        FilesystemCall::Rename { from, .. } => from,
        FilesystemCall::List(path)
        | FilesystemCall::Exists(path)
        | FilesystemCall::Read(path)
        | FilesystemCall::ReadToString(path)
        | FilesystemCall::ReadBounded(path)
        | FilesystemCall::CreateDirAll(path)
        | FilesystemCall::WritePrivate(path)
        | FilesystemCall::MakeExecutable(path)
        | FilesystemCall::RemoveFile(path)
        | FilesystemCall::RemoveDir(path)
        | FilesystemCall::Entry(path)
        | FilesystemCall::Resolve(path)
        | FilesystemCall::CreatePrivateDir(path)
        | FilesystemCall::SetModified(path)
        | FilesystemCall::AccessAcl(path)
        | FilesystemCall::IgnoresOwnership(path) => path,
    }
}

impl CacheFilesystem for ScriptedFilesystem {
    fn list(&self, dir: &Path) -> io::Result<Vec<OsString>> {
        self.intercepted(&FilesystemCall::List(self.recorded(dir)), || {
            self.recorder.list(dir)
        })
    }

    fn exists(&self, path: &Path) -> bool {
        self.intercepted(&FilesystemCall::Exists(self.recorded(path)), || {
            Ok(self.recorder.exists(path))
        })
        .unwrap_or(false)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.intercepted(&FilesystemCall::Read(self.recorded(path)), || {
            self.recorder.read(path)
        })
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.intercepted(
            &FilesystemCall::ReadToString(self.recorded(path)),
            || self.recorder.read_to_string(path),
        )
    }

    fn read_bounded(&self, path: &Path, limit: u64) -> io::Result<Vec<u8>> {
        self.intercepted(
            &FilesystemCall::ReadBounded(self.recorded(path)),
            || self.recorder.read_bounded(path, limit),
        )
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::CreateDirAll(self.recorded(path)),
            || self.recorder.create_dir_all(path),
        )
    }

    fn write_private(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::WritePrivate(self.recorded(path)),
            || self.recorder.write_private(path, bytes),
        )
    }

    fn make_executable(&self, path: &Path) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::MakeExecutable(self.recorded(path)),
            || self.recorder.make_executable(path),
        )
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::Rename {
                from: self.recorded(from),
                to: self.recorded(to),
            },
            || self.recorder.rename(from, to),
        )
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::RemoveFile(self.recorded(path)),
            || self.recorder.remove_file(path),
        )
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::RemoveDir(self.recorded(path)),
            || self.recorder.remove_dir(path),
        )
    }

    fn entry(&self, path: &Path) -> io::Result<EntryMetadata> {
        self.intercepted(&FilesystemCall::Entry(self.recorded(path)), || {
            self.recorder.entry(path)
        })
    }

    fn resolve(&self, path: &Path) -> io::Result<ResolvedDirectory> {
        self.intercepted(&FilesystemCall::Resolve(self.recorded(path)), || {
            self.recorder.resolve(path)
        })
    }

    fn create_private_dir(&self, path: &Path) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::CreatePrivateDir(self.recorded(path)),
            || self.recorder.create_private_dir(path),
        )
    }

    fn set_modified(&self, path: &Path, seconds: u64) -> io::Result<()> {
        self.intercepted(
            &FilesystemCall::SetModified(self.recorded(path)),
            || self.recorder.set_modified(path, seconds),
        )
    }

    fn access_acl(&self, path: &Path) -> io::Result<AccessAcl> {
        self.intercepted(
            &FilesystemCall::AccessAcl(self.recorded(path)),
            || self.recorder.access_acl(path),
        )
    }

    fn ignores_ownership(&self, path: &Path) -> io::Result<bool> {
        self.intercepted(
            &FilesystemCall::IgnoresOwnership(self.recorded(path)),
            || self.recorder.ignores_ownership(path),
        )
    }
}
