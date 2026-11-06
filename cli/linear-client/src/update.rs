//! An update names only the fields it changes; Linear leaves every field its
//! input omits as it is.

use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use tracker::ExternalId;

use crate::classify::Operation;
use crate::classify::Outcome;
use crate::client::refuse_identifier_op;
use crate::client::LinearClient;
use crate::failure::LinearFailure;

const UPDATE: &str = "mutation($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) { success issue { id identifier } }
  }";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UpdateFields<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    /// A UUID from [`LinearClient::resolve_state`], never a state name.
    pub state_id: Option<&'a str>,
    pub assignee_id: Option<&'a str>,
    pub priority: Option<i64>,
}

impl UpdateFields<'_> {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.state_id.is_none()
            && self.assignee_id.is_none()
            && self.priority.is_none()
    }

    fn input(&self) -> Value {
        let named = [
            ("title", self.title.map(Value::from)),
            ("description", self.description.map(Value::from)),
            ("stateId", self.state_id.map(Value::from)),
            ("assigneeId", self.assignee_id.map(Value::from)),
            ("priority", self.priority.map(Value::from)),
        ];
        Value::Object(
            named
                .into_iter()
                .filter_map(|(key, value)| {
                    value.map(|value| (key.to_owned(), value))
                })
                .collect::<Map<_, _>>(),
        )
    }
}

impl LinearClient {
    /// `update`, surfacing the structured discriminant.
    ///
    /// # Errors
    ///
    /// [`LinearFailure`] for a refused identifier, an update naming no field,
    /// or a wire failure.
    pub fn update_op(
        &self,
        id: &ExternalId,
        fields: &UpdateFields<'_>,
    ) -> Result<(), LinearFailure> {
        refuse_identifier_op(id, Operation::Update)?;
        if fields.is_empty() {
            return Err(LinearFailure::wire(
                Outcome::RequestInvalid,
                Operation::Update,
                "an update must name at least one field".to_owned(),
            ));
        }
        let variables = json!({"id": id.as_str(), "input": fields.input()});
        self.call_op(UPDATE, &variables, Operation::Update, id.as_str())?;
        Ok(())
    }
}
