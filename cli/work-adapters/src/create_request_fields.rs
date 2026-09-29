//! The frontmatter fields a work item's create request is built from, read
//! as YAML so a title the renderer escaped reaches the tracker as written.

use document::DocumentError;

use crate::frontmatter_strings::FrontmatterStrings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateRequestFields {
    pub title: String,
    pub kind: String,
}

/// `content`'s `title` and `kind`, each empty when absent or not a string.
///
/// # Errors
///
/// [`DocumentError`] when the frontmatter is unterminated or not valid YAML.
pub fn read(content: &str) -> Result<CreateRequestFields, DocumentError> {
    let frontmatter = FrontmatterStrings::parse(content)?;
    let field = |key| frontmatter.get(key).unwrap_or_default().to_owned();
    Ok(CreateRequestFields {
        title: field("title"),
        kind: field("kind"),
    })
}

#[cfg(test)]
mod tests {
    use document::DocumentError;

    use super::read;

    #[test]
    fn escapes_in_a_double_quoted_title_are_decoded(
    ) -> Result<(), DocumentError> {
        let fields = read(
            "---\ntitle: \"Say \\\"hi\\\" to C:\\\\temp\"\nkind: story\n---\n",
        )?;

        assert_eq!(fields.title, r#"Say "hi" to C:\temp"#);
        assert_eq!(fields.kind, "story");
        Ok(())
    }

    #[test]
    fn a_missing_field_reads_empty() -> Result<(), DocumentError> {
        let fields = read("---\nid: x\n---\n")?;

        assert_eq!(fields.title, "");
        assert_eq!(fields.kind, "");
        Ok(())
    }
}
