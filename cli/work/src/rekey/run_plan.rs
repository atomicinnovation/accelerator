//! The persisted run plan that binds a preview to its apply and lets an
//! interrupted run finish.

use std::path::PathBuf;

use crate::rekey::entry::RekeyEntry;
use crate::rekey::fingerprint::IdentityFingerprint;
use crate::rekey::write_set::Recorded;
use crate::rekey::write_set::WriteSet;

/// Bumped whenever the run plan gains a field; a reader finishes plans of
/// any schema up to its own.
pub const RUN_PLAN_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadablePlan {
    pub path: PathBuf,
    pub cause: Unreadability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreadability {
    Malformed(String),
    NewerSchema(u32),
}

/// # Errors
///
/// [`Unreadability::NewerSchema`] when `schema` is newer than this reader's.
pub const fn readable_schema(schema: u32) -> Result<(), Unreadability> {
    if schema > RUN_PLAN_SCHEMA {
        Err(Unreadability::NewerSchema(schema))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunPlan {
    Proposed(ProposedPlan),
    InProgress(InProgressPlan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedPlan {
    fingerprint: IdentityFingerprint,
    entries: Vec<RekeyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InProgressPlan {
    fingerprint: IdentityFingerprint,
    entries: Vec<RekeyEntry>,
    write_set: WriteSet,
}

impl ProposedPlan {
    #[must_use]
    pub const fn new(
        fingerprint: IdentityFingerprint,
        entries: Vec<RekeyEntry>,
    ) -> Self {
        Self {
            fingerprint,
            entries,
        }
    }

    #[must_use]
    pub const fn fingerprint(&self) -> &IdentityFingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub fn entries(&self) -> &[RekeyEntry] {
        &self.entries
    }

    /// One `<old> → <new>` line per entry, by old `id` byte-wise.
    #[must_use]
    pub fn preview_lines(&self) -> Vec<String> {
        let mut entries: Vec<&RekeyEntry> = self.entries.iter().collect();
        entries.sort_by(|left, right| left.old.cmp(&right.old));
        entries
            .into_iter()
            .map(|entry| format!("{} → {}", entry.old, entry.target.id()))
            .collect()
    }

    #[must_use]
    pub fn started(self, write_set: WriteSet) -> InProgressPlan {
        InProgressPlan::new(self.fingerprint, self.entries, write_set)
    }
}

impl InProgressPlan {
    #[must_use]
    pub const fn new(
        fingerprint: IdentityFingerprint,
        entries: Vec<RekeyEntry>,
        write_set: WriteSet,
    ) -> Self {
        Self {
            fingerprint,
            entries,
            write_set,
        }
    }

    #[must_use]
    pub const fn fingerprint(&self) -> &IdentityFingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub fn entries(&self) -> &[RekeyEntry] {
        &self.entries
    }

    #[must_use]
    pub const fn write_set(&self) -> &WriteSet {
        &self.write_set
    }

    #[must_use]
    pub fn extended_with(
        self,
        additions: impl IntoIterator<Item = (String, Recorded)>,
    ) -> Self {
        Self {
            write_set: self.write_set.extended_with(additions),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use corpus::WorkItemIdScheme;

    use super::readable_schema;
    use super::ProposedPlan;
    use super::Unreadability;
    use super::RUN_PLAN_SCHEMA;
    use crate::rekey::entry::RekeyEntry;
    use crate::rekey::entry::RekeyTarget;
    use crate::rekey::fingerprint::IdentityFingerprint;
    use crate::rekey::write_set::Digest;
    use crate::rekey::write_set::Recorded;
    use crate::rekey::write_set::StartState;
    use crate::rekey::write_set::WriteSet;

    fn renumbered(old: &str, new: &str) -> RekeyEntry {
        RekeyEntry {
            old: old.to_owned(),
            target: RekeyTarget::Renumbered(new.to_owned()),
        }
    }

    fn proposed() -> ProposedPlan {
        ProposedPlan::new(
            IdentityFingerprint::of(&WorkItemIdScheme::numeric(), &[]),
            vec![
                renumbered("draft-k3x9mq", "ACC-0042"),
                renumbered("0041", "ACC-0041"),
            ],
        )
    }

    fn recorded(path: &str, digest: &str) -> (String, Recorded) {
        (
            path.to_owned(),
            Recorded {
                state: StartState::Content(Digest::new(digest)),
                base: None,
            },
        )
    }

    #[test]
    fn preview_lines_are_ordered_by_old_id_byte_wise() {
        assert_eq!(
            proposed().preview_lines(),
            vec!["0041 → ACC-0041", "draft-k3x9mq → ACC-0042"]
        );
    }

    #[test]
    fn starting_a_proposed_plan_records_its_write_set_with_start_states() {
        let write_set = WriteSet::default()
            .extended_with([recorded("meta/work/0041-a.md", "a")]);
        let started = proposed().started(write_set.clone());
        assert_eq!(started.entries(), proposed().entries.as_slice());
        assert_eq!(started.write_set(), &write_set);
    }

    #[test]
    fn an_in_progress_plan_extends_its_write_set_and_keeps_its_entries() {
        let started = proposed().started(WriteSet::default());
        let extended =
            started.extended_with([recorded("meta/notes/late.md", "late")]);
        assert_eq!(extended.entries(), proposed().entries.as_slice());
        assert!(extended
            .write_set()
            .recorded("meta/notes/late.md")
            .is_some());
    }

    #[test]
    fn a_plan_of_this_or_an_older_schema_is_readable() {
        assert_eq!(readable_schema(RUN_PLAN_SCHEMA), Ok(()));
        assert_eq!(readable_schema(RUN_PLAN_SCHEMA - 1), Ok(()));
    }

    #[test]
    fn a_newer_schema_plan_is_unreadable_naming_the_version() {
        assert_eq!(
            readable_schema(RUN_PLAN_SCHEMA + 1),
            Err(Unreadability::NewerSchema(RUN_PLAN_SCHEMA + 1))
        );
    }
}
