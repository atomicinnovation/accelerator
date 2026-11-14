//! Adding a comment to a story or an epic.
//!
//! A comment `POST` is sent again only after a 429: a 5xx or a lost answer
//! may already have added it, and a second send would add it twice.

use serde_json::json;
use serde_json::Value;

use crate::classify::Operation;
use crate::shortcut_key::ShortcutKey;
use crate::transport::Resend;
use crate::writes::WriteError;
use crate::Method;
use crate::ShortcutClient;

impl ShortcutClient {
    /// # Errors
    ///
    /// [`WriteError::Tracker`] for a failed or refused send.
    pub fn add_comment(
        &self,
        key: ShortcutKey,
        body: &str,
    ) -> Result<Value, WriteError> {
        self.write(
            Operation::Comment,
            &Method::POST,
            &self.paths().comments(key),
            &json!({"text": body}),
            Resend::OnlyWhenThrottled,
        )
        .map_err(WriteError::Tracker)
    }
}
