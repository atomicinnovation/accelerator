//! The name of the directory that keeps one pulled item's local edits.

use std::path::Path;
use std::path::PathBuf;

/// Where every pull's kept edits live, relative to the recovery-state root.
pub const PARENT: &str = "sync-recovery";

/// `<epoch>-<suffix>-<id>`: the run, a suffix apart, and the item.
///
/// The suffix tells two runs in one second apart. Neither the epoch nor the
/// suffix holds a `-`, so the id is recovered whole even when it holds one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryLabel {
    epoch: String,
    suffix: String,
    id: String,
}

impl RecoveryLabel {
    #[must_use]
    pub fn new(epoch: u64, suffix: &str, id: &str) -> Self {
        Self {
            epoch: epoch.to_string(),
            suffix: suffix.to_owned(),
            id: id.to_owned(),
        }
    }

    /// A directory name whose epoch does not parse still names its item, so
    /// its copy is kept since an unknown date rather than disowned.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let mut parts = name.splitn(3, '-');
        let epoch = parts.next().filter(|epoch| !epoch.is_empty())?;
        let suffix = parts.next().filter(|suffix| is_drawn_suffix(suffix))?;
        let id = parts.next().filter(|id| !id.is_empty())?;
        Some(Self {
            epoch: epoch.to_owned(),
            suffix: suffix.to_owned(),
            id: id.to_owned(),
        })
    }

    #[must_use]
    pub fn dir_name(&self) -> String {
        format!("{}-{}-{}", self.epoch, self.suffix, self.id)
    }

    /// The directory under the recovery-state root.
    #[must_use]
    pub fn dir(&self) -> PathBuf {
        Path::new(PARENT).join(self.dir_name())
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// When the edits were kept; `None` when the name's epoch is unreadable.
    #[must_use]
    pub fn epoch(&self) -> Option<u64> {
        self.epoch.parse().ok()
    }
}

fn is_drawn_suffix(suffix: &str) -> bool {
    suffix.len() == 6
        && suffix
            .bytes()
            .all(|byte| work::draft_id::ALPHABET.contains(&byte))
}

/// Names the directory a pull keeps an item's edits in.
pub trait RecoveryLabels {
    /// `None` when `id` cannot name a directory safely.
    fn label_for(&self, id: &str, epoch: u64) -> Option<RecoveryLabel>;
}

#[cfg(test)]
mod tests {
    use super::RecoveryLabel;

    #[test]
    fn a_recovery_label_round_trips_a_hyphenated_id() {
        let label = RecoveryLabel::new(1_700_000_000, "k7mq3x", "ENG-42");

        assert_eq!(label.dir_name(), "1700000000-k7mq3x-ENG-42");
        assert_eq!(RecoveryLabel::parse(&label.dir_name()), Some(label));
    }

    #[test]
    fn a_name_missing_a_part_or_a_drawn_suffix_is_not_a_label() {
        for name in [
            "1700000000-k7mq3x",
            "1700000000--0001",
            "-k7mq3x-0001",
            "1700000000-K7MQ3X-0001",
            "1700000000-k7mq-0001",
        ] {
            assert_eq!(RecoveryLabel::parse(name), None, "{name}");
        }
    }

    #[test]
    fn a_label_whose_epoch_is_unreadable_keeps_its_item() {
        let label = RecoveryLabel::parse("soon-k7mq3x-0001");

        assert_eq!(label.as_ref().map(RecoveryLabel::id), Some("0001"));
        assert_eq!(label.and_then(|label| label.epoch()), None);
    }
}
