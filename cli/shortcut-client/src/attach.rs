//! Attaching a link to a story through `external_links_add`.
//!
//! The `PATCH` appends, so it is sent again only after a 429.

use serde_json::json;
use serde_json::Value;

use crate::classify::Operation;
use crate::shortcut_key::ShortcutKey;
use crate::shortcut_key::StoryId;
use crate::transport::Resend;
use crate::writes::WriteError;
use crate::Method;
use crate::ShortcutClient;
use crate::Url;

impl ShortcutClient {
    /// # Errors
    ///
    /// [`WriteError::NotHttps`] for any URL but an https one, sent nowhere,
    /// and [`WriteError::Tracker`] for a failed or refused send.
    pub fn attach_link(
        &self,
        story: StoryId,
        url: &str,
    ) -> Result<Value, WriteError> {
        let https = Url::parse(url).is_ok_and(|parsed| {
            parsed.scheme() == "https" && parsed.has_host()
        });
        if !https {
            return Err(WriteError::NotHttps(url.to_owned()));
        }
        self.write(
            Operation::Attach,
            &Method::PATCH,
            &self.paths().entity(ShortcutKey::Story(story)),
            &json!({"external_links_add": [url]}),
            Resend::OnlyWhenThrottled,
        )
        .map_err(WriteError::Tracker)
    }
}
