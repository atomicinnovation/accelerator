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

/// Keyed by repo-relative path: each path's state when the run first
/// touched it, and every state the run has since set out to write there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteSet {
    recorded: BTreeMap<String, Recorded>,
    intended: BTreeMap<String, Vec<StartState>>,
}

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
        self.recorded.get(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Recorded)> {
        self.recorded.iter()
    }

    #[must_use]
    pub fn intended(&self, path: &str) -> &[StartState] {
        self.intended.get(path).map_or(&[], Vec::as_slice)
    }

    /// Every path whose state `now` differs from the one recorded for it: to
    /// restore, under the base it was recorded against, when it held content,
    /// or to remove when it was absent. A path missing from `now` is absent.
    #[must_use]
    pub fn unreverted(&self, now: &BTreeMap<String, StartState>) -> Unreverted {
        let mut unreverted = Unreverted::default();
        for (path, recorded) in &self.recorded {
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
            self.recorded.entry(path).or_insert(recorded);
        }
        self
    }

    /// Remembers each state the run is about to write, alongside every
    /// state it set out to write there before, since a retirement rolled
    /// back restores the path to one of those.
    #[must_use]
    pub fn intending(
        mut self,
        writes: impl IntoIterator<Item = (String, StartState)>,
    ) -> Self {
        for (path, state) in writes {
            let intended = self.intended.entry(path).or_default();
            if !intended.contains(&state) {
                intended.push(state);
            }
        }
        self
    }

    /// The recorded paths whose state `now` the run accounts for: the one it
    /// started from, or one it set out to write. Any other state is an edit
    /// made outside the run. A path missing from `now` is absent.
    #[must_use]
    pub fn attributable_to_the_run(
        &self,
        now: &BTreeMap<String, StartState>,
    ) -> Self {
        let recorded = self
            .recorded
            .iter()
            .filter(|(path, recorded)| {
                let current = now.get(*path).unwrap_or(&StartState::Absent);
                *current == recorded.state
                    || self.intended(path).contains(current)
            })
            .map(|(path, recorded)| (path.clone(), recorded.clone()))
            .collect();
        Self {
            recorded,
            intended: self.intended.clone(),
        }
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

    fn attributable(
        write_set: &WriteSet,
        states: &[(&str, StartState)],
    ) -> Vec<String> {
        write_set
            .attributable_to_the_run(&now(states))
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }

    #[test]
    fn a_path_at_a_state_the_run_intended_is_attributable_to_it() {
        let intending = started().intending([
            (OLD.to_owned(), StartState::Absent),
            (REFERRER.to_owned(), content("rewritten")),
        ]);

        assert_eq!(
            attributable(
                &intending,
                &[(OLD, StartState::Absent), (REFERRER, content("rewritten"))]
            ),
            vec![REFERRER.to_owned(), OLD.to_owned(), NEW.to_owned()]
        );
    }

    #[test]
    fn a_path_changed_to_a_state_the_run_never_intended_is_not() {
        let intending =
            started().intending([(REFERRER.to_owned(), content("rewritten"))]);

        assert_eq!(
            attributable(
                &intending,
                &[(OLD, content("old")), (REFERRER, content("edited"))]
            ),
            vec![OLD.to_owned(), NEW.to_owned()]
        );
    }

    #[test]
    fn a_path_rolled_back_to_an_earlier_intended_state_is_attributable() {
        let intending = started()
            .intending([(REFERRER.to_owned(), content("first entry"))])
            .intending([(REFERRER.to_owned(), content("second entry"))]);

        assert!(attributable(
            &intending,
            &[(OLD, content("old")), (REFERRER, content("first entry"))]
        )
        .contains(&REFERRER.to_owned()));
    }

    #[test]
    fn an_intended_state_is_remembered_once() {
        let intending = started()
            .intending([(REFERRER.to_owned(), content("rewritten"))])
            .intending([(REFERRER.to_owned(), content("rewritten"))]);

        assert_eq!(intending.intended(REFERRER), &[content("rewritten")]);
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
