//! The frontmatter fields a work item's create request is built from, read
//! as YAML so a title the renderer escaped reaches the tracker as written.

use document::DocumentError;
use document::Scalar;
use document::Yaml;

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
    let frontmatter = document::parse(content)?;
    let field = |key| match &frontmatter {
        Yaml::Mapping(mapping) => match mapping.get(key) {
            Some(Yaml::Scalar(Scalar::String(value))) => value.clone(),
            _ => String::new(),
        },
        Yaml::Scalar(_) | Yaml::Sequence(_) => String::new(),
    };
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
