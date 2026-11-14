//! The one builder for a story or epic update `PATCH`.
//!
//! It sends exactly the fields it is given, in one request: a field left
//! `None` is absent from the body, never sent empty.

use serde_json::Map;
use serde_json::Value;

use crate::classify::Operation;
use crate::story_state::resolve_story_state;
use crate::transport::Resend;
use crate::writes::StoryUpdate;
use crate::writes::WriteError;
use crate::Method;
use crate::ShortcutClient;
use crate::ShortcutKey;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateFields {
    pub name: Option<String>,
    pub description: Option<String>,
    pub workflow_state_id: Option<i64>,
    pub owner_ids: Option<Vec<String>>,
    pub estimate: Option<i64>,
}

impl UpdateFields {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    #[must_use]
    pub fn body(&self) -> Value {
        let mut body = Map::new();
        if let Some(name) = &self.name {
            body.insert("name".to_owned(), Value::from(name.as_str()));
        }
        if let Some(description) = &self.description {
            body.insert(
                "description".to_owned(),
                Value::from(description.as_str()),
            );
        }
        if let Some(state) = self.workflow_state_id {
            body.insert("workflow_state_id".to_owned(), Value::from(state));
        }
        if let Some(owners) = &self.owner_ids {
            body.insert("owner_ids".to_owned(), Value::from(owners.clone()));
        }
        if let Some(estimate) = self.estimate {
            body.insert("estimate".to_owned(), Value::from(estimate));
        }
        Value::Object(body)
    }
}

impl ShortcutClient {
    /// Sends every field `update` names in one `PATCH`, after resolving a
    /// state against the story's workflow and an owner against the members.
    ///
    /// # Errors
    ///
    /// [`WriteError::NotAStory`] for a state or estimate on an epic, a state
    /// or owner refusal, all sending nothing, and [`WriteError`] for a failed
    /// read or send.
    pub fn update_entity(
        &self,
        key: ShortcutKey,
        update: &StoryUpdate,
    ) -> Result<Value, WriteError> {
        if update.names_a_story_only_field() {
            key.into_story()
                .map_err(|epic| WriteError::NotAStory(epic.0))?;
        }
        let workflow_state_id = match (&update.state, key) {
            (Some(name), ShortcutKey::Story(story)) => {
                let workflow = self.story_workflow(story)?;
                Some(
                    resolve_story_state(self.catalogue(), workflow, name)
                        .map_err(WriteError::State)?,
                )
            }
            _ => None,
        };
        let owner_ids = update
            .owner
            .as_deref()
            .map(|mention| self.owner_uuid(mention).map(|uuid| vec![uuid]))
            .transpose()?;
        let fields = UpdateFields {
            name: update.title.clone(),
            description: update.description.clone(),
            workflow_state_id,
            owner_ids,
            estimate: update.estimate,
        };
        self.write(
            Operation::Update,
            &Method::PATCH,
            &self.paths().entity(key),
            &fields.body(),
            Resend::WhileRetryable,
        )
        .map_err(WriteError::Tracker)
    }
}
