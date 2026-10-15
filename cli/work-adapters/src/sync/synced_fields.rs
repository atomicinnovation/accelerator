//! The tracker fields sync owns, read from either side.

use document::DocumentError;
use remote_projection::split_projected;
use work::sync::SyncedTrackerFields;

use crate::create_request_fields;

/// The fields a push of `content` sends.
///
/// # Errors
///
/// [`DocumentError`] when the frontmatter is unterminated or not valid YAML.
pub fn local_fields(
    content: &str,
) -> Result<SyncedTrackerFields, DocumentError> {
    Ok(SyncedTrackerFields {
        title: create_request_fields::read(content)?.title,
    })
}

/// The fields a tracker's projected body carries.
#[must_use]
pub fn remote_fields(projected: &str) -> SyncedTrackerFields {
    SyncedTrackerFields {
        title: split_projected(projected).title.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use document::DocumentError;
    use work::sync::SyncedTrackerFields;

    use super::local_fields;
    use super::remote_fields;

    fn titled(title: &str) -> SyncedTrackerFields {
        SyncedTrackerFields {
            title: title.to_owned(),
        }
    }

    #[test]
    fn local_fields_read_the_frontmatter_title() -> Result<(), DocumentError> {
        assert_eq!(
            local_fields(
                "---\ntitle: Add search\nstatus: ready\n---\n\nBody\n"
            )?,
            titled("Add search")
        );
        Ok(())
    }

    #[test]
    fn a_quoted_title_reads_as_the_title_the_push_sends(
    ) -> Result<(), DocumentError> {
        assert_eq!(
            local_fields("---\ntitle: \"Say \\\"hi\\\": now\"\n---\n")?,
            titled(r#"Say "hi": now"#)
        );
        Ok(())
    }

    #[test]
    fn remote_fields_read_the_split_projected_title() {
        assert_eq!(
            remote_fields("Add search\n# Heading\n"),
            titled("Add search")
        );
    }

    #[test]
    fn an_absent_title_reads_as_empty() -> Result<(), DocumentError> {
        assert_eq!(local_fields("---\nid: x\n---\n")?, titled(""));
        assert_eq!(remote_fields(""), titled(""));
        Ok(())
    }
}
