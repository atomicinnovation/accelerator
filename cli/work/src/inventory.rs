//! Which listed work files are work items, and which a sync cannot use: one
//! pass over the listing, so the identified items and the inventory of what
//! is left can never disagree.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::identity::ItemIdentity;
use crate::work_item_files::identity_of;
use crate::work_item_files::ReadFailure;
use crate::work_item_files::WorkItemFile;
use crate::work_item_files::WorkItemListing;

/// What is wrong when a work file cannot be used: the environment, which a
/// retry after fixing it overcomes, or the file's content, which stays wrong
/// until someone edits it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Environment,
    Content,
}

/// How a file that names a work item fails to be one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MalformedCause {
    UnclosedFence,
    NoFenceButNamed(String),
    NoIdButNamed(String),
    ConflictMarkers,
}

impl std::fmt::Display for MalformedCause {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnclosedFence => {
                formatter.write_str("frontmatter opened but never closed")
            }
            Self::NoFenceButNamed(id) => write!(
                formatter,
                "no frontmatter, but the filename names work item {id}"
            ),
            Self::NoIdButNamed(id) => write!(
                formatter,
                "frontmatter has no id, but the filename names work item {id}"
            ),
            Self::ConflictMarkers => {
                formatter.write_str("contains VCS conflict markers")
            }
        }
    }
}

/// Why a listed file is not one of the corpus's work items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnidentifiedReason {
    Unreadable(String),
    Undecodable,
    Malformed(MalformedCause),
    NoFrontmatter,
    NoId,
}

impl UnidentifiedReason {
    /// What is wrong, or `None` for a file that is simply not a work item.
    #[must_use]
    pub const fn fault(&self) -> Option<Fault> {
        match self {
            Self::Unreadable(_) => Some(Fault::Environment),
            Self::Undecodable | Self::Malformed(_) => Some(Fault::Content),
            Self::NoFrontmatter | Self::NoId => None,
        }
    }

    /// How a report names the reason.
    #[must_use]
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::Unreadable(_) => "unreadable",
            Self::Undecodable => "undecodable",
            Self::Malformed(_) => "malformed",
            Self::NoFrontmatter => "no-frontmatter",
            Self::NoId => "no-id",
        }
    }
}

impl std::fmt::Display for UnidentifiedReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable(detail) => formatter.write_str(detail),
            Self::Undecodable => formatter.write_str("not UTF-8"),
            Self::Malformed(cause) => cause.fmt(formatter),
            Self::NoFrontmatter => formatter.write_str("no frontmatter"),
            Self::NoId => formatter.write_str("frontmatter has no id"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unidentified {
    pub path: PathBuf,
    pub reason: UnidentifiedReason,
}

/// The ids of the corpus's work items, and every listed file that is not
/// one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CorpusInventory {
    pub ids: BTreeSet<String>,
    pub unidentified: Vec<Unidentified>,
}

impl CorpusInventory {
    /// An inventory of `ids` with nothing unidentified.
    #[must_use]
    pub fn complete(ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            ids: ids.into_iter().collect(),
            unidentified: Vec::new(),
        }
    }

    /// Whether every work file was identified or is plainly not a work
    /// item, so the corpus's `external_id`s are all known.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.blocking_fault().is_none()
    }

    /// Whether an id missing from the inventory is missing from the
    /// corpus: an empty inventory proves nothing.
    #[must_use]
    pub fn proves_absence(&self) -> bool {
        self.is_complete() && !self.ids.is_empty()
    }

    /// What keeps the inventory incomplete: the content when any blocking
    /// file is malformed or undecodable, else the environment.
    #[must_use]
    pub fn blocking_fault(&self) -> Option<Fault> {
        self.unidentified
            .iter()
            .filter_map(|entry| entry.reason.fault())
            .max_by_key(|fault| matches!(fault, Fault::Content))
    }
}

/// Whether a filename names a work item, and which.
pub trait NamesWorkItem {
    fn work_item_named(&self, file_name: &str) -> Option<String>;
}

/// The paths the version control system reports as conflicted.
pub trait ConflictedPaths {
    fn is_conflicted(&self, path: &Path) -> bool;
}

/// The listing split into the work items it holds and an inventory of
/// everything else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentifiedCorpus {
    pub items: Vec<(WorkItemFile, ItemIdentity)>,
    pub inventory: CorpusInventory,
}

#[must_use]
pub fn identify(
    listing: &WorkItemListing,
    names: &dyn NamesWorkItem,
    conflicted: &dyn ConflictedPaths,
) -> IdentifiedCorpus {
    let mut corpus = IdentifiedCorpus::default();
    for unreadable in &listing.unreadable {
        corpus.inventory.unidentified.push(Unidentified {
            path: unreadable.path.clone(),
            reason: match &unreadable.reason {
                ReadFailure::Undecodable => UnidentifiedReason::Undecodable,
                ReadFailure::Io(detail) => {
                    UnidentifiedReason::Unreadable(detail.clone())
                }
            },
        });
    }
    for file in &listing.files {
        match identified(file, names, conflicted) {
            Ok(identity) => {
                corpus.inventory.ids.insert(identity.id.clone());
                corpus.items.push((file.clone(), identity));
            }
            Err(reason) => corpus.inventory.unidentified.push(Unidentified {
                path: file.path.clone(),
                reason,
            }),
        }
    }
    corpus
        .inventory
        .unidentified
        .sort_by(|left, right| left.path.cmp(&right.path));
    corpus
}

fn identified(
    file: &WorkItemFile,
    names: &dyn NamesWorkItem,
    conflicted: &dyn ConflictedPaths,
) -> Result<ItemIdentity, UnidentifiedReason> {
    if conflicted.is_conflicted(&file.path)
        || holds_conflict_markers(&file.content)
    {
        return Err(UnidentifiedReason::Malformed(
            MalformedCause::ConflictMarkers,
        ));
    }
    let name_id = file
        .path
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(|name| names.work_item_named(name));
    if !opens_frontmatter(&file.content) {
        return Err(name_id.map_or(UnidentifiedReason::NoFrontmatter, |id| {
            UnidentifiedReason::Malformed(MalformedCause::NoFenceButNamed(id))
        }));
    }
    if let Some(identity) = identity_of(file) {
        return Ok(identity);
    }
    if !closes_frontmatter(&file.content) {
        return Err(UnidentifiedReason::Malformed(
            MalformedCause::UnclosedFence,
        ));
    }
    Err(name_id.map_or(UnidentifiedReason::NoId, |id| {
        UnidentifiedReason::Malformed(MalformedCause::NoIdButNamed(id))
    }))
}

fn is_frontmatter_fence(line: &str) -> bool {
    line.strip_prefix("---")
        .is_some_and(|rest| rest.trim().is_empty())
}

fn opens_frontmatter(content: &str) -> bool {
    content.lines().next().is_some_and(is_frontmatter_fence)
}

fn closes_frontmatter(content: &str) -> bool {
    content.lines().skip(1).any(is_frontmatter_fence)
}

/// A line-start run of seven or more of one marker character, followed by
/// nothing or a space.
fn marker_run(line: &str) -> Option<(char, usize)> {
    let first = line.chars().next()?;
    if !matches!(first, '<' | '=' | '>' | '%' | '+' | '|') {
        return None;
    }
    let length = line.chars().take_while(|&c| c == first).count();
    let rest = &line[length..];
    (length >= 7 && (rest.is_empty() || rest.starts_with(' ')))
        .then_some((first, length))
}

/// The line ranges of every complete conflict block: an opening `<` run, a
/// middle run (`=` or `|` for git, `%` or `+` for jj), then a closing `>`
/// run, every run the same length.
fn conflict_blocks(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(('<', length)) = marker_run(lines[index]) else {
            index += 1;
            continue;
        };
        let mut middle = false;
        let mut close = None;
        for (offset, line) in lines.iter().enumerate().skip(index + 1) {
            match marker_run(line) {
                Some(('<', _)) => break,
                Some(('=' | '|' | '%' | '+', run)) if run == length => {
                    middle = true;
                }
                Some(('>', run)) if run == length && middle => {
                    close = Some(offset);
                    break;
                }
                _ => {}
            }
        }
        match close {
            Some(end) => {
                blocks.push((index, end));
                index = end + 1;
            }
            None => index += 1,
        }
    }
    blocks
}

fn is_code_fence(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

/// Whether `content` holds a complete conflict block outside fenced code.
/// Fences are tracked ignoring every line inside a block, so a conflict
/// whose sides add or remove a fence cannot hide itself.
fn holds_conflict_markers(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let blocks = conflict_blocks(&lines);
    let mut in_fence = false;
    let mut next_block = blocks.iter().peekable();
    let mut index = 0;
    while index < lines.len() {
        if let Some(&&(start, end)) = next_block.peek() {
            if index == start {
                if !in_fence {
                    return true;
                }
                next_block.next();
                index = end + 1;
                continue;
            }
        }
        if is_code_fence(lines[index]) {
            in_fence = !in_fence;
        }
        index += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::identify;
    use super::ConflictedPaths;
    use super::CorpusInventory;
    use super::Fault;
    use super::MalformedCause;
    use super::NamesWorkItem;
    use super::Unidentified;
    use super::UnidentifiedReason;
    use crate::work_item_files::ReadFailure;
    use crate::work_item_files::UnreadableFile;
    use crate::work_item_files::WorkItemFile;
    use crate::work_item_files::WorkItemListing;

    struct NumberedNames;

    impl NamesWorkItem for NumberedNames {
        fn work_item_named(&self, file_name: &str) -> Option<String> {
            let prefix = file_name.split('-').next()?;
            if prefix == "draft" {
                return Some(
                    file_name.split('-').take(2).collect::<Vec<_>>().join("-"),
                );
            }
            (prefix.len() == 4 && prefix.chars().all(|c| c.is_ascii_digit()))
                .then(|| prefix.to_owned())
        }
    }

    struct NoneConflicted;

    impl ConflictedPaths for NoneConflicted {
        fn is_conflicted(&self, _path: &Path) -> bool {
            false
        }
    }

    struct Conflicted(PathBuf);

    impl ConflictedPaths for Conflicted {
        fn is_conflicted(&self, path: &Path) -> bool {
            path == self.0
        }
    }

    fn reason_for(name: &str, content: &str) -> Option<UnidentifiedReason> {
        let listing = WorkItemListing {
            files: vec![WorkItemFile {
                path: PathBuf::from(name),
                content: content.to_owned(),
            }],
            unreadable: Vec::new(),
        };
        identify(&listing, &NumberedNames, &NoneConflicted)
            .inventory
            .unidentified
            .first()
            .map(|entry| entry.reason.clone())
    }

    #[test]
    fn a_well_formed_work_item_is_identified() {
        let listing = WorkItemListing {
            files: vec![WorkItemFile {
                path: PathBuf::from("0001-one.md"),
                content: "---\nid: \"0001\"\n---\n\nBody\n".to_owned(),
            }],
            unreadable: Vec::new(),
        };

        let corpus = identify(&listing, &NumberedNames, &NoneConflicted);

        assert_eq!(corpus.items.len(), 1);
        assert!(corpus.inventory.ids.contains("0001"));
        assert!(corpus.inventory.unidentified.is_empty());
    }

    #[test]
    fn a_well_formed_file_without_an_id_is_no_id() {
        assert_eq!(
            reason_for("README.md", "---\ntitle: Readme\n---\n\nHi\n"),
            Some(UnidentifiedReason::NoId)
        );
    }

    #[test]
    fn an_id_less_file_named_like_a_work_item_is_malformed() {
        assert_eq!(
            reason_for("0002-two.md", "---\ntitle: Two\n---\n\nHi\n"),
            Some(UnidentifiedReason::Malformed(MalformedCause::NoIdButNamed(
                "0002".to_owned()
            )))
        );
    }

    #[test]
    fn a_file_with_an_unclosed_fence_is_malformed() {
        assert_eq!(
            reason_for("README.md", "---\ntitle: Readme\n\nHi\n"),
            Some(UnidentifiedReason::Malformed(MalformedCause::UnclosedFence))
        );
    }

    #[test]
    fn a_fence_less_file_named_like_a_work_item_is_malformed() {
        assert_eq!(
            reason_for("0002-two.md", "# Two\n"),
            Some(UnidentifiedReason::Malformed(
                MalformedCause::NoFenceButNamed("0002".to_owned())
            ))
        );
    }

    #[test]
    fn a_fence_less_file_not_named_like_a_work_item_has_no_frontmatter() {
        assert_eq!(
            reason_for("README.md", "# Readme\n"),
            Some(UnidentifiedReason::NoFrontmatter)
        );
    }

    #[test]
    fn a_fence_less_draft_is_malformed() {
        assert_eq!(
            reason_for("draft-k7mq3x-title.md", "# Draft\n"),
            Some(UnidentifiedReason::Malformed(
                MalformedCause::NoFenceButNamed("draft-k7mq3x".to_owned())
            ))
        );
    }

    #[test]
    fn unreadable_and_undecodable_files_are_inventoried() {
        let listing = WorkItemListing {
            files: Vec::new(),
            unreadable: vec![
                UnreadableFile {
                    path: PathBuf::from("0002-two.md"),
                    reason: ReadFailure::Io("Permission denied".to_owned()),
                },
                UnreadableFile {
                    path: PathBuf::from("0001-one.md"),
                    reason: ReadFailure::Undecodable,
                },
            ],
        };

        let inventory =
            identify(&listing, &NumberedNames, &NoneConflicted).inventory;

        assert_eq!(
            inventory.unidentified,
            vec![
                Unidentified {
                    path: PathBuf::from("0001-one.md"),
                    reason: UnidentifiedReason::Undecodable,
                },
                Unidentified {
                    path: PathBuf::from("0002-two.md"),
                    reason: UnidentifiedReason::Unreadable(
                        "Permission denied".to_owned()
                    ),
                },
            ]
        );
    }

    const GIT_BLOCK: &str =
        "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> branch\n";
    const JJ_BLOCK: &str = "<<<<<<< Conflict 1 of 1\n%%%%%%% Changes from \
                            base to side #1\n-old\n+new\n+++++++ Contents of \
                            side #2\nother\n>>>>>>> Conflict 1 of 1 ends\n";
    const LONG_JJ_BLOCK: &str = "<<<<<<<< Conflict 1 of 1\n%%%%%%%% Changes \
                                 from base to side #1\n-old\n+new\n++++++++ \
                                 Contents of side #2\nother\n>>>>>>>> \
                                 Conflict 1 of 1 ends\n";

    fn item_holding(body: &str) -> String {
        format!("---\nid: \"0001\"\n---\n\n{body}")
    }

    #[test]
    fn conflict_markers_make_a_file_malformed_only_as_a_complete_block() {
        let conflicted = Some(UnidentifiedReason::Malformed(
            MalformedCause::ConflictMarkers,
        ));
        let cases = [
            (GIT_BLOCK.to_owned(), conflicted.clone()),
            (JJ_BLOCK.to_owned(), conflicted.clone()),
            (LONG_JJ_BLOCK.to_owned(), conflicted.clone()),
            (
                "<<<<<<< HEAD\n```\n=======\ntext\n>>>>>>> branch\nmore\n"
                    .to_owned(),
                conflicted,
            ),
            ("<<<<<<< HEAD\nalone\n".to_owned(), None),
            (format!("  {}", GIT_BLOCK.replace('\n', "\n  ")), None),
            (format!("```\n{GIT_BLOCK}```\n"), None),
            (
                "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>>> branch\n"
                    .to_owned(),
                None,
            ),
        ];
        for (body, expected) in cases {
            assert_eq!(
                reason_for("0001-one.md", &item_holding(&body)),
                expected,
                "{body}"
            );
        }
    }

    #[test]
    fn a_conflict_before_the_frontmatter_is_malformed() {
        assert_eq!(
            reason_for(
                "0001-one.md",
                &format!("{JJ_BLOCK}---\nid: \"0001\"\n---\n")
            ),
            Some(UnidentifiedReason::Malformed(
                MalformedCause::ConflictMarkers
            ))
        );
    }

    #[test]
    fn a_fenced_example_of_markers_stays_identifiable() {
        assert_eq!(
            reason_for(
                "0001-one.md",
                &item_holding(&format!(
                    "Example:\n\n```text\n{GIT_BLOCK}```\n"
                ))
            ),
            None
        );
    }

    #[test]
    fn a_vcs_conflicted_file_is_malformed_whatever_its_text() {
        let listing = WorkItemListing {
            files: vec![WorkItemFile {
                path: PathBuf::from("0001-one.md"),
                content: item_holding("Clean text\n"),
            }],
            unreadable: Vec::new(),
        };

        let corpus = identify(
            &listing,
            &NumberedNames,
            &Conflicted(PathBuf::from("0001-one.md")),
        );

        assert!(corpus.items.is_empty());
        assert_eq!(
            corpus.inventory.unidentified[0].reason,
            UnidentifiedReason::Malformed(MalformedCause::ConflictMarkers)
        );
    }

    fn inventory_of(
        reasons: Vec<UnidentifiedReason>,
        ids: &[&str],
    ) -> CorpusInventory {
        CorpusInventory {
            ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            unidentified: reasons
                .into_iter()
                .map(|reason| Unidentified {
                    path: PathBuf::from("x.md"),
                    reason,
                })
                .collect(),
        }
    }

    #[test]
    fn an_inventory_with_an_unreadable_or_malformed_file_cannot_prove_absence()
    {
        for reason in [
            UnidentifiedReason::Unreadable("denied".to_owned()),
            UnidentifiedReason::Undecodable,
            UnidentifiedReason::Malformed(MalformedCause::UnclosedFence),
        ] {
            let inventory = inventory_of(vec![reason], &["0001"]);
            assert!(!inventory.is_complete());
            assert!(!inventory.proves_absence());
        }
    }

    #[test]
    fn an_inventory_with_only_informational_files_proves_absence() {
        let inventory = inventory_of(
            vec![UnidentifiedReason::NoId, UnidentifiedReason::NoFrontmatter],
            &["0001"],
        );

        assert!(inventory.is_complete());
        assert!(inventory.proves_absence());
    }

    #[test]
    fn an_empty_inventory_cannot_prove_absence() {
        assert!(!CorpusInventory::complete(Vec::new()).proves_absence());
    }

    #[test]
    fn an_empty_inventory_is_complete() {
        assert!(CorpusInventory::complete(Vec::new()).is_complete());
    }

    #[test]
    fn blocking_fault_over_unreadable_malformed_undecodable_and_mixed_sets() {
        let unreadable = || UnidentifiedReason::Unreadable("denied".to_owned());
        let malformed =
            || UnidentifiedReason::Malformed(MalformedCause::UnclosedFence);
        for (reasons, fault) in [
            (vec![unreadable()], Some(Fault::Environment)),
            (vec![malformed()], Some(Fault::Content)),
            (vec![UnidentifiedReason::Undecodable], Some(Fault::Content)),
            (vec![unreadable(), malformed()], Some(Fault::Content)),
            (vec![UnidentifiedReason::NoId], None),
        ] {
            assert_eq!(inventory_of(reasons, &[]).blocking_fault(), fault);
        }
    }
}
