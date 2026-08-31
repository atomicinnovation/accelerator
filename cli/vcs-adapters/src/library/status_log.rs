//! The status and log adapters over `gix` and `jj-lib`, in-process.
//!
//! Both backends populate the backend-neutral `vcs::status`/`vcs::log` models
//! the renderer consumes. The git side reads `gix::Repository::status`
//! and a first-parent revwalk; the jj side reads the shared working-copy
//! snapshot (`snapshot::working_copy_diff`) plus a first-parent change-id peel
//! (`snapshot::head_commit`), so the `UserSettings` construction stays confined
//! to `snapshot`.
//!
//! git's change types come from two sources — the tree->index diff (staged, what
//! the commit will contain) and the index->worktree diff (unstaged and
//! untracked). `resolve` collapses same-path collisions by commit-accuracy:
//! conflict overrides, else the staged type wins, else the worktree type stands.

use std::path::Path;

use jj_lib::repo::Repo as _;
use vcs::log::LogEntry;
use vcs::log::LogReport;
use vcs::status::ChangeType;
use vcs::status::FileChange;
use vcs::status::StatusReport;

use crate::library::snapshot;
use crate::library::Error;

const RECENT_LIMIT: usize = 5;

fn git_err<E>(root: &Path) -> impl Fn(E) -> Error + '_
where
    E: std::error::Error + Send + Sync + 'static,
{
    move |source| Error::Git {
        path: root.to_path_buf(),
        source: Box::new(source),
    }
}

fn jj_err<E>(root: &Path) -> impl Fn(E) -> Error + '_
where
    E: std::error::Error + Send + Sync + 'static,
{
    move |source| Error::Jj {
        path: root.to_path_buf(),
        source: Box::new(source),
    }
}

/// Whether an observation came from the staged (tree->index) diff or the
/// worktree (index->worktree, including the dirwalk) diff — the discriminator
/// [`resolve`] needs, since the staged type is what the commit records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Staged,
    Worktree,
}

/// One raw status observation, neutral over gix's two item enums, so [`classify`]
/// is unit-testable off any real repository. `Renamed` carries the old path; the
/// item's own `path` is the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Observation {
    Untracked,
    Added,
    Removed,
    Modified,
    Conflicted,
    Renamed { old: String },
    Copied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawChange {
    source: Source,
    observation: Observation,
    path: String,
}

/// The `FileChange`(s) one observation produces: 0, 1, or 2. A rename yields two
/// (delete old, add new); a copy yields the new path only, since the source is
/// unchanged and present.
fn classify(raw: &RawChange) -> Vec<FileChange> {
    let at = |change_type, path: &str| FileChange {
        change_type,
        path: path.to_owned(),
    };
    match &raw.observation {
        Observation::Untracked => vec![at(ChangeType::Untracked, &raw.path)],
        // A copy surfaces as the new path only; the source is unchanged.
        Observation::Added | Observation::Copied => {
            vec![at(ChangeType::Added, &raw.path)]
        }
        Observation::Removed => vec![at(ChangeType::Deleted, &raw.path)],
        Observation::Modified => vec![at(ChangeType::Modified, &raw.path)],
        Observation::Conflicted => vec![at(ChangeType::Conflicted, &raw.path)],
        Observation::Renamed { old } => {
            vec![
                at(ChangeType::Deleted, old),
                at(ChangeType::Added, &raw.path),
            ]
        }
    }
}

/// Collapses same-path collisions to one entry per path by commit-accuracy:
/// conflict overrides; otherwise the staged (tree->index) type wins, because
/// that is what the commit will contain; otherwise the worktree type stands.
fn resolve(items: Vec<(Source, FileChange)>) -> Vec<FileChange> {
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Accumulated {
        conflicted: bool,
        staged: Option<ChangeType>,
        worktree: Option<ChangeType>,
    }

    let mut by_path: BTreeMap<String, Accumulated> = BTreeMap::new();
    for (source, change) in items {
        let accumulated = by_path.entry(change.path).or_default();
        if change.change_type == ChangeType::Conflicted {
            accumulated.conflicted = true;
        }
        match source {
            Source::Staged => accumulated.staged = Some(change.change_type),
            Source::Worktree => {
                accumulated.worktree.get_or_insert(change.change_type);
            }
        }
    }

    by_path
        .into_iter()
        .filter_map(|(path, accumulated)| {
            let change_type = if accumulated.conflicted {
                ChangeType::Conflicted
            } else {
                accumulated.staged.or(accumulated.worktree)?
            };
            Some(FileChange { change_type, path })
        })
        .collect()
}

fn to_string(location: &gix::bstr::BStr) -> String {
    String::from_utf8_lossy(location).into_owned()
}

/// The raw observation for one gix status item, or `None` for an item that
/// carries no user-facing change (an index-update hint, an ignored dir entry).
fn observe(item: &gix::status::Item) -> Option<RawChange> {
    use gix::status::index_worktree::iter::Summary;

    match item {
        gix::status::Item::TreeIndex(change) => {
            let (observation, path) = match change {
                gix::diff::index::Change::Addition { location, .. } => {
                    (Observation::Added, to_string(location.as_ref()))
                }
                gix::diff::index::Change::Deletion { location, .. } => {
                    (Observation::Removed, to_string(location.as_ref()))
                }
                gix::diff::index::Change::Modification { location, .. } => {
                    (Observation::Modified, to_string(location.as_ref()))
                }
                gix::diff::index::Change::Rewrite {
                    source_location,
                    location,
                    copy,
                    ..
                } => {
                    if *copy {
                        (Observation::Copied, to_string(location.as_ref()))
                    } else {
                        (
                            Observation::Renamed {
                                old: to_string(source_location.as_ref()),
                            },
                            to_string(location.as_ref()),
                        )
                    }
                }
            };
            Some(RawChange {
                source: Source::Staged,
                observation,
                path,
            })
        }
        gix::status::Item::IndexWorktree(inner) => {
            let path = to_string(item.location());
            let observation = match inner.summary()? {
                Summary::Conflict => Observation::Conflicted,
                Summary::Removed => Observation::Removed,
                Summary::Modified | Summary::TypeChange => {
                    Observation::Modified
                }
                Summary::Added => Observation::Untracked,
                Summary::IntentToAdd => Observation::Added,
                Summary::Copied => Observation::Copied,
                Summary::Renamed => Observation::Renamed {
                    old: worktree_rename_source(inner)?,
                },
            };
            Some(RawChange {
                source: Source::Worktree,
                observation,
                path,
            })
        }
    }
}

/// The source path of a worktree rename, from the inner `Rewrite` item. Only
/// reachable when rewrite-tracking is enabled (off by default), so it never
/// fires in the fixture matrix, but keeps `Renamed` honest if a global config
/// turns tracking on.
fn worktree_rename_source(
    item: &gix::status::index_worktree::Item,
) -> Option<String> {
    match item {
        gix::status::index_worktree::Item::Rewrite { source, .. } => {
            Some(to_string(source.rela_path()))
        }
        _ => None,
    }
}

pub(super) fn git_status(root: &Path) -> Result<StatusReport, Error> {
    let repository = gix::open(root).map_err(git_err(root))?;

    let branch = repository
        .head_name()
        .map_err(git_err(root))?
        .map_or_else(Vec::new, |name| vec![to_string(name.shorten())]);

    let status = repository
        .status(gix::progress::Discard)
        .map_err(git_err(root))?
        .untracked_files(gix::status::UntrackedFiles::Files);
    let iter = status
        .into_iter(Vec::<gix::bstr::BString>::new())
        .map_err(git_err(root))?;

    let mut items = Vec::new();
    for item in iter {
        let item = item.map_err(git_err(root))?;
        if let Some(raw) = observe(&item) {
            for change in classify(&raw) {
                items.push((raw.source, change));
            }
        }
    }

    Ok(StatusReport {
        branch,
        changes: resolve(items),
    })
}

pub(super) fn git_log(root: &Path) -> Result<LogReport, Error> {
    let repository = gix::open(root).map_err(git_err(root))?;
    let head = match repository.head_commit() {
        Ok(commit) => commit,
        Err(error) if super::is_unborn_head(&error) => {
            return Ok(LogReport::default());
        }
        Err(error) => return Err(git_err(root)(error)),
    };

    let mut entries = Vec::new();
    for info in repository
        .rev_walk([head.id])
        .first_parent_only()
        .all()
        .map_err(git_err(root))?
        .take(RECENT_LIMIT)
    {
        let info = info.map_err(git_err(root))?;
        let commit = info.object().map_err(git_err(root))?;
        let message = commit.message().map_err(git_err(root))?;
        entries.push(LogEntry {
            short_id: info.id.to_hex_with_len(12).to_string(),
            subject: message.summary().to_string(),
        });
    }

    Ok(LogReport { entries })
}

const fn jj_change_type(entry: &snapshot::DiffEntry) -> Option<ChangeType> {
    match (entry.before_present, entry.after_present) {
        (false, true) => Some(ChangeType::Added),
        (true, false) => Some(ChangeType::Deleted),
        (true, true) => Some(ChangeType::Modified),
        (false, false) => None,
    }
}

pub(super) fn jj_status(root: &Path) -> Result<StatusReport, Error> {
    use std::collections::BTreeMap;

    let Some(snapshot) = snapshot::working_copy_diff(root)? else {
        return Ok(StatusReport::default());
    };

    let mut by_path: BTreeMap<String, ChangeType> = BTreeMap::new();
    for entry in &snapshot.changes {
        if let Some(change_type) = jj_change_type(entry) {
            by_path.insert(entry.path.clone(), change_type);
        }
    }
    // A merge conflict cancels in the change diff (the same conflict sits in
    // both trees), so it is read from the snapshot tree and unioned in — a path
    // both changed and conflicted is listed once, as conflicted.
    for (path, _value) in snapshot.tree.conflicts() {
        by_path.insert(
            path.as_internal_file_string().to_owned(),
            ChangeType::Conflicted,
        );
    }

    let changes = by_path
        .into_iter()
        .map(|(path, change_type)| FileChange { change_type, path })
        .collect();

    Ok(StatusReport {
        branch: snapshot.branch,
        changes,
    })
}

pub(super) fn jj_log(root: &Path) -> Result<LogReport, Error> {
    let Some(head) = snapshot::head_commit(root)? else {
        return Ok(LogReport::default());
    };

    let store = head.repo.store();
    let root_commit_id = store.root_commit_id().clone();
    let mut current = head.commit.parent_ids().first().cloned();
    let mut entries = Vec::new();
    while let Some(id) = current {
        if id == root_commit_id {
            break;
        }
        let commit = store.get_commit(&id).map_err(jj_err(root))?;
        let change_id = commit.change_id().reverse_hex();
        entries.push(LogEntry {
            short_id: change_id.get(..12).unwrap_or(&change_id).to_owned(),
            subject: commit
                .description()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned(),
        });
        if entries.len() == RECENT_LIMIT {
            break;
        }
        current = commit.parent_ids().first().cloned();
    }

    Ok(LogReport { entries })
}

#[cfg(test)]
mod tests {
    use vcs::status::ChangeType;
    use vcs::status::FileChange;

    use super::classify;
    use super::resolve;
    use super::Observation;
    use super::RawChange;
    use super::Source;

    fn raw(source: Source, observation: Observation, path: &str) -> RawChange {
        RawChange {
            source,
            observation,
            path: path.to_owned(),
        }
    }

    fn change(change_type: ChangeType, path: &str) -> FileChange {
        FileChange {
            change_type,
            path: path.to_owned(),
        }
    }

    #[test]
    fn an_untracked_observation_classifies_as_untracked() {
        assert_eq!(
            classify(&raw(Source::Worktree, Observation::Untracked, "a")),
            vec![change(ChangeType::Untracked, "a")]
        );
    }

    #[test]
    fn a_staged_add_classifies_as_added() {
        assert_eq!(
            classify(&raw(Source::Staged, Observation::Added, "a")),
            vec![change(ChangeType::Added, "a")]
        );
    }

    #[test]
    fn a_removal_classifies_as_deleted() {
        assert_eq!(
            classify(&raw(Source::Staged, Observation::Removed, "a")),
            vec![change(ChangeType::Deleted, "a")]
        );
    }

    #[test]
    fn a_modification_classifies_as_modified() {
        assert_eq!(
            classify(&raw(Source::Worktree, Observation::Modified, "a")),
            vec![change(ChangeType::Modified, "a")]
        );
    }

    #[test]
    fn a_conflict_classifies_as_conflicted() {
        assert_eq!(
            classify(&raw(Source::Worktree, Observation::Conflicted, "a")),
            vec![change(ChangeType::Conflicted, "a")]
        );
    }

    #[test]
    fn a_rename_classifies_as_deleted_old_plus_added_new() {
        assert_eq!(
            classify(&raw(
                Source::Staged,
                Observation::Renamed {
                    old: "old.txt".to_owned()
                },
                "new.txt"
            )),
            vec![
                change(ChangeType::Deleted, "old.txt"),
                change(ChangeType::Added, "new.txt"),
            ]
        );
    }

    #[test]
    fn a_copy_classifies_as_added_new_only() {
        assert_eq!(
            classify(&raw(Source::Staged, Observation::Copied, "copy.txt")),
            vec![change(ChangeType::Added, "copy.txt")]
        );
    }

    #[test]
    fn staged_add_over_worktree_modify_resolves_to_added() {
        assert_eq!(
            resolve(vec![
                (Source::Staged, change(ChangeType::Added, "a")),
                (Source::Worktree, change(ChangeType::Modified, "a")),
            ]),
            vec![change(ChangeType::Added, "a")]
        );
    }

    #[test]
    fn staged_add_over_worktree_delete_resolves_to_added() {
        assert_eq!(
            resolve(vec![
                (Source::Staged, change(ChangeType::Added, "a")),
                (Source::Worktree, change(ChangeType::Deleted, "a")),
            ]),
            vec![change(ChangeType::Added, "a")]
        );
    }

    #[test]
    fn staged_modify_over_worktree_delete_resolves_to_modified() {
        assert_eq!(
            resolve(vec![
                (Source::Staged, change(ChangeType::Modified, "a")),
                (Source::Worktree, change(ChangeType::Deleted, "a")),
            ]),
            vec![change(ChangeType::Modified, "a")]
        );
    }

    #[test]
    fn a_staged_delete_over_an_untracked_file_resolves_to_deleted() {
        assert_eq!(
            resolve(vec![
                (Source::Staged, change(ChangeType::Deleted, "a")),
                (Source::Worktree, change(ChangeType::Untracked, "a")),
            ]),
            vec![change(ChangeType::Deleted, "a")]
        );
    }

    #[test]
    fn an_untracked_only_path_resolves_to_untracked() {
        assert_eq!(
            resolve(vec![(
                Source::Worktree,
                change(ChangeType::Untracked, "a")
            )]),
            vec![change(ChangeType::Untracked, "a")]
        );
    }

    #[test]
    fn a_conflict_overrides_a_staged_type_on_the_same_path() {
        assert_eq!(
            resolve(vec![
                (Source::Staged, change(ChangeType::Modified, "a")),
                (Source::Worktree, change(ChangeType::Conflicted, "a")),
            ]),
            vec![change(ChangeType::Conflicted, "a")]
        );
    }
}
