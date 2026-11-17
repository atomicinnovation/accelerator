//! Whether every work-item file can be read for the IDs it holds, which
//! commands that read or write IDs need before trusting the corpus.

use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use std::path::PathBuf;

use crate::draft_id::DraftId;
use crate::id_pattern::escaped;
use crate::id_scheme::IdScheme;
use crate::shown_path::relative_to;
use crate::work_item_files::frontmatter_of;
use crate::work_item_files::identity_of;
use crate::work_item_files::list_field;
use crate::work_item_files::nonempty_field;
use crate::work_item_files::ReadFailure;
use crate::work_item_files::WorkItemFile;
use crate::work_item_files::WorkItemListing;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Malformation {
    /// Its filename does not start with its `id` and a slug; `slug` is the
    /// one its filename already has after another ID.
    Misnamed { id: String, slug: Option<String> },
    /// It names an ID in `aliases` or `external_id` but has no `id`.
    NoId,
    /// Its whole filename is an ID, with no slug, and it has no `id`.
    NamedOnlyById { id: String },
    /// Its bytes are not UTF-8, so none of its IDs can be read.
    Undecodable,
}

/// A work-item file whose IDs cannot be read as the corpus needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedItem {
    path: PathBuf,
    malformation: Malformation,
}

impl MalformedItem {
    #[must_use]
    pub const fn malformation(&self) -> &Malformation {
        &self.malformation
    }

    fn what(&self) -> String {
        let path = escaped(&self.path.to_string_lossy());
        match &self.malformation {
            Malformation::Misnamed { id, .. } => format!(
                "{path} has `id` '{}' but its filename does not start with \
                 its `id` and a slug",
                escaped(id),
            ),
            Malformation::NoId => format!(
                "{path} names an ID in `aliases` or `external_id` but has no \
                 `id`"
            ),
            Malformation::NamedOnlyById { id } => format!(
                "{path} is named only by the ID '{}', with no slug and no `id`",
                escaped(id),
            ),
            Malformation::Undecodable => format!("{path} is not UTF-8 text"),
        }
    }

    fn remedy(&self) -> String {
        match &self.malformation {
            Malformation::Misnamed { id, slug } => format!(
                "Rename it to `{}-{}.md`",
                escaped(id),
                slug.as_deref().map_or_else(|| "<slug>".to_owned(), escaped)
            ),
            Malformation::NoId => {
                "Give it the `id` its filename starts with".to_owned()
            }
            Malformation::NamedOnlyById { id } => {
                let id = escaped(id);
                format!(
                    "Give it `id: \"{id}\"` and rename it to `{id}-<slug>.md`, \
                     or move it out of the work directory"
                )
            }
            Malformation::Undecodable => {
                "Re-save it as UTF-8, or move it out of the work directory"
                    .to_owned()
            }
        }
    }
}

/// The work-item files whose IDs cannot be read, in path order, which
/// commands that read or write IDs refuse to run over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedItems {
    items: Vec<MalformedItem>,
}

impl MalformedItems {
    pub const CODE: &'static str = "E_WORK_ITEM_MALFORMED";

    #[must_use]
    pub fn first(&self) -> &MalformedItem {
        &self.items[0]
    }

    #[must_use]
    pub fn items(&self) -> &[MalformedItem] {
        &self.items
    }

    #[must_use]
    pub fn relative_to(self, root: &Path) -> Self {
        Self {
            items: self
                .items
                .into_iter()
                .map(|item| MalformedItem {
                    path: relative_to(item.path, root),
                    ..item
                })
                .collect(),
        }
    }
}

impl Display for MalformedItems {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if let [only] = self.items.as_slice() {
            return write!(
                formatter,
                "{}: {}. {}, then re-run this command",
                Self::CODE,
                only.what(),
                only.remedy(),
            );
        }
        write!(
            formatter,
            "{}: {} work items cannot be read for their IDs. Fix each one \
             as shown, then re-run this command:",
            Self::CODE,
            self.items.len(),
        )?;
        for item in &self.items {
            write!(formatter, "\n    {}. {}", item.what(), item.remedy())?;
        }
        Ok(())
    }
}

impl std::error::Error for MalformedItems {}

/// Refuses every work item whose IDs cannot be read.
///
/// That is one whose filename does not hold its `id` before a slug, as the
/// scheme reads both; one naming an ID with no `id`; one named only by an
/// ID; and one that is not UTF-8. Any other file with no frontmatter, or
/// naming no ID, is not a work item and passes.
///
/// # Errors
///
/// The [`MalformedItems`] naming those files in path order.
pub fn refuse_malformed(
    scheme: &IdScheme,
    listing: &WorkItemListing,
) -> Result<(), MalformedItems> {
    let undecodable = listing
        .unreadable
        .iter()
        .filter(|file| file.reason == ReadFailure::Undecodable)
        .map(|file| MalformedItem {
            path: file.path.clone(),
            malformation: Malformation::Undecodable,
        });
    let mut items: Vec<MalformedItem> = listing
        .files
        .iter()
        .filter_map(|file| malformation_of(scheme, file))
        .chain(undecodable)
        .collect();
    if items.is_empty() {
        return Ok(());
    }
    items.sort_by(|left, right| left.path.cmp(&right.path));
    Err(MalformedItems { items })
}

fn malformation_of(
    scheme: &IdScheme,
    file: &WorkItemFile,
) -> Option<MalformedItem> {
    let malformed = |malformation| MalformedItem {
        path: file.path.clone(),
        malformation,
    };
    match identity_of(file) {
        Some(identity)
            if !names_with_slug(scheme, &file.path, &identity.id) =>
        {
            Some(malformed(Malformation::Misnamed {
                id: identity.id,
                slug: slug_after_an_id(scheme, &file.path),
            }))
        }
        Some(_) => None,
        None if names_an_id(file) => Some(malformed(Malformation::NoId)),
        None => id_naming(scheme, &file.path)
            .map(|id| malformed(Malformation::NamedOnlyById { id })),
    }
}

fn slug_after_an_id(scheme: &IdScheme, path: &Path) -> Option<String> {
    use corpus::IdConvention as _;
    let stem = path.file_stem()?.to_str()?;
    let start = scheme.slug_start(stem)?;
    stem.get(start..)
        .filter(|slug| !slug.is_empty())
        .map(str::to_owned)
}

fn id_naming(scheme: &IdScheme, path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    (scheme.read_held(stem).is_some() || DraftId::parse(stem).is_some())
        .then(|| stem.to_owned())
}

fn names_with_slug(scheme: &IdScheme, path: &Path, id: &str) -> bool {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    holds_through_scheme(scheme, &stem, id) || holds_as_written(&stem, id)
}

fn holds_through_scheme(scheme: &IdScheme, stem: &str, id: &str) -> bool {
    use corpus::IdConvention as _;
    let has_slug = scheme
        .slug_start(stem)
        .is_some_and(|start| start < stem.len());
    has_slug
        && scheme
            .id_in_filename(stem)
            .is_some_and(|held| scheme.identity_key(id) == Some(held))
}

fn holds_as_written(stem: &str, id: &str) -> bool {
    stem.to_ascii_lowercase()
        .strip_prefix(&id.to_ascii_lowercase())
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|slug| !slug.is_empty())
}

fn names_an_id(file: &WorkItemFile) -> bool {
    frontmatter_of(&file.content).is_some_and(|frontmatter| {
        !list_field(frontmatter, "aliases").is_empty()
            || nonempty_field(frontmatter, "external_id").is_some()
    })
}

#[cfg(test)]
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::refuse_malformed;
    use super::Malformation;
    use super::MalformedItems;
    use crate::id_pattern::IdKey;
    use crate::id_pattern::IdPattern;
    use crate::id_scheme::IdScheme;
    use crate::work_item_files::ReadFailure;
    use crate::work_item_files::UnreadableFile;
    use crate::work_item_files::WorkItemFile;
    use crate::work_item_files::WorkItemListing;

    fn malformed_in(
        scheme: &IdScheme,
        files: &[WorkItemFile],
    ) -> Result<(), MalformedItems> {
        refuse_malformed(
            scheme,
            &WorkItemListing {
                files: files.to_vec(),
                unreadable: Vec::new(),
            },
        )
    }

    fn numeric() -> IdScheme {
        IdScheme::numeric()
    }

    fn keyed() -> IdScheme {
        IdScheme::under_any_tracker(
            IdPattern::parse("{key}-{number:04d}")
                .unwrap_or_else(|error| unreachable!("{error}")),
            IdKey::parse("PP").ok(),
        )
    }

    fn file(path: &str, content: &str) -> WorkItemFile {
        WorkItemFile {
            path: PathBuf::from("/repo/meta/work").join(path),
            content: content.to_owned(),
        }
    }

    #[test]
    fn items_named_by_their_id_and_files_that_are_not_items_pass() {
        let files = [
            file("PP-0001-a.md", "---\nid: \"PP-0001\"\n---\n"),
            file(
                "drafts/draft-k7mq3x-b.md",
                "---\nid: \"draft-k7mq3x\"\n---\n",
            ),
            file("README.md", "# Work items\n"),
            file("notes.md", "---\ntitle: \"Notes\"\n---\n"),
        ];
        assert_eq!(malformed_in(&numeric(), &files), Ok(()));
    }

    #[test]
    fn an_item_whose_filename_names_another_id_is_malformed() {
        let refused = malformed_in(
            &numeric(),
            &[file("0400-a.md", "---\nid: \"PP-0400\"\n---\n")],
        );
        assert!(
            matches!(
                &refused,
                Err(refusal)
                    if matches!(
                        refusal.first().malformation(),
                        Malformation::Misnamed { .. }
                    )
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn a_misnamed_items_fix_keeps_the_slug_its_filename_has() {
        let refusal = malformed_in(
            &numeric(),
            &[file("0400-dark-mode.md", "---\nid: \"PP-0400\"\n---\n")],
        )
        .err()
        .unwrap_or_else(|| unreachable!("refused"));

        assert!(
            refusal
                .to_string()
                .contains("Rename it to `PP-0400-dark-mode.md`"),
            "{refusal}"
        );
    }

    #[test]
    fn an_item_without_a_slug_in_its_filename_is_malformed() {
        let refusal = malformed_in(
            &numeric(),
            &[file("PP-0400.md", "---\nid: \"PP-0400\"\n---\n")],
        )
        .err()
        .unwrap_or_else(|| unreachable!("refused"))
        .relative_to(Path::new("/repo"));

        assert_eq!(
            refusal.first().malformation(),
            &Malformation::Misnamed {
                id: "PP-0400".to_owned(),
                slug: None,
            }
        );
        assert_eq!(
            refusal.to_string(),
            "E_WORK_ITEM_MALFORMED: meta/work/PP-0400.md has `id` \
             'PP-0400' but its filename does not start with its `id` and a \
             slug. Rename it to `PP-0400-<slug>.md`, then re-run this command"
        );
    }

    #[test]
    fn a_file_naming_an_id_without_one_is_malformed() {
        for content in [
            "---\naliases: [\"0400\"]\n---\n",
            "---\nexternal_id: \"ENG-7\"\n---\n",
        ] {
            let refusal =
                malformed_in(&numeric(), &[file("PP-0400-a.md", content)])
                    .err()
                    .unwrap_or_else(|| unreachable!("refused"))
                    .relative_to(Path::new("/repo"));

            assert_eq!(refusal.first().malformation(), &Malformation::NoId);
            assert_eq!(
                refusal.to_string(),
                "E_WORK_ITEM_MALFORMED: meta/work/PP-0400-a.md names an ID \
                 in `aliases` or `external_id` but has no `id`. Give it the \
                 `id` its filename starts with, then re-run this command"
            );
        }
    }

    #[test]
    fn every_malformed_item_is_listed_in_path_order_with_its_fix() {
        let refusal = malformed_in(
            &numeric(),
            &[
                file("PP-0402.md", "---\nid: \"PP-0402\"\n---\n"),
                file("PP-0401.md", "---\nid: \"PP-0401\"\n---\n"),
                file("PP-0403-c.md", "---\naliases: [\"0403\"]\n---\n"),
            ],
        )
        .err()
        .unwrap_or_else(|| unreachable!("refused"))
        .relative_to(Path::new("/repo"));

        assert_eq!(
            refusal.to_string(),
            "E_WORK_ITEM_MALFORMED: 3 work items cannot be read for their \
             IDs. Fix each one as shown, then re-run this command:\n    \
             meta/work/PP-0401.md has `id` 'PP-0401' but its filename does \
             not start with its `id` and a slug. Rename it to \
             `PP-0401-<slug>.md`\n    \
             meta/work/PP-0402.md has `id` 'PP-0402' but its filename does \
             not start with its `id` and a slug. Rename it to \
             `PP-0402-<slug>.md`\n    \
             meta/work/PP-0403-c.md names an ID in `aliases` or \
             `external_id` but has no `id`. Give it the `id` its filename \
             starts with"
        );
    }

    #[test]
    fn a_filename_the_scheme_reads_as_the_items_id_is_well_formed() {
        let files = [
            file("PP-42-short.md", "---\nid: \"PP-0042\"\n---\n"),
            file("0043-legacy.md", "---\nid: \"PP-0043\"\n---\n"),
        ];
        assert_eq!(malformed_in(&keyed(), &files), Ok(()));
    }

    #[test]
    fn a_filename_the_scheme_reads_as_no_id_is_malformed() {
        let refused = malformed_in(
            &keyed(),
            &[file("notes-a.md", "---\nid: \"PP-0001\"\n---\n")],
        );
        assert!(
            matches!(
                &refused,
                Err(refusal)
                    if refusal.first().malformation()
                        == &Malformation::Misnamed {
                            id: "PP-0001".to_owned(),
                            slug: None,
                        }
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn a_file_named_only_by_an_id_with_no_id_of_its_own_is_malformed() {
        for content in ["# Notes\n", "---\ntitle: \"Notes\"\n---\n"] {
            let refusal = malformed_in(
                &numeric(),
                &[file("0400.md", content), file("README.md", "# Work\n")],
            )
            .err()
            .unwrap_or_else(|| unreachable!("refused"))
            .relative_to(Path::new("/repo"));

            assert_eq!(
                refusal.to_string(),
                "E_WORK_ITEM_MALFORMED: meta/work/0400.md is named only by \
                 the ID '0400', with no slug and no `id`. Give it `id: \
                 \"0400\"` and rename it to `0400-<slug>.md`, or move it out \
                 of the work directory, then re-run this command"
            );
        }
    }

    #[test]
    fn a_work_item_file_that_is_not_utf8_is_malformed() {
        let listing = WorkItemListing {
            files: Vec::new(),
            unreadable: vec![
                UnreadableFile {
                    path: PathBuf::from("/repo/meta/work/0401-binary.md"),
                    reason: ReadFailure::Undecodable,
                },
                UnreadableFile {
                    path: PathBuf::from("/repo/meta/work/0402-locked.md"),
                    reason: ReadFailure::Io("permission denied".to_owned()),
                },
            ],
        };

        let refusal = refuse_malformed(&numeric(), &listing)
            .err()
            .unwrap_or_else(|| unreachable!("refused"))
            .relative_to(Path::new("/repo"));

        assert_eq!(
            refusal.to_string(),
            "E_WORK_ITEM_MALFORMED: meta/work/0401-binary.md is not UTF-8 \
             text. Re-save it as UTF-8, or move it out of the work \
             directory, then re-run this command"
        );
    }

    #[test]
    fn a_filename_naming_its_id_in_another_case_is_well_formed() {
        assert_eq!(
            malformed_in(
                &numeric(),
                &[file("pp-0001-a.md", "---\nid: \"PP-0001\"\n---\n")]
            ),
            Ok(())
        );
    }
}
