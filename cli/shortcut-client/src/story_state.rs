//! The one lookup from a state name to a workflow state id, used by
//! `transition` and `update --state`.
//!
//! A story's state belongs to the story's own workflow, so the name resolves
//! within that workflow only, from the catalogue.

use std::fmt;

use crate::catalogue::Catalogue;

const REFRESH: &str =
    "run `accelerator shortcut init discover` to refresh the catalogue";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateRefusal {
    WorkflowMissing {
        workflow: i64,
    },
    Unknown {
        name: String,
        workflow: String,
        valid: Vec<String>,
    },
    Ambiguous {
        name: String,
        workflow: String,
        candidates: Vec<String>,
    },
}

impl fmt::Display for StateRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkflowMissing { workflow } => write!(
                formatter,
                "E_STATE_NO_WORKFLOW: the story's workflow {workflow} is not \
                 in catalogue.json; {REFRESH}"
            ),
            Self::Unknown {
                name,
                workflow,
                valid,
            } => write!(
                formatter,
                "E_STATE_UNKNOWN: the {workflow} workflow has no state named \
                 {name:?}; its states are {}. If the state is new, {REFRESH}",
                valid.join(", ")
            ),
            Self::Ambiguous {
                name,
                workflow,
                candidates,
            } => write!(
                formatter,
                "E_STATE_AMBIGUOUS: the {workflow} workflow has more than one \
                 state named {name:?}: {}. Rename one in Shortcut, then {REFRESH}",
                candidates.join(", ")
            ),
        }
    }
}

impl std::error::Error for StateRefusal {}

/// # Errors
///
/// [`StateRefusal`] when the workflow is not catalogued, or `name` matches
/// none or more than one of its states, ignoring case.
pub fn resolve_story_state(
    catalogue: &Catalogue,
    workflow_id: i64,
    name: &str,
) -> Result<i64, StateRefusal> {
    let workflow = catalogue.workflow(workflow_id).ok_or(
        StateRefusal::WorkflowMissing {
            workflow: workflow_id,
        },
    )?;
    let matching: Vec<_> = workflow
        .states
        .iter()
        .filter(|state| state.name.eq_ignore_ascii_case(name))
        .collect();
    match matching.as_slice() {
        [state] => Ok(state.id),
        [] => Err(StateRefusal::Unknown {
            name: name.to_owned(),
            workflow: workflow.name.clone(),
            valid: workflow
                .states
                .iter()
                .map(|state| state.name.clone())
                .collect(),
        }),
        several => Err(StateRefusal::Ambiguous {
            name: name.to_owned(),
            workflow: workflow.name.clone(),
            candidates: several
                .iter()
                .map(|state| format!("{} ({})", state.name, state.id))
                .collect(),
        }),
    }
}
