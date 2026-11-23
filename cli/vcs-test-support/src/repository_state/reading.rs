//! Reading a repository's commits and references as their IDs name them.
//!
//! Each reader spawns exactly two processes, whatever the number of commits:
//! one listing, and one `git cat-file --batch` session that reads every
//! commit, tree and blob.

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

use super::RawCommit;
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

/// Reads a git repository's `HEAD`, branches and tags, and every commit they
/// reach.
///
/// # Errors
///
/// When git cannot list the references or read an object.
pub(super) fn git(
    root: &Path,
    hermetic: &Hermetic,
) -> Result<RawRepository, Error> {
    let listing = hermetic.git(
        &[
            "for-each-ref",
            "--format=%(refname)%00%(objectname)%00%(*objectname)",
            "refs/heads",
            "refs/tags",
        ],
        root,
    )?;
    let mut store = ObjectStore::open(root, None, hermetic)?;

    let mut references = BTreeMap::new();
    for line in listing.lines() {
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
    let mut working_copy_parents = Vec::new();
    if let Some(head) =
        store.object("HEAD")?.filter(|head| head.kind == "commit")
    {
        references.insert("HEAD".to_owned(), head.id.clone());
        working_copy_parents.push(head.id);
    }

    let mut commits = Vec::new();
    let mut seen = BTreeSet::new();
    let mut unread: Vec<String> = references.values().cloned().collect();
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
        references,
        working_copy_parents,
    })
}

/// Reads every visible commit of a jj repository, its bookmarks and its
/// working copy's parents, without snapshotting the working copy.
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
    let mut references = BTreeMap::new();
    let mut working_copy_parents = Vec::new();
    for record in listing
        .split(RECORD)
        .filter(|record| !record.trim().is_empty())
    {
        let fields: Vec<&str> = record.split(FIELD).collect();
        let [id, change, parents, bookmarks, working_copy, description] =
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
        for bookmark in bookmarks.split(',').filter(|name| !name.is_empty()) {
            references.insert(bookmark.to_owned(), id.clone());
        }
        if !working_copy.is_empty() {
            working_copy_parents.clone_from(&parents);
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
    Ok(RawRepository {
        kind: RepositoryKind::Jj(colocation),
        commits,
        references,
        working_copy_parents,
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

    fn files(&mut self, tree: &str) -> Result<BTreeMap<String, String>, Error> {
        let mut files = BTreeMap::new();
        self.add_files(tree, "", &mut files)?;
        Ok(files)
    }

    fn add_files(
        &mut self,
        tree: &str,
        prefix: &str,
        files: &mut BTreeMap<String, String>,
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
                "160000" => {}
                mode => {
                    let content = self.blob(&entry.id)?;
                    let content = if mode == "120000" {
                        format!("symlink to {content}")
                    } else {
                        content
                    };
                    files.insert(path, content);
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
