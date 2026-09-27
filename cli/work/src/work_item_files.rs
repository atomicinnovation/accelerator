//! The port through which the work-item corpus is read, and the identities
//! parsed from what it yields.

use std::path::PathBuf;

use crate::identity::ItemIdentity;
use crate::show::read_field_raw;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemFile {
    pub path: PathBuf,
    pub content: String,
}

pub trait WorkItemFiles {
    /// Every work-item file, drafts included, sorted by path.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the corpus cannot be listed.
    fn files(&self) -> Result<Vec<WorkItemFile>, kernel::Error>;

    /// The work-item files outside the drafts directory, sorted by path.
    ///
    /// # Errors
    ///
    /// A [`kernel::Error`] when the corpus cannot be listed.
    fn canonical(&self) -> Result<Vec<WorkItemFile>, kernel::Error>;
}

/// The identifying fields of one file, or `None` when it has no closed
/// frontmatter or no non-empty `id` / `work_item_id`.
#[must_use]
pub fn identity_of(file: &WorkItemFile) -> Option<ItemIdentity> {
    let frontmatter = frontmatter_of(&file.content)?;
    let id = nonempty_field(frontmatter, "id")
        .or_else(|| nonempty_field(frontmatter, "work_item_id"))?;
    Some(ItemIdentity {
        path: file.path.clone(),
        id,
        aliases: list_field(frontmatter, "aliases"),
        external_id: nonempty_field(frontmatter, "external_id"),
    })
}

#[must_use]
pub fn identities(files: &[WorkItemFile]) -> Vec<ItemIdentity> {
    files.iter().filter_map(identity_of).collect()
}

fn is_fence(line: &str) -> bool {
    line.strip_prefix("---")
        .is_some_and(|rest| rest.chars().all(char::is_whitespace))
}

fn frontmatter_of(content: &str) -> Option<&str> {
    let after_open = content.strip_prefix("---")?;
    let (open_rest, body) = after_open.split_once('\n')?;
    if !open_rest.trim().is_empty() {
        return None;
    }
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        if is_fence(line.trim_end_matches(['\n', '\r'])) {
            return Some(&body[..offset]);
        }
        offset += line.len();
    }
    None
}

fn nonempty_field(frontmatter: &str, field: &str) -> Option<String> {
    read_field_raw(frontmatter, field).filter(|value| !value.trim().is_empty())
}

fn list_field(frontmatter: &str, field: &str) -> Vec<String> {
    match read_field_raw(frontmatter, field) {
        Some(inline) if !inline.is_empty() => {
            crate::tags::parse_current_tags(&inline)
        }
        Some(_) => block_list_items(frontmatter, field),
        None => Vec::new(),
    }
}

fn block_list_items(frontmatter: &str, field: &str) -> Vec<String> {
    let key = format!("{field}:");
    frontmatter
        .lines()
        .skip_while(|line| line.trim_end() != key)
        .skip(1)
        .map_while(|line| {
            line.starts_with(char::is_whitespace)
                .then(|| line.trim_start().strip_prefix('-'))
                .flatten()
        })
        .map(|item| {
            item.trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_owned()
        })
        .filter(|item| !item.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::identities;
    use super::WorkItemFile;

    fn file(frontmatter: &str) -> WorkItemFile {
        WorkItemFile {
            path: PathBuf::from("meta/work/0001-title.md"),
            content: format!("---\n{frontmatter}---\n\n# Title\n"),
        }
    }

    fn aliases_of(frontmatter: &str) -> Vec<String> {
        identities(&[file(frontmatter)])
            .into_iter()
            .flat_map(|identity| identity.aliases)
            .collect()
    }

    #[test]
    fn aliases_are_read_from_an_inline_list() {
        assert_eq!(
            aliases_of(
                "id: \"ENG-1\"\naliases: [\"draft-k7mq3x\", \"0001\"]\n"
            ),
            vec!["draft-k7mq3x", "0001"]
        );
    }

    #[test]
    fn aliases_are_read_from_a_block_list() {
        assert_eq!(
            aliases_of(
                "id: \"ENG-1\"\naliases:\n  - \"draft-k7mq3x\"\n  - 0001\n\
                 tags: []\n"
            ),
            vec!["draft-k7mq3x", "0001"]
        );
    }

    #[test]
    fn quoted_and_unquoted_values_are_read_alike() {
        let quoted = identities(&[file(
            "id: \"ENG-1\"\naliases: [\"draft-k7mq3x\"]\n\
             external_id: \"ENG-1\"\n",
        )]);
        let unquoted = identities(&[file(
            "id: ENG-1\naliases: [draft-k7mq3x]\nexternal_id: ENG-1\n",
        )]);

        assert_eq!(quoted, unquoted);
        assert_eq!(quoted[0].id, "ENG-1");
        assert_eq!(quoted[0].external_id.as_deref(), Some("ENG-1"));
    }

    #[test]
    fn an_empty_aliases_list_yields_none() {
        assert!(aliases_of("id: \"0001\"\naliases: []\n").is_empty());
        assert!(aliases_of("id: \"0001\"\n").is_empty());
    }

    #[test]
    fn an_empty_external_id_is_no_link() {
        let read = identities(&[file("id: \"0001\"\nexternal_id: \"\"\n")]);

        assert_eq!(read[0].external_id, None);
    }

    #[test]
    fn id_falls_back_to_work_item_id() {
        let read = identities(&[file("work_item_id: \"0001\"\n")]);

        assert_eq!(read[0].id, "0001");
    }

    #[test]
    fn a_file_without_an_id_or_frontmatter_has_no_identity() {
        let no_id = file("title: \"t\"\n");
        let no_frontmatter = WorkItemFile {
            path: PathBuf::from("meta/work/notes.md"),
            content: "# Notes\n".to_owned(),
        };

        assert!(identities(&[no_id, no_frontmatter]).is_empty());
    }
}
