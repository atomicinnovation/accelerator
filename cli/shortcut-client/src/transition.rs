//! Moving a story to a named state of its own workflow.

use serde_json::Value;

use crate::classify::Operation;
use crate::shortcut_key::ShortcutKey;
use crate::shortcut_key::StoryId;
use crate::story_state::resolve_story_state;
use crate::transport::Resend;
use crate::update::UpdateFields;
use crate::writes::WriteError;
use crate::Method;
use crate::ShortcutClient;

impl ShortcutClient {
    /// # Errors
    ///
    /// [`WriteError::State`] for a name the story's workflow cannot resolve,
    /// sending nothing, and [`WriteError`] for a failed read or send.
    pub fn transition(
        &self,
        story: StoryId,
        state: &str,
    ) -> Result<Value, WriteError> {
        let workflow = self.story_workflow(story)?;
        let state_id = resolve_story_state(self.catalogue(), workflow, state)
            .map_err(WriteError::State)?;
        let fields = UpdateFields {
            workflow_state_id: Some(state_id),
            ..UpdateFields::default()
        };
        self.write(
            Operation::Transition,
            &Method::PATCH,
            &self.paths().entity(ShortcutKey::Story(story)),
            &fields.body(),
            Resend::WhileRetryable,
        )
        .map_err(WriteError::Tracker)
    }
}
