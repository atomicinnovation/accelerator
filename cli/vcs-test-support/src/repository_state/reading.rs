//! Reading a repository's commits and references as their IDs name them.
//!
//! Each reader spawns a fixed number of processes, whatever the number of
//! commits: a listing of the commits or references, a listing of git's index
//! where there is one, and one `git cat-file --batch` session that reads
//! every commit, tree and blob.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::Child;
use std::process::ChildStdin;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Stdio;

use super::FileKind;
use super::FileState;
use super::Head;
use super::RawCommit;
use super::RawGitSide;
use super::RawRepository;
use super::RepositoryKind;
use crate::colocation::JjColocation;
use crate::hermetic::Hermetic;
use crate::Error;

const FIELD: char = '\u{1f}';
const RECORD: char = '\u{1e}';

const JJ_LISTING: &str = r#"commit_id ++ "\x1f" ++ change_id ++ "\x1f"
    ++ parents.map(|parent| parent.commit_id()).join(",") ++ "\x1f"
    ++ local_bookmarks.map(|bookmark| bookmark.name()).join(",") ++ "\x1f"
    ++ if(current_working_copy, "working-copy") ++ "\x1f"
    ++ description ++ "\x1e""#;

/// Reads a git repository's `HEAD`, branches, tags and index, and every
/// commit they reach.
///
/// # Errors
///
/// When git cannot list the references or the index, or read an object.
pub(super) fn git(
    root: &Path,
    hermetic: &Hermetic,
) -> Result<RawRepository, Error> {
    let mut store = ObjectStore::open(root, None, hermetic)?;
    let git = git_side(root, hermetic, &mut store)?;
    let working_copy_parents: Vec<String> =
        git.references.get(HEAD).cloned().into_iter().collect();

    let mut commits = Vec::new();
    let mut seen = BTreeSet::new();
    let mut unread: Vec<String> = git.references.values().cloned().collect();
    while let Some(id) = unread.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let commit = store.commit(&id)?;
        unread.extend(commit.parents.iter().cloned());
        commits.push(RawCommit {
            id,
            change: None,
            parents: commit.parents,
            description: commit.message,
            files: store.files(&commit.tree)?,
        });
    }
    Ok(RawRepository {
        kind: RepositoryKind::Git,
        commits,
        bookmarks: BTreeMap::new(),
        git: Some(git),
        working_copy_parents,
        working_copy_commit: None,
    })
}

const HEAD: &str = "HEAD";

/// Reads git's `HEAD`, branches, tags and index, which `store` reads the
/// objects of.
fn git_side(
    root: &Path,
    hermetic: &Hermetic,
    store: &mut ObjectStore,
) -> Result<RawGitSide, Error> {
    let references_listing = hermetic.git(
        &[
            "for-each-ref",
            "--format=%(refname)%00%(objectname)%00%(*objectname)",
            "refs/heads",
            "refs/tags",
        ],
        root,
    )?;
    let index_listing = hermetic.git(&["ls-files", "--stage", "-z"], root)?;

    let mut references = BTreeMap::new();
    for line in references_listing.lines() {
        let mut fields = line.split('\0');
        let (Some(name), Some(object), peeled) =
            (fields.next(), fields.next(), fields.next())
        else {
            return Err(Error::message(format!("unreadable reference {line}")));
        };
        let target =
            peeled.filter(|peeled| !peeled.is_empty()).unwrap_or(object);
        references.insert(name.to_owned(), target.to_owned());
    }
    if let Some(head) = store.object(HEAD)?.filter(|head| head.kind == "commit")
    {
        references.insert(HEAD.to_owned(), head.id);
    }

    let mut index = BTreeMap::new();
    for entry in index_listing.split('\0').filter(|entry| !entry.is_empty()) {
        let unreadable =
            || Error::message(format!("unreadable index entry {entry}"));
        let (staged, path) = entry.split_once('\t').ok_or_else(unreadable)?;
        let [mode, id, stage] = staged.split(' ').collect::<Vec<_>>()[..]
        else {
            return Err(unreadable());
        };
        if mode == GITLINK_MODE {
            continue;
        }
        let key = if stage == "0" {
            path.to_owned()
        } else {
            format!("{path} (stage {stage})")
        };
        let file = FileState {
            kind: FileKind::of_git_mode(mode),
            content: store.blob(id)?,
        };
        index.insert(key, file);
    }
    Ok(RawGitSide {
        head: head(root)?,
        references,
        index,
    })
}

fn head(root: &Path) -> Result<Head, Error> {
    let path = root.join(".git").join(HEAD);
    let head = fs::read_to_string(&path).map_err(|error| {
        Error::message(format!("cannot read {}: {error}", path.display()))
    })?;
    Ok(head
        .trim()
        .strip_prefix("ref: ")
        .map_or(Head::Detached, |branch| Head::Attached(branch.to_owned())))
}

const GITLINK_MODE: &str = "160000";

/// Reads every visible commit of a jj repository, its bookmarks, its working
/// copy's parents and, when colocated, its git side, without snapshotting
/// the working copy.
///
/// # Errors
///
/// When jj cannot list the commits, or the backing git store cannot be read.
pub(super) fn jj(
    root: &Path,
    hermetic: &Hermetic,
    colocation: JjColocation,
) -> Result<RawRepository, Error> {
    let listing = hermetic.jj(
        &[
            "--ignore-working-copy",
            "log",
            "--no-graph",
            "-r",
            "all() ~ root()",
            "-T",
            JJ_LISTING,
        ],
        root,
    )?;
    let mut store =
        ObjectStore::open(root, Some(&backing_store(root)?), hermetic)?;

    let mut commits = Vec::new();
    let mut bookmarks = BTreeMap::new();
    let mut working_copy_parents = Vec::new();
    let mut working_copy_commit = None;
    for record in listing
        .split(RECORD)
        .filter(|record| !record.trim().is_empty())
    {
        let fields: Vec<&str> = record.split(FIELD).collect();
        let [id, change, parents, bookmarked, working_copy, description] =
            fields.as_slice()
        else {
            return Err(Error::message(format!(
                "unreadable jj commit {record}"
            )));
        };
        let id = id.trim().to_owned();
        let parents: Vec<String> = parents
            .split(',')
            .filter(|parent| {
                !parent.is_empty() && !names_the_root_commit(parent)
            })
            .map(str::to_owned)
            .collect();
        for bookmark in bookmarked.split(',').filter(|name| !name.is_empty()) {
            bookmarks.insert(bookmark.to_owned(), id.clone());
        }
        if !working_copy.is_empty() {
            working_copy_parents.clone_from(&parents);
            working_copy_commit = Some(id.clone());
        }
        let tree = store.commit(&id)?.tree;
        commits.push(RawCommit {
            id,
            change: Some((*change).to_owned()),
            parents,
            description: (*description).to_owned(),
            files: store.files(&tree)?,
        });
    }
    let git = match colocation {
        JjColocation::Pure => None,
        JjColocation::Colocated => Some(git_side(root, hermetic, &mut store)?),
    };
    Ok(RawRepository {
        kind: RepositoryKind::Jj(colocation),
        commits,
        bookmarks,
        git,
        working_copy_parents,
        working_copy_commit,
    })
}

fn names_the_root_commit(id: &str) -> bool {
    id.chars().all(|digit| digit == '0')
}

fn backing_store(root: &Path) -> Result<std::path::PathBuf, Error> {
    let store = root.join(".jj/repo/store");
    let target =
        fs::read_to_string(store.join("git_target")).map_err(|error| {
            Error::message(format!(
                "{} names no backing git store: {error}",
                root.display()
            ))
        })?;
    Ok(store.join(target.trim()))
}

struct Object {
    id: String,
    kind: String,
    content: Vec<u8>,
}

struct GitCommit {
    tree: String,
    parents: Vec<String>,
    message: String,
}

/// One `git cat-file --batch` session, answering every object a projection
/// asks for.
struct ObjectStore {
    session: Child,
    requests: Option<ChildStdin>,
    answers: BufReader<ChildStdout>,
    blobs: HashMap<String, String>,
}

impl ObjectStore {
    fn open(
        root: &Path,
        git_dir: Option<&Path>,
        hermetic: &Hermetic,
    ) -> Result<Self, Error> {
        let mut command = Command::new("git");
        if let Some(git_dir) = git_dir {
            command.arg("--git-dir").arg(git_dir);
        }
        command
            .args(["cat-file", "--batch"])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hermetic.apply(&mut command);
        let mut session = command.spawn().map_err(|error| {
            Error::message(format!("git cat-file: {error}"))
        })?;
        let requests = session.stdin.take();
        let answers = session
            .stdout
            .take()
            .map(BufReader::new)
            .ok_or_else(|| Error::message("git cat-file gave no output"))?;
        Ok(Self {
            session,
            requests,
            answers,
            blobs: HashMap::new(),
        })
    }

    fn object(&mut self, name: &str) -> Result<Option<Object>, Error> {
        let requests = self
            .requests
            .as_mut()
            .ok_or_else(|| Error::message("git cat-file has closed"))?;
        writeln!(requests, "{name}")?;
        requests.flush()?;
        let mut header = String::new();
        self.answers.read_line(&mut header)?;
        let header = header.trim_end();
        if header.ends_with(" missing") {
            return Ok(None);
        }
        let unreadable =
            || Error::message(format!("unreadable object header {header}"));
        let mut fields = header.split(' ');
        let (Some(id), Some(kind), Some(size)) =
            (fields.next(), fields.next(), fields.next())
        else {
            return Err(unreadable());
        };
        let size: usize = size.parse().map_err(|_| unreadable())?;
        let mut content = vec![0; size + 1];
        self.answers.read_exact(&mut content)?;
        content.truncate(size);
        Ok(Some(Object {
            id: id.to_owned(),
            kind: kind.to_owned(),
            content,
        }))
    }

    fn of_kind(&mut self, name: &str, kind: &str) -> Result<Object, Error> {
        match self.object(name)? {
            Some(object) if object.kind == kind => Ok(object),
            Some(object) => Err(Error::message(format!(
                "{name} is a {}, not a {kind}",
                object.kind
            ))),
            None => {
                Err(Error::message(format!("the git store holds no {name}")))
            }
        }
    }

    fn commit(&mut self, id: &str) -> Result<GitCommit, Error> {
        let object = self.of_kind(id, "commit")?;
        let text = String::from_utf8_lossy(&object.content);
        let (headers, message) = text.split_once("\n\n").unwrap_or((&text, ""));
        let mut tree = None;
        let mut parents = Vec::new();
        for header in headers.lines() {
            if let Some(id) = header.strip_prefix("tree ") {
                tree = Some(id.to_owned());
            } else if let Some(id) = header.strip_prefix("parent ") {
                parents.push(id.to_owned());
            }
        }
        Ok(GitCommit {
            tree: tree.ok_or_else(|| {
                Error::message(format!("commit {id} names no tree"))
            })?,
            parents,
            message: message.to_owned(),
        })
    }

    fn files(
        &mut self,
        tree: &str,
    ) -> Result<BTreeMap<String, FileState>, Error> {
        let mut files = BTreeMap::new();
        self.add_files(tree, "", &mut files)?;
        Ok(files)
    }

    fn add_files(
        &mut self,
        tree: &str,
        prefix: &str,
        files: &mut BTreeMap<String, FileState>,
    ) -> Result<(), Error> {
        let object = self.of_kind(tree, "tree")?;
        let id_length = object.id.len() / 2;
        for entry in TreeEntries::new(&object.content, id_length) {
            let entry = entry.ok_or_else(|| {
                Error::message(format!("unreadable tree {tree}"))
            })?;
            let path = format!("{prefix}{}", entry.name);
            match entry.mode.as_str() {
                "40000" => {
                    self.add_files(&entry.id, &format!("{path}/"), files)?;
                }
                GITLINK_MODE => {}
                mode => {
                    let file = FileState {
                        kind: FileKind::of_git_mode(mode),
                        content: self.blob(&entry.id)?,
                    };
                    files.insert(path, file);
                }
            }
        }
        Ok(())
    }

    fn blob(&mut self, id: &str) -> Result<String, Error> {
        if let Some(content) = self.blobs.get(id) {
            return Ok(content.clone());
        }
        let object = self.of_kind(id, "blob")?;
        let content = String::from_utf8_lossy(&object.content).into_owned();
        self.blobs.insert(id.to_owned(), content.clone());
        Ok(content)
    }
}

impl Drop for ObjectStore {
    fn drop(&mut self) {
        drop(self.requests.take());
        let _ = self.session.wait();
    }
}

struct TreeEntry {
    mode: String,
    name: String,
    id: String,
}

/// The entries of a raw tree object: `<mode> <name>\0<binary id>` repeated.
struct TreeEntries<'t> {
    rest: &'t [u8],
    id_length: usize,
}

impl<'t> TreeEntries<'t> {
    const fn new(content: &'t [u8], id_length: usize) -> Self {
        Self {
            rest: content,
            id_length,
        }
    }
}

impl Iterator for TreeEntries<'_> {
    type Item = Option<TreeEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let entry = (|| {
            let space = self.rest.iter().position(|&byte| byte == b' ')?;
            let nul = self.rest.iter().position(|&byte| byte == 0)?;
            let id = self.rest.get(nul + 1..nul + 1 + self.id_length)?;
            let entry = TreeEntry {
                mode: String::from_utf8_lossy(self.rest.get(..space)?)
                    .into_owned(),
                name: String::from_utf8_lossy(self.rest.get(space + 1..nul)?)
                    .into_owned(),
                id: hex(id),
            };
            self.rest = self.rest.get(nul + 1 + self.id_length..)?;
            Some(entry)
        })();
        if entry.is_none() {
            self.rest = &[];
        }
        Some(entry)
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0xf)],
            ]
        })
        .map(char::from)
        .collect()
}

impl FileKind {
    fn of_git_mode(mode: &str) -> Self {
        match mode {
            "100755" => Self::Executable,
            "120000" => Self::Symlink,
            _ => Self::Regular,
        }
    }
}
