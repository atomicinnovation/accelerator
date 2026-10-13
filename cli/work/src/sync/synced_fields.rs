//! The tracker fields sync owns beyond the body.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedTrackerFields {
    pub title: String,
}

impl SyncedTrackerFields {
    /// One `key=<byte length>:value` line per field in key order, so the
    /// serialisation stays injective as fields are added.
    #[must_use]
    pub fn digest_input(&self) -> String {
        format!("title={}:{}\n", self.title.len(), self.title)
    }
}

#[cfg(test)]
mod tests {
    use super::SyncedTrackerFields;

    #[test]
    fn the_digest_input_serialises_fields_in_key_order() {
        let fields = SyncedTrackerFields {
            title: "Add search: phase 2".to_owned(),
        };

        assert_eq!(fields.digest_input(), "title=19:Add search: phase 2\n");
    }
}
