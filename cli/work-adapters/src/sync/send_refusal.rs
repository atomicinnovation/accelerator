//! Whether a planned send of a local body to the tracker must be refused.

use tracker::Unrenderable;
use work::sync::Action;

use crate::sync::body_comparison::BodyComparison;

/// Why a send of the local body was refused before it reached the tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendRefusal {
    /// The local body still holds the tracker's raw document.
    PullResidue,
    /// A planned push would replace content the local body cannot express.
    RemoteUnrenderable(Unrenderable),
    /// The remote was not read, so neither check above could run.
    RemoteUnfetched,
}

/// Whether sending `local_body` under `action` must be refused, against the
/// fetched `projected` remote.
///
/// An overwrite is a human's choice to replace the remote whole, so only a
/// planned push is refused over an unrenderable remote; residue is refused
/// for both.
#[must_use]
pub fn send_refusal(
    comparison: &BodyComparison<'_>,
    action: Action,
    local_body: &str,
    projected: Option<&str>,
) -> Option<SendRefusal> {
    if !action.sends_to_tracker() {
        return None;
    }
    let Some(projected) = projected else {
        return Some(SendRefusal::RemoteUnfetched);
    };
    if comparison.pull_residue(local_body, projected).is_some() {
        return Some(SendRefusal::PullResidue);
    }
    match (action, comparison.local_body(projected)) {
        (Action::Push, Err(cause)) => {
            Some(SendRefusal::RemoteUnrenderable(cause))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use tracker::BodyDialect;
    use tracker::PlainDialect;
    use tracker::Unrenderable;
    use work::sync::Action;

    use super::send_refusal;
    use super::SendRefusal;
    use crate::sync::body_comparison::BodyComparison;

    struct PanelRefusingDialect;

    impl BodyDialect for PanelRefusingDialect {
        fn local_body(
            &self,
            description: &str,
        ) -> Result<String, Unrenderable> {
            if description.contains("panel") {
                Err(Unrenderable {
                    detail: "panel".to_owned(),
                })
            } else {
                Ok(description.to_owned())
            }
        }

        fn is_wire_document(&self, body: &str) -> bool {
            body.trim_start().starts_with("{\"type\":\"doc\"")
        }
    }

    const RESIDUE: &str = "Title\n{\"type\":\"doc\"}\n";
    const RENDERABLE: &str = "Title\nRemote body\n";
    const UNRENDERABLE: &str = "Title\n{\"type\":\"doc\",\"panel\":1}\n";

    fn refusal(
        action: Action,
        local_body: &str,
        remote: Option<&str>,
    ) -> Option<SendRefusal> {
        send_refusal(
            &BodyComparison::new(&PanelRefusingDialect),
            action,
            local_body,
            remote,
        )
    }

    #[test]
    fn a_push_or_overwrite_of_residue_is_refused() {
        for action in [Action::Push, Action::Overwrite] {
            assert_eq!(
                refusal(action, RESIDUE, Some(RENDERABLE)),
                Some(SendRefusal::PullResidue),
                "{action:?}"
            );
        }
    }

    #[test]
    fn an_action_sending_no_description_is_not_refused() {
        for action in [Action::Pull, Action::Prompt, Action::Noop] {
            assert_eq!(refusal(action, RESIDUE, None), None, "{action:?}");
        }
    }

    #[test]
    fn a_send_over_an_unfetched_remote_is_refused() {
        assert_eq!(
            refusal(Action::Push, "Local body\n", None),
            Some(SendRefusal::RemoteUnfetched)
        );
    }

    #[test]
    fn a_linear_overwrite_of_a_body_equal_to_the_remote_is_not_refused() {
        assert_eq!(
            send_refusal(
                &BodyComparison::new(&PlainDialect),
                Action::Overwrite,
                "Remote body\n",
                Some(RENDERABLE),
            ),
            None
        );
    }

    #[test]
    fn a_planned_push_over_an_unrenderable_remote_is_refused() {
        assert_eq!(
            refusal(Action::Push, "Local body\n", Some(UNRENDERABLE)),
            Some(SendRefusal::RemoteUnrenderable(Unrenderable {
                detail: "panel".to_owned(),
            }))
        );
    }

    #[test]
    fn an_overwrite_over_an_unrenderable_remote_is_not_refused() {
        assert_eq!(
            refusal(Action::Overwrite, "Local body\n", Some(UNRENDERABLE)),
            None
        );
    }
}
