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
use crate::transition::StateId;

const UPDATE: &str = "mutation($id: String!, $input: IssueUpdateInput!) {
    issueUpdate(id: $id, input: $input) { success issue { id identifier } }
  }";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UpdateFields<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub state_id: Option<&'a StateId>,
    pub assignee_id: Option<&'a str>,
    pub priority: Option<i64>,
}

impl UpdateFields<'_> {
    fn input(&self) -> Map<String, Value> {
        let named = [
            ("title", self.title.map(Value::from)),
            ("description", self.description.map(Value::from)),
            ("stateId", self.state_id.map(|id| Value::from(id.as_str()))),
            ("assigneeId", self.assignee_id.map(Value::from)),
            ("priority", self.priority.map(Value::from)),
        ];
        named
            .into_iter()
            .filter_map(|(key, value)| {
                value.map(|value| (key.to_owned(), value))
            })
            .collect()
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
        let input = fields.input();
        if input.is_empty() {
            return Err(LinearFailure::wire(
                Outcome::RequestInvalid,
                Operation::Update,
                "an update must name at least one field".to_owned(),
            ));
        }
        let variables = json!({"id": id.as_str(), "input": input});
        self.call_op(UPDATE, &variables, Operation::Update, id.as_str())?;
        Ok(())
    }
}
