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
    epoch: u64,
    suffix: String,
    id: String,
}

impl RecoveryLabel {
    #[must_use]
    pub fn new(epoch: u64, suffix: &str, id: &str) -> Self {
        Self {
            epoch,
            suffix: suffix.to_owned(),
            id: id.to_owned(),
        }
    }

    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let mut parts = name.splitn(3, '-');
        let epoch = parts.next()?.parse().ok()?;
        let suffix = parts.next().filter(|suffix| !suffix.is_empty())?;
        let id = parts.next().filter(|id| !id.is_empty())?;
        Some(Self::new(epoch, suffix, id))
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

    #[must_use]
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }
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
    fn a_name_missing_a_part_is_not_a_label() {
        for name in
            ["1700000000-k7mq3x", "1700000000--0001", "soon-k7mq3x-0001"]
        {
            assert_eq!(RecoveryLabel::parse(name), None, "{name}");
        }
    }
}
