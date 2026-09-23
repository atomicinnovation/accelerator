//! `jj status` and `jj log` as parity oracles: what the real binary reports as
//! changed, and which commits it bases the working copy on.

use std::collections::BTreeSet;
use std::path::Path;

use crate::hermetic::Hermetic;
use crate::Error;

/// The paths on `jj status`'s change lines, both sides of each rename
/// included.
///
/// Snapshots the working copy, so call it only after reading whatever state
/// it is compared against.
///
/// # Errors
///
/// When `jj status` cannot be run or exits non-zero.
pub fn changed_paths(
    env: &Hermetic,
    root: &Path,
) -> Result<BTreeSet<String>, Error> {
    Ok(changed_paths_in(&env.jj(&["status"], root)?))
}

/// The ids of `@`'s parent commits, sorted.
///
/// # Errors
///
/// When `jj log` cannot be run or exits non-zero.
pub fn parent_commit_ids(
    env: &Hermetic,
    root: &Path,
) -> Result<Vec<String>, Error> {
    let listing = env.jj(
        &[
            "log",
            "--no-graph",
            "-r",
            "parents(@)",
            "-T",
            "commit_id ++ \"\\n\"",
        ],
        root,
    )?;
    let mut ids: Vec<String> = listing
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    ids.sort_unstable();
    Ok(ids)
}

/// The run base a migration should record: `@`'s parent commit ids, sorted
/// and joined by `+`.
///
/// # Errors
///
/// When `jj log` cannot be run or exits non-zero.
pub fn run_base_oracle(env: &Hermetic, root: &Path) -> Result<String, Error> {
    Ok(parent_commit_ids(env, root)?.join("+"))
}

const CHANGE_KINDS: [&str; 5] = ["A", "M", "D", "R", "C"];

fn changed_paths_in(status: &str) -> BTreeSet<String> {
    status
        .lines()
        .skip_while(|line| *line != "Working copy changes:")
        .skip(1)
        .map_while(|line| line.split_once(' '))
        .filter(|(kind, _)| CHANGE_KINDS.contains(kind))
        .flat_map(|(_, path)| both_sides(path))
        .collect()
}

fn both_sides(path: &str) -> Vec<String> {
    if let Some((prefix, rest)) = path.split_once('{') {
        if let Some((renamed, suffix)) = rest.split_once('}') {
            if let Some((from, to)) = renamed.split_once(" => ") {
                return vec![
                    format!("{prefix}{from}{suffix}"),
                    format!("{prefix}{to}{suffix}"),
                ];
            }
        }
    }
    match path.split_once(" => ") {
        Some((from, to)) => vec![from.to_owned(), to.to_owned()],
        None => vec![path.to_owned()],
    }
}

#[cfg(test)]
mod tests {
    use super::changed_paths_in;

    fn status(changes: &str) -> String {
        format!(
            "Working copy changes:\n{changes}Working copy  (@) : abc 123 \
             (no description set)\nParent commit (@-): def 456 init\n"
        )
    }

    fn paths(listed: &[&str]) -> Vec<String> {
        listed.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn a_plain_add_is_listed() {
        let listed = changed_paths_in(&status("A meta/new.md\n"));

        assert_eq!(
            listed.into_iter().collect::<Vec<_>>(),
            paths(&["meta/new.md"])
        );
    }

    #[test]
    fn a_braced_rename_lists_both_sides() {
        let listed = changed_paths_in(&status(
            "R meta/{a.md => b.md}\nR meta/{x => y}/c.md\nR {top.md => moved.md}\n",
        ));

        assert_eq!(
            listed.into_iter().collect::<Vec<_>>(),
            paths(&[
                "meta/a.md",
                "meta/b.md",
                "meta/x/c.md",
                "meta/y/c.md",
                "moved.md",
                "top.md",
            ])
        );
    }

    #[test]
    fn an_unbraced_rename_lists_both_sides() {
        let listed = changed_paths_in(&status("R old.md => new.md\n"));

        assert_eq!(
            listed.into_iter().collect::<Vec<_>>(),
            paths(&["new.md", "old.md"])
        );
    }

    #[test]
    fn an_untracked_line_is_ignored() {
        let listed = changed_paths_in(&status("M meta/a.md\n? meta/big.bin\n"));

        assert_eq!(
            listed.into_iter().collect::<Vec<_>>(),
            paths(&["meta/a.md"])
        );
    }

    #[test]
    fn a_status_with_no_changes_lists_nothing() {
        let listed = changed_paths_in(
            "The working copy has no changes.\nWorking copy  (@) : abc 123 \
             (empty) (no description set)\n",
        );

        assert!(listed.is_empty());
    }
}
