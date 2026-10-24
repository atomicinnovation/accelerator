//! One item's move onto its new `id`.

use crate::draft_id::DraftId;
use crate::retirement::Retirement;
use crate::tracker_key::TrackerKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RekeyTarget {
    Renumbered(String),
    TrackerKey(TrackerKey),
    Draft(DraftId),
}

impl RekeyTarget {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Renumbered(id) => id,
            Self::TrackerKey(key) => key.as_str(),
            Self::Draft(draft) => draft.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RekeyEntry {
    pub old: String,
    pub target: RekeyTarget,
}

impl RekeyEntry {
    #[must_use]
    pub fn retirement(&self) -> Retirement<'_> {
        match &self.target {
            RekeyTarget::Renumbered(id) => {
                Retirement::renumbering(&self.old, id)
            }
            RekeyTarget::TrackerKey(key) => Retirement::onto(&self.old, key),
            RekeyTarget::Draft(draft) => {
                Retirement::into_draft(&self.old, draft)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RekeyEntry;
    use super::RekeyTarget;
    use crate::draft_id::DraftId;
    use crate::retirement::Retirement;
    use crate::tracker_key::TrackerKey;

    fn entry(old: &str, target: RekeyTarget) -> RekeyEntry {
        RekeyEntry {
            old: old.to_owned(),
            target,
        }
    }

    #[test]
    fn a_renumbered_target_retires_as_a_renumbering() {
        let entry = entry("0042", RekeyTarget::Renumbered("ACC-0042".into()));
        assert_eq!(entry.target.id(), "ACC-0042");
        assert_eq!(
            entry.retirement(),
            Retirement::renumbering("0042", "ACC-0042")
        );
    }

    #[test]
    fn a_tracker_key_target_retires_onto_the_key() {
        let key = TrackerKey::parse("ENG-42")
            .unwrap_or_else(|| unreachable!("a tracker key"));
        let entry = entry("0042", RekeyTarget::TrackerKey(key.clone()));
        assert_eq!(entry.target.id(), "ENG-42");
        assert_eq!(entry.retirement(), Retirement::onto("0042", &key));
    }

    #[test]
    fn a_draft_target_retires_into_the_draft() {
        let draft = DraftId::parse("draft-k3x9mq")
            .unwrap_or_else(|| unreachable!("a draft ID"));
        let entry = entry("0042", RekeyTarget::Draft(draft.clone()));
        assert_eq!(entry.target.id(), "draft-k3x9mq");
        assert_eq!(entry.retirement(), Retirement::into_draft("0042", &draft));
    }
}
