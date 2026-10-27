//! Relative markdown links between corpus files: spelling one from a
//! directory, and re-basing a file's links when the file moves.

use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

/// How a link written in `dir` spells `target`, both being absolute.
#[must_use]
pub fn link_between(dir: &Path, target: &Path) -> String {
    let dir = normalised(dir);
    let target = normalised(target);
    let shared = dir
        .components()
        .zip(target.components())
        .take_while(|(left, right)| left == right)
        .count();
    let ups = dir.components().count() - shared;
    std::iter::repeat_n("..".to_owned(), ups)
        .chain(
            target
                .components()
                .skip(shared)
                .map(|part| part.as_os_str().to_string_lossy().into_owned()),
        )
        .collect::<Vec<_>>()
        .join("/")
}

/// `content` with every relative link target, written as seen from
/// `from_dir`, respelled as seen from `to_dir`.
#[must_use]
pub fn rebased(content: &str, from_dir: &Path, to_dir: &Path) -> String {
    let move_between = Move { from_dir, to_dir };
    content
        .split_inclusive('\n')
        .map(|line| {
            reference_definition_target(line).map_or_else(
                || move_between.inline_links_in(line),
                |target| move_between.respelled_within(line, target),
            )
        })
        .collect()
}

struct Move<'a> {
    from_dir: &'a Path,
    to_dir: &'a Path,
}

impl Move<'_> {
    fn inline_links_in(&self, line: &str) -> String {
        let mut rebased = String::with_capacity(line.len());
        let mut rest = line;
        while let Some(opening) = rest.find("](") {
            let target_start = opening + 2;
            let target_len = rest[target_start..]
                .find(|c: char| c == ')' || c.is_whitespace())
                .unwrap_or(rest.len() - target_start);
            let target_end = target_start + target_len;
            rebased.push_str(&rest[..target_start]);
            rebased.push_str(&self.target(&rest[target_start..target_end]));
            rest = &rest[target_end..];
        }
        rebased.push_str(rest);
        rebased
    }

    fn respelled_within(&self, line: &str, target: (usize, usize)) -> String {
        let (start, end) = target;
        format!(
            "{}{}{}",
            &line[..start],
            self.target(&line[start..end]),
            &line[end..]
        )
    }

    fn target(&self, target: &str) -> String {
        if !is_relative(target) {
            return target.to_owned();
        }
        let (path, fragment) = target
            .find('#')
            .map_or((target, ""), |hash| target.split_at(hash));
        format!(
            "{}{fragment}",
            link_between(self.to_dir, &self.from_dir.join(path))
        )
    }
}

fn is_relative(target: &str) -> bool {
    let names_a_scheme = target
        .split('/')
        .next()
        .is_some_and(|first| first.contains(':'));
    !target.is_empty()
        && !target.starts_with(['#', '/', '<'])
        && !names_a_scheme
}

/// The byte range of a reference definition's target, as in
/// `[label]: target`.
fn reference_definition_target(line: &str) -> Option<(usize, usize)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 || !line[indent..].starts_with('[') {
        return None;
    }
    let after_label = indent + line[indent..].find("]:")? + 2;
    let rest = &line[after_label..];
    let start = after_label + (rest.len() - rest.trim_start().len());
    let len = line[start..]
        .find(char::is_whitespace)
        .unwrap_or(line.len() - start);
    (len > 0).then_some((start, start + len))
}

fn normalised(path: &Path) -> PathBuf {
    let mut normalised = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalised.pop();
            }
            other => normalised.push(other),
        }
    }
    normalised
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::link_between;
    use super::rebased;

    const WORK: &str = "/repo/meta/work";
    const DRAFTS: &str = "/repo/meta/work/drafts";

    #[test]
    fn a_sibling_is_spelled_by_name() {
        assert_eq!(
            link_between(Path::new(WORK), Path::new("/repo/meta/work/a.md")),
            "a.md"
        );
    }

    #[test]
    fn a_file_up_and_across_is_spelled_through_its_shared_ancestor() {
        assert_eq!(
            link_between(Path::new(DRAFTS), Path::new("/repo/meta/plans/p.md")),
            "../../plans/p.md"
        );
    }

    #[test]
    fn an_inline_link_moving_down_a_directory_climbs_out_of_it() {
        assert_eq!(
            rebased(
                "[a](a.md) and [b](b.md)\n",
                Path::new(WORK),
                Path::new(DRAFTS)
            ),
            "[a](../a.md) and [b](../b.md)\n"
        );
    }

    #[test]
    fn an_inline_link_moving_up_a_directory_drops_its_climb() {
        assert_eq!(
            rebased("[a](../a.md)", Path::new(DRAFTS), Path::new(WORK)),
            "[a](a.md)"
        );
    }

    #[test]
    fn a_fragment_and_a_title_survive_rebasing() {
        assert_eq!(
            rebased(
                "[a](a.md#scope \"A\")",
                Path::new(WORK),
                Path::new(DRAFTS)
            ),
            "[a](../a.md#scope \"A\")"
        );
    }

    #[test]
    fn a_reference_definition_is_rebased() {
        assert_eq!(
            rebased("[a]: a.md \"A\"\n", Path::new(WORK), Path::new(DRAFTS)),
            "[a]: ../a.md \"A\"\n"
        );
    }

    #[test]
    fn anchors_absolute_paths_and_urls_are_left_alone() {
        let content = "[a](#scope) [b](/abs.md) [c](https://x.io/c.md) \
                       [d](work-item:0042) [e](<a b.md>)";

        assert_eq!(
            rebased(content, Path::new(WORK), Path::new(DRAFTS)),
            content
        );
    }
}
