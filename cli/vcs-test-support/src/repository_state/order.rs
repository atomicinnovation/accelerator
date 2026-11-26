//! A total topological order over commits that two builds of the same script
//! agree on, whatever IDs the builds gave their commits.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::masking::IdMasking;
use super::CommitPosition;
use super::FileState;
use super::RawCommit;
use super::RawRepository;
use crate::Error;

/// What tells two commits apart when both could take the next position.
///
/// The derived order compares fields in declaration order, which is the
/// order the tie-breaks are consulted in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TieBreak<'r> {
    description: &'r str,
    files: BTreeMap<String, FileState>,
    parents: Vec<CommitPosition>,
    references: Vec<&'r str>,
    is_working_copy_parent: bool,
    is_working_copy: bool,
}

/// Each commit's position, keyed by commit ID.
///
/// Parents come before children. Among the commits whose parents all have
/// positions, the next position goes to the least by [`TieBreak`].
///
/// # Errors
///
/// When a commit names a parent the repository does not hold, the parents
/// form a cycle, or two commits nothing tells apart compete for a position.
pub(super) fn ordered_positions(
    repository: &RawRepository,
) -> Result<BTreeMap<String, CommitPosition>, Error> {
    refuse_unknown_parents(repository)?;
    let references = references_by_commit(repository);
    let working_copy_parents: BTreeSet<&str> = repository
        .working_copy_parents
        .iter()
        .map(String::as_str)
        .collect();
    let mut positions: BTreeMap<String, CommitPosition> = BTreeMap::new();
    while positions.len() < repository.commits.len() {
        let masking = IdMasking::of(&repository.commits, &positions);
        let mut candidates: Vec<(TieBreak<'_>, &RawCommit)> = repository
            .commits
            .iter()
            .filter(|commit| !positions.contains_key(&commit.id))
            .filter_map(|commit| {
                let parents = commit
                    .parents
                    .iter()
                    .map(|parent| positions.get(parent).copied())
                    .collect::<Option<Vec<_>>>()?;
                let tie_break = TieBreak {
                    description: &commit.description,
                    files: commit
                        .files
                        .iter()
                        .map(|(path, file)| {
                            (path.clone(), file.masked(&masking))
                        })
                        .collect(),
                    parents,
                    references: references
                        .get(commit.id.as_str())
                        .cloned()
                        .unwrap_or_default(),
                    is_working_copy_parent: working_copy_parents
                        .contains(commit.id.as_str()),
                    is_working_copy: repository.working_copy_commit.as_ref()
                        == Some(&commit.id),
                };
                Some((tie_break, commit))
            })
            .collect();
        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        let next = match candidates.as_slice() {
            [] => {
                return Err(Error::message(
                    "the commits' parents form a cycle".to_owned(),
                ))
            }
            [(least, first), (runner_up, second), ..] if least == runner_up => {
                return Err(Error::message(format!(
                    "nothing tells commits {} and {} apart, so they have no \
                     order two builds would agree on",
                    first.id, second.id
                )))
            }
            [(_, least), ..] => least,
        };
        positions.insert(next.id.clone(), CommitPosition(positions.len()));
    }
    Ok(positions)
}

fn refuse_unknown_parents(repository: &RawRepository) -> Result<(), Error> {
    let held: BTreeSet<&str> = repository
        .commits
        .iter()
        .map(|commit| commit.id.as_str())
        .collect();
    for commit in &repository.commits {
        if let Some(parent) = commit
            .parents
            .iter()
            .find(|parent| !held.contains(parent.as_str()))
        {
            return Err(Error::message(format!(
                "commit {} names parent {parent}, which the repository does \
                 not hold",
                commit.id
            )));
        }
    }
    Ok(())
}

fn references_by_commit(
    repository: &RawRepository,
) -> BTreeMap<&str, Vec<&str>> {
    let mut by_commit: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let git_references = repository.git.iter().flat_map(|git| &git.references);
    for (name, id) in repository.bookmarks.iter().chain(git_references) {
        by_commit
            .entry(id.as_str())
            .or_default()
            .push(name.as_str());
    }
    by_commit
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::ordered_positions;
    use crate::colocation::JjColocation;
    use crate::repository_state::CommitPosition;
    use crate::repository_state::FileKind;
    use crate::repository_state::FileState;
    use crate::repository_state::RawCommit;
    use crate::repository_state::RawRepository;
    use crate::repository_state::RepositoryKind;

    fn commit(id: &str, parents: &[&str]) -> RawCommit {
        RawCommit {
            id: id.to_owned(),
            change: None,
            parents: parents.iter().map(|&parent| parent.to_owned()).collect(),
            description: String::new(),
            files: BTreeMap::new(),
        }
    }

    fn described(description: &str, raw: RawCommit) -> RawCommit {
        RawCommit {
            description: description.to_owned(),
            ..raw
        }
    }

    fn holding(content: &str, raw: RawCommit) -> RawCommit {
        RawCommit {
            files: BTreeMap::from([(
                "a".to_owned(),
                FileState {
                    kind: FileKind::Regular,
                    content: content.to_owned(),
                },
            )]),
            ..raw
        }
    }

    fn repository(commits: Vec<RawCommit>) -> RawRepository {
        RawRepository {
            kind: RepositoryKind::Jj(JjColocation::Pure),
            commits,
            bookmarks: BTreeMap::new(),
            git: None,
            working_copy_parents: Vec::new(),
            working_copy_commit: None,
        }
    }

    fn referring(
        references: &[(&str, &str)],
        repository: RawRepository,
    ) -> RawRepository {
        RawRepository {
            bookmarks: references
                .iter()
                .map(|&(name, id)| (name.to_owned(), id.to_owned()))
                .collect(),
            ..repository
        }
    }

    fn on_working_copy_parents(
        parents: &[&str],
        repository: RawRepository,
    ) -> RawRepository {
        RawRepository {
            working_copy_parents: parents
                .iter()
                .map(|&parent| parent.to_owned())
                .collect(),
            ..repository
        }
    }

    fn on_working_copy_commit(
        commit: &str,
        repository: RawRepository,
    ) -> RawRepository {
        RawRepository {
            working_copy_commit: Some(commit.to_owned()),
            ..repository
        }
    }

    fn reversed(repository: RawRepository) -> RawRepository {
        RawRepository {
            commits: repository.commits.into_iter().rev().collect(),
            ..repository
        }
    }

    fn order_of(repository: &RawRepository) -> Vec<String> {
        let positions = ordered_positions(repository).unwrap_or_default();
        let mut ordered: Vec<(CommitPosition, String)> = positions
            .into_iter()
            .map(|(id, position)| (position, id))
            .collect();
        ordered.sort();
        ordered.into_iter().map(|(_, id)| id).collect()
    }

    fn ordered_either_way(repository: RawRepository) -> Vec<String> {
        let forwards = order_of(&repository);
        let backwards = order_of(&reversed(repository));
        assert_eq!(forwards, backwards, "the input order changed the order");
        forwards
    }

    #[test]
    fn parents_come_before_children() {
        let order = ordered_either_way(repository(vec![
            commit("child", &["parent"]),
            commit("parent", &[]),
            commit("grandchild", &["child"]),
        ]));

        assert_eq!(order, ["parent", "child", "grandchild"]);
    }

    #[test]
    fn siblings_are_ordered_by_description() {
        let order = ordered_either_way(repository(vec![
            described("b", commit("x", &[])),
            described("a", commit("y", &[])),
        ]));

        assert_eq!(order, ["y", "x"]);
    }

    #[test]
    fn siblings_described_alike_are_ordered_by_their_files() {
        let order = ordered_either_way(repository(vec![
            holding("2", commit("x", &[])),
            holding("1", commit("y", &[])),
        ]));

        assert_eq!(order, ["y", "x"]);
    }

    #[test]
    fn commits_alike_in_content_are_ordered_by_their_parents_positions() {
        let order = ordered_either_way(repository(vec![
            described("first", commit("p1", &[])),
            described("second", commit("p2", &[])),
            described("third", commit("x", &["p2"])),
            described("third", commit("y", &["p1"])),
        ]));

        assert_eq!(order, ["p1", "p2", "y", "x"]);
    }

    #[test]
    fn commits_alike_but_for_references_are_ordered_by_reference_names() {
        let order = ordered_either_way(referring(
            &[("b", "x"), ("a", "y")],
            repository(vec![commit("x", &[]), commit("y", &[])]),
        ));

        assert_eq!(order, ["y", "x"]);
    }

    #[test]
    fn commits_alike_but_for_the_working_copy_are_ordered_by_it() {
        let order = ordered_either_way(on_working_copy_parents(
            &["x"],
            repository(vec![commit("x", &[]), commit("y", &[])]),
        ));

        assert_eq!(order, ["y", "x"]);
    }

    #[test]
    fn commits_alike_but_for_being_the_working_copy_are_ordered_by_it() {
        let order = ordered_either_way(on_working_copy_commit(
            "x",
            repository(vec![commit("x", &[]), commit("y", &[])]),
        ));

        assert_eq!(order, ["y", "x"]);
    }

    #[test]
    fn files_naming_commits_order_by_the_positions_they_name_not_their_ids() {
        let named_first = "ffffffffffffffffffffffffffffffffffffffff";
        let named_second = "0000000000000000000000000000000000000001";
        let order = ordered_either_way(repository(vec![
            described("a", commit(named_first, &[])),
            described("b", commit(named_second, &[])),
            described(
                "c",
                holding(named_first, commit("x", &[named_first, named_second])),
            ),
            described(
                "c",
                holding(
                    named_second,
                    commit("y", &[named_first, named_second]),
                ),
            ),
        ]));

        assert_eq!(order, [named_first, named_second, "x", "y"]);
    }

    #[test]
    fn an_earlier_tie_break_outranks_a_later_one() {
        let order = ordered_either_way(referring(
            &[("a", "x"), ("b", "y")],
            repository(vec![
                described("2", commit("x", &[])),
                described("1", commit("y", &[])),
            ]),
        ));

        assert_eq!(order, ["y", "x"]);
    }

    #[test]
    fn commits_nothing_tells_apart_are_refused_naming_them() {
        let error = ordered_positions(&repository(vec![
            commit("x", &[]),
            commit("y", &[]),
        ]))
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();

        assert!(error.contains('x') && error.contains('y'), "{error}");
    }

    #[test]
    fn a_parent_the_repository_does_not_hold_is_refused() {
        let error =
            ordered_positions(&repository(vec![commit("x", &["gone"])]))
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();

        assert!(error.contains("gone"), "{error}");
    }
}
