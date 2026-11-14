//! The one builder for a story or epic update `PATCH`.
//!
//! It sends exactly the fields it is given, in one request: a field left
//! `None` is absent from the body, never sent empty.

use serde_json::Map;
use serde_json::Value;

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
