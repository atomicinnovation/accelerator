//! What the write operations share: the request a caller describes, why a
//! write was refused, and the lookups made before a write is sent.

use std::fmt;

use serde_json::Value;
use tracker::TrackerError;

use crate::client::page_detail;
use crate::client::ReadFailure;
use crate::paging::collect;
use crate::shortcut_key::EpicId;
use crate::shortcut_key::StoryId;
use crate::story_state::StateRefusal;
use crate::ShortcutClient;

/// The fields `update` may change. A field left `None` is not sent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub state: Option<String>,
    pub owner: Option<String>,
    pub estimate: Option<i64>,
}

impl StoryUpdate {
    #[must_use]
    pub const fn names_a_story_only_field(&self) -> bool {
        self.state.is_some() || self.estimate.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerRefusal {
    Unknown { mention: String, known: Vec<String> },
    Ambiguous { mention: String },
}

impl fmt::Display for OwnerRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown { mention, known } => write!(
                formatter,
                "E_OWNER_UNKNOWN: no workspace member has the mention name \
                 {mention:?}; the members are {}",
                known.join(", ")
            ),
            Self::Ambiguous { mention } => write!(
                formatter,
                "E_OWNER_AMBIGUOUS: more than one workspace member has the \
                 mention name {mention:?}"
            ),
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    NotAStory(EpicId),
    NotHttps(String),
    State(StateRefusal),
    Owner(OwnerRefusal),
    Read(ReadFailure),
    Tracker(TrackerError),
}

impl fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAStory(epic) => write!(
                formatter,
                "E_NOT_A_STORY: epic-{} is an epic, and only a story has this; \
                 an epic supports show, update and comment",
                epic.number()
            ),
            Self::NotHttps(url) => write!(
                formatter,
                "E_ATTACH_BAD_URL: {url:?} is not an https URL"
            ),
            Self::State(refusal) => refusal.fmt(formatter),
            Self::Owner(refusal) => refusal.fmt(formatter),
            Self::Read(failure) => failure.fmt(formatter),
            Self::Tracker(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WriteError {}

impl ShortcutClient {
    pub(crate) fn story_workflow(
        &self,
        story: StoryId,
    ) -> Result<i64, WriteError> {
        let key = crate::ShortcutKey::Story(story);
        let body = self
            .read(&self.paths().entity(key))
            .map_err(WriteError::Read)?;
        body.pointer("/entity/workflow/id")
            .or_else(|| body.pointer("/entity/workflow_id"))
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                WriteError::Read(ReadFailure::Unsent(format!(
                    "sc-{} came back without a workflow",
                    story.number()
                )))
            })
    }

    pub(crate) fn owner_uuid(
        &self,
        mention: &str,
    ) -> Result<String, WriteError> {
        let members = collect(
            self.transport(),
            self.paths(),
            &self.paths().members(),
            None,
            &self.transport().deadline(),
        )
        .map_err(|failure| {
            WriteError::Tracker(TrackerError::Retryable {
                detail: format!("shortcut members: {}", page_detail(&failure)),
            })
        })?;
        let matching: Vec<&Value> = members
            .entities
            .iter()
            .filter(|member| {
                mention_of(member)
                    .is_some_and(|name| name.eq_ignore_ascii_case(mention))
            })
            .collect();
        match matching.as_slice() {
            [member] => member
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| unknown_owner(mention, &members.entities)),
            [] => Err(unknown_owner(mention, &members.entities)),
            _ => Err(WriteError::Owner(OwnerRefusal::Ambiguous {
                mention: mention.to_owned(),
            })),
        }
    }
}

fn mention_of(member: &Value) -> Option<&str> {
    member.get("mention_name").and_then(Value::as_str)
}

fn unknown_owner(mention: &str, members: &[Value]) -> WriteError {
    WriteError::Owner(OwnerRefusal::Unknown {
        mention: mention.to_owned(),
        known: members
            .iter()
            .filter_map(mention_of)
            .map(str::to_owned)
            .collect(),
    })
}
