//! Every path a re-key run may write, with its state when the run first
//! touched it, so a revert can be checked against the bytes themselves.

use std::collections::BTreeMap;

/// A content digest, computed where the bytes are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest(String);

impl Digest {
    #[must_use]
    pub fn new(digest: impl Into<String>) -> Self {
        Self(digest.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartState {
    Absent,
    Content(Digest),
}

/// The version-control commits a path was recorded against: one, or a
/// merge's parents.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Base(Vec<String>);

impl Base {
    #[must_use]
    pub const fn new(commits: Vec<String>) -> Self {
        Self(commits)
    }

    #[must_use]
    pub fn commits(&self) -> &[String] {
        &self.0
    }
}

/// `base` is `None` when the working-copy check was bypassed or there is no
/// version control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub state: StartState,
    pub base: Option<Base>,
}

/// Keyed by repo-relative path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteSet(BTreeMap<String, Recorded>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Unreverted {
    pub restore: BTreeMap<Option<Base>, Vec<String>>,
    pub remove: Vec<String>,
}

impl Unreverted {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.restore.is_empty() && self.remove.is_empty()
    }
}

impl WriteSet {
    #[must_use]
    pub fn recorded(&self, path: &str) -> Option<&Recorded> {
        self.0.get(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Recorded)> {
        self.0.iter()
    }

    /// Every path whose state `now` differs from the one recorded for it: to
    /// restore, under the base it was recorded against, when it held content,
    /// or to remove when it was absent. A path missing from `now` is absent.
    #[must_use]
    pub fn unreverted(&self, now: &BTreeMap<String, StartState>) -> Unreverted {
        let mut unreverted = Unreverted::default();
        for (path, recorded) in &self.0 {
            let current = now.get(path).unwrap_or(&StartState::Absent);
            if *current == recorded.state {
                continue;
            }
            match recorded.state {
                StartState::Absent => unreverted.remove.push(path.clone()),
                StartState::Content(_) => unreverted
                    .restore
                    .entry(recorded.base.clone())
                    .or_default()
                    .push(path.clone()),
            }
        }
        unreverted
    }

    /// Adds each path not yet recorded; a recorded path keeps the state it
    /// was first recorded with.
    #[must_use]
    pub fn extended_with(
        mut self,
        additions: impl IntoIterator<Item = (String, Recorded)>,
    ) -> Self {
        for (path, recorded) in additions {
            self.0.entry(path).or_insert(recorded);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::Base;
    use super::Digest;
    use super::Recorded;
    use super::StartState;
    use super::Unreverted;
    use super::WriteSet;

    const OLD: &str = "meta/work/0042-search.md";
    const NEW: &str = "meta/work/ACC-0042-search.md";
    const REFERRER: &str = "meta/plans/2026-01-01-search.md";

    fn content(digest: &str) -> StartState {
        StartState::Content(Digest::new(digest))
    }

    fn base(commit: &str) -> Base {
        Base::new(vec![commit.to_owned()])
    }

    fn recorded(state: StartState, base: Option<Base>) -> Recorded {
        Recorded { state, base }
    }

    fn started() -> WriteSet {
        WriteSet::default().extended_with([
            (OLD.to_owned(), recorded(content("old"), Some(base("c1")))),
            (
                NEW.to_owned(),
                recorded(StartState::Absent, Some(base("c1"))),
            ),
            (
                REFERRER.to_owned(),
                recorded(content("ref"), Some(base("c1"))),
            ),
        ])
    }

    fn now(states: &[(&str, StartState)]) -> BTreeMap<String, StartState> {
        states
            .iter()
            .map(|(path, state)| ((*path).to_owned(), state.clone()))
            .collect()
    }

    #[test]
    fn a_write_set_restored_to_its_start_states_has_no_changes() {
        let restored = now(&[
            (OLD, content("old")),
            (NEW, StartState::Absent),
            (REFERRER, content("ref")),
        ]);
        assert!(started().unreverted(&restored).is_empty());
    }

    #[test]
    fn a_write_set_path_differing_from_its_start_digest_is_changed() {
        let half_run = now(&[
            (OLD, StartState::Absent),
            (NEW, StartState::Absent),
            (REFERRER, content("rewritten")),
        ]);
        assert_eq!(
            started().unreverted(&half_run),
            Unreverted {
                restore: BTreeMap::from([(
                    Some(base("c1")),
                    vec![REFERRER.to_owned(), OLD.to_owned()]
                )]),
                remove: Vec::new(),
            }
        );
    }

    #[test]
    fn a_new_file_where_the_start_state_was_absent_is_changed() {
        let created = now(&[
            (OLD, content("old")),
            (NEW, content("retired")),
            (REFERRER, content("ref")),
        ]);
        assert_eq!(
            started().unreverted(&created),
            Unreverted {
                restore: BTreeMap::new(),
                remove: vec![NEW.to_owned()],
            }
        );
    }

    #[test]
    fn extending_never_replaces_a_recorded_start_state() {
        let extended = started().extended_with([(
            REFERRER.to_owned(),
            recorded(content("rewritten"), Some(base("c2"))),
        )]);
        assert_eq!(
            extended.recorded(REFERRER),
            Some(&recorded(content("ref"), Some(base("c1"))))
        );
    }

    #[test]
    fn restore_paths_are_grouped_by_the_base_they_were_recorded_against() {
        const LATE: &str = "meta/notes/committed-after-the-halt.md";
        const FORCED: &str = "meta/notes/recorded-when-forced.md";
        let resumed = started().extended_with([
            (LATE.to_owned(), recorded(content("late"), Some(base("c2")))),
            (FORCED.to_owned(), recorded(content("forced"), None)),
        ]);
        let rewritten = now(&[
            (OLD, content("old")),
            (NEW, StartState::Absent),
            (REFERRER, content("rewritten")),
            (LATE, content("rewritten")),
            (FORCED, content("rewritten")),
        ]);
        assert_eq!(
            resumed.unreverted(&rewritten).restore,
            BTreeMap::from([
                (None, vec![FORCED.to_owned()]),
                (Some(base("c1")), vec![REFERRER.to_owned()]),
                (Some(base("c2")), vec![LATE.to_owned()]),
            ])
        );
    }
}
