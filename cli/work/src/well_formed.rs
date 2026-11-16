//! Whether every work-item file can be read for the IDs it holds, which
//! commands that read or write IDs need before trusting the corpus.

use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use std::path::PathBuf;

use crate::id_pattern::escaped;
use crate::shown_path::relative_to;
use crate::work_item_files::frontmatter_of;
use crate::work_item_files::identity_of;
use crate::work_item_files::list_field;
use crate::work_item_files::nonempty_field;
use crate::work_item_files::WorkItemFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Malformation {
    /// Its filename does not start with its `id` and a slug.
    NoSlug { id: String },
    /// It names an ID in `aliases` or `external_id` but has no `id`.
    NoId,
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
            Malformation::NoSlug { id } => format!(
                "{path} has `id` '{}' but its filename does not start with \
                 its `id` and a slug",
                escaped(id),
            ),
            Malformation::NoId => format!(
                "{path} names an ID in `aliases` or `external_id` but has no \
                 `id`"
            ),
        }
    }

    fn remedy(&self) -> String {
        match &self.malformation {
            Malformation::NoSlug { id } => {
                format!("Rename it to `{}-<slug>.md`", escaped(id))
            }
            Malformation::NoId => {
                "Give it the `id` its filename starts with".to_owned()
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
/// That is one with an `id` its filename does not start with before a
/// slug, or one naming an ID with no `id`. A file with no frontmatter, or
/// naming no ID, is not a work item and passes.
///
/// # Errors
///
/// The [`MalformedItems`] naming those files in path order.
pub fn refuse_malformed(files: &[WorkItemFile]) -> Result<(), MalformedItems> {
    let mut items: Vec<MalformedItem> =
        files.iter().filter_map(malformation_of).collect();
    if items.is_empty() {
        return Ok(());
    }
    items.sort_by(|left, right| left.path.cmp(&right.path));
    Err(MalformedItems { items })
}

fn malformation_of(file: &WorkItemFile) -> Option<MalformedItem> {
    let malformed = |malformation| MalformedItem {
        path: file.path.clone(),
        malformation,
    };
    match identity_of(file) {
        Some(identity) if !names_with_slug(&file.path, &identity.id) => {
            Some(malformed(Malformation::NoSlug { id: identity.id }))
        }
        Some(_) => None,
        None => names_an_id(file).then(|| malformed(Malformation::NoId)),
    }
}

fn names_with_slug(path: &Path, id: &str) -> bool {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    stem.strip_prefix(&id.to_ascii_lowercase())
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
    use crate::work_item_files::WorkItemFile;

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
        assert_eq!(refuse_malformed(&files), Ok(()));
    }

    #[test]
    fn an_item_whose_filename_names_another_id_is_malformed() {
        let refused = refuse_malformed(&[file(
            "0400-a.md",
            "---\nid: \"PP-0400\"\n---\n",
        )]);
        assert!(
            matches!(
                &refused,
                Err(refusal)
                    if matches!(
                        refusal.first().malformation(),
                        Malformation::NoSlug { .. }
                    )
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn an_item_without_a_slug_in_its_filename_is_malformed() {
        let refusal = refuse_malformed(&[file(
            "PP-0400.md",
            "---\nid: \"PP-0400\"\n---\n",
        )])
        .err()
        .unwrap_or_else(|| unreachable!("refused"))
        .relative_to(Path::new("/repo"));

        assert_eq!(
            refusal.first().malformation(),
            &Malformation::NoSlug {
                id: "PP-0400".to_owned()
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
            let refusal = refuse_malformed(&[file("PP-0400-a.md", content)])
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
        let refusal = refuse_malformed(&[
            file("PP-0402.md", "---\nid: \"PP-0402\"\n---\n"),
            file("PP-0401.md", "---\nid: \"PP-0401\"\n---\n"),
            file("PP-0403-c.md", "---\naliases: [\"0403\"]\n---\n"),
        ])
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
    fn a_filename_naming_its_id_in_another_case_is_well_formed() {
        assert_eq!(
            refuse_malformed(&[file(
                "pp-0001-a.md",
                "---\nid: \"PP-0001\"\n---\n"
            )]),
            Ok(())
        );
    }
}
