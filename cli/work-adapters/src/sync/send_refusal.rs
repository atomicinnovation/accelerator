//! Whether a planned send of a local body to the tracker must be refused.

use tracker::Unrenderable;
use work::sync::Action;

use crate::sync::baseline::Entry;
use crate::sync::body_comparison::BodyComparison;

/// The shape of the baseline entry an item is classified against, as far as
/// a send cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryShape {
    /// One hash per side, which cannot show which local fact moved. An item
    /// with no entry, or one recorded under an unknown recipe, is treated
    /// the same.
    Legacy,
    Split,
}

impl EntryShape {
    #[must_use]
    pub const fn of(entry: Option<&Entry>) -> Self {
        match entry {
            Some(Entry::Split { .. }) => Self::Split,
            _ => Self::Legacy,
        }
    }
}

/// Whether an item has no safe local overwrite.
///
/// Its entry is legacy, its local body holds no residue, and its remote
/// holds content with no Markdown form: an overwrite would delete content
/// the plugin cannot display, to deliver a local edit whose extent the
/// legacy hash cannot show.
#[must_use]
pub fn legacy_unrenderable(
    comparison: &BodyComparison<'_>,
    shape: EntryShape,
    local_body: &str,
    projected: &str,
) -> bool {
    shape == EntryShape::Legacy
        && comparison.pull_residue(local_body, projected).is_none()
        && comparison.local_body(projected).is_err()
}

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
/// An overwrite is a human's choice to replace the remote whole, so over an
/// unrenderable remote it is refused only where [`legacy_unrenderable`]
/// holds, while a planned push always is; residue is refused for both. A
/// retitle sends no description, so nothing about the bodies can refuse it.
#[must_use]
pub fn send_refusal(
    comparison: &BodyComparison<'_>,
    action: Action,
    shape: EntryShape,
    local_body: &str,
    projected: Option<&str>,
) -> Option<SendRefusal> {
    if !matches!(action, Action::Push(_) | Action::Overwrite) {
        return None;
    }
    let Some(projected) = projected else {
        return Some(SendRefusal::RemoteUnfetched);
    };
    if comparison.pull_residue(local_body, projected).is_some() {
        return Some(SendRefusal::PullResidue);
    }
    let refused_over_unrenderable = matches!(action, Action::Push(_))
        || legacy_unrenderable(comparison, shape, local_body, projected);
    match comparison.local_body(projected) {
        Err(cause) if refused_over_unrenderable => {
            Some(SendRefusal::RemoteUnrenderable(cause))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use corpus_adapters::CommonMarkBody;
    use tracker::BodyDialect;
    use tracker::PlainDialect;
    use tracker::Unrenderable;
    use work::sync::Action;
    use work::sync::FieldDelivery;

    use super::send_refusal;
    use super::EntryShape;
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
        refusal_under(EntryShape::Split, action, local_body, remote)
    }

    fn refusal_under(
        shape: EntryShape,
        action: Action,
        local_body: &str,
        remote: Option<&str>,
    ) -> Option<SendRefusal> {
        send_refusal(
            &BodyComparison::new(&PanelRefusingDialect, &CommonMarkBody),
            action,
            shape,
            local_body,
            remote,
        )
    }

    #[test]
    fn a_push_or_overwrite_of_residue_is_refused() {
        for action in [Action::Push(FieldDelivery::Send), Action::Overwrite] {
            assert_eq!(
                refusal(action, RESIDUE, Some(RENDERABLE)),
                Some(SendRefusal::PullResidue),
                "{action:?}"
            );
        }
    }

    #[test]
    fn an_action_sending_no_description_is_not_refused() {
        for action in
            [Action::Pull, Action::Prompt, Action::Noop, Action::Retitle]
        {
            assert_eq!(refusal(action, RESIDUE, None), None, "{action:?}");
        }
    }

    #[test]
    fn a_send_over_an_unfetched_remote_is_refused() {
        assert_eq!(
            refusal(Action::Push(FieldDelivery::Send), "Local body\n", None),
            Some(SendRefusal::RemoteUnfetched)
        );
    }

    #[test]
    fn a_linear_overwrite_of_a_body_equal_to_the_remote_is_not_refused() {
        assert_eq!(
            send_refusal(
                &BodyComparison::new(&PlainDialect, &CommonMarkBody),
                Action::Overwrite,
                EntryShape::Split,
                "Remote body\n",
                Some(RENDERABLE),
            ),
            None
        );
    }

    #[test]
    fn a_planned_push_over_an_unrenderable_remote_is_refused() {
        assert_eq!(
            refusal(
                Action::Push(FieldDelivery::Send),
                "Local body\n",
                Some(UNRENDERABLE)
            ),
            Some(SendRefusal::RemoteUnrenderable(Unrenderable {
                detail: "panel".to_owned(),
            }))
        );
    }

    #[test]
    fn a_legacy_overwrite_over_an_unrenderable_remote_is_refused() {
        assert_eq!(
            refusal_under(
                EntryShape::Legacy,
                Action::Overwrite,
                "Local body\n",
                Some(UNRENDERABLE)
            ),
            Some(SendRefusal::RemoteUnrenderable(Unrenderable {
                detail: "panel".to_owned(),
            }))
        );
    }

    #[test]
    fn a_legacy_overwrite_of_residue_is_refused_as_pull_residue() {
        assert_eq!(
            refusal_under(
                EntryShape::Legacy,
                Action::Overwrite,
                RESIDUE,
                Some(UNRENDERABLE)
            ),
            Some(SendRefusal::PullResidue)
        );
    }

    #[test]
    fn a_retitle_of_an_item_holding_residue_is_not_refused() {
        assert_eq!(
            refusal_under(
                EntryShape::Legacy,
                Action::Retitle,
                RESIDUE,
                Some(RENDERABLE)
            ),
            None
        );
    }

    #[test]
    fn a_split_overwrite_over_an_unrenderable_remote_is_not_refused() {
        assert_eq!(
            refusal(Action::Overwrite, "Local body\n", Some(UNRENDERABLE)),
            None
        );
    }
}
