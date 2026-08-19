//! Comment creation.
//!
//! Linear comment bodies are Markdown-native, so there is no conversion layer
//! in either direction — the body passes through verbatim.

use serde_json::json;
use serde_json::Value;

use crate::client::LinearClient;
use crate::surface::interpret;
use crate::surface::SurfaceError;

const COMMENT_CREATE: &str = "mutation($input: CommentCreateInput!) {
    commentCreate(input: $input) { success comment { id } }
  }";

impl LinearClient {
    /// Adds a Markdown comment to an issue.
    ///
    /// # Errors
    ///
    /// [`SurfaceError`] for a transport failure, a response carrying
    /// `errors[]`, or a non-JSON response.
    pub fn add_comment(
        &self,
        identifier: &str,
        body: &str,
    ) -> Result<Value, SurfaceError> {
        let variables = json!({
            "input": { "issueId": identifier, "body": body },
        });
        let received = self.transport().send(COMMENT_CREATE, &variables)?;
        interpret(&received, "comment")
    }
}
