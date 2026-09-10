//! Writes the discovered Linear team key into `linear.team_key`.
//!
//! An absent key is written automatically; an existing, differing key is
//! overwritten only when confirmed (or `--force`d), so a routine re-init that
//! refreshes the catalogue cannot silently destroy a hand-set value. The
//! confirmation is an injectable seam so both arms are deterministically
//! testable.

use config::ConfigAccess;
use config::ConfigError;
use config::Key;
use config::Level;
use config::Resolved;

/// Asked whether an existing, differing team key may be overwritten.
pub trait OverwriteConfirmer {
    fn confirm_overwrite(&self, existing: &str, discovered: &str) -> bool;
}

/// The production confirmer: prompts on a terminal, and — with no TTY to prompt
/// on — declines, so a scripted re-init preserves the existing value rather than
/// blocking or silently clobbering it.
pub struct TtyConfirmer;

impl OverwriteConfirmer for TtyConfirmer {
    fn confirm_overwrite(&self, existing: &str, discovered: &str) -> bool {
        use std::io::{stdin, IsTerminal, Write as _};
        if !stdin().is_terminal() {
            return false;
        }
        eprint!(
            "linear.team_key is set to {existing:?}; overwrite with the \
             discovered {discovered:?}? [y/N] "
        );
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        if stdin().read_line(&mut answer).is_err() {
            return false;
        }
        matches!(answer.trim(), "y" | "Y" | "yes" | "Yes")
    }
}

/// The disposition of a writeback, reported to the operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackOutcome {
    Written,
    Unchanged,
    Overwritten,
    Preserved,
}

/// Writes `discovered` into `linear.team_key` at the team level.
///
/// Absent or equal values need no confirmation. A differing existing value is
/// overwritten only when `force` is set or the `confirmer` accepts; otherwise
/// the existing value is preserved.
///
/// # Errors
///
/// A [`ConfigError`] when the team level cannot be read or written.
pub fn write_team_key(
    config: &dyn ConfigAccess,
    discovered: &str,
    force: bool,
    confirmer: &dyn OverwriteConfirmer,
) -> Result<WritebackOutcome, ConfigError> {
    let key = Key::parse("linear.team_key")?;
    let existing = match config.get(&key, Some(Level::Team))? {
        Resolved::Found(value) => Some(config::render_value(&value)),
        Resolved::Absent => None,
    };

    match existing.as_deref() {
        None | Some("") => {
            config.set(&key, discovered, Level::Team)?;
            Ok(WritebackOutcome::Written)
        }
        Some(current) if current == discovered => {
            Ok(WritebackOutcome::Unchanged)
        }
        Some(current) => {
            if force || confirmer.confirm_overwrite(current, discovered) {
                config.set(&key, discovered, Level::Team)?;
                Ok(WritebackOutcome::Overwritten)
            } else {
                Ok(WritebackOutcome::Preserved)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use config::{
        ConfigAccess, ConfigError, Key, Level, Resolved, Scalar, Value,
    };

    use super::{write_team_key, OverwriteConfirmer, WritebackOutcome};

    struct RecordingConfig {
        team: RefCell<HashMap<String, String>>,
    }

    impl RecordingConfig {
        fn new(pairs: &[(&str, &str)]) -> Self {
            Self {
                team: RefCell::new(
                    pairs
                        .iter()
                        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                        .collect(),
                ),
            }
        }

        fn value_of(&self, key: &str) -> Option<String> {
            self.team.borrow().get(key).cloned()
        }
    }

    impl ConfigAccess for RecordingConfig {
        fn get(
            &self,
            key: &Key,
            _level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            Ok(self.team.borrow().get(&key.to_string()).map_or(
                Resolved::Absent,
                |value| {
                    Resolved::Found(Value::Scalar(Scalar::String(
                        value.clone(),
                    )))
                },
            ))
        }

        fn set(
            &self,
            key: &Key,
            value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            self.team
                .borrow_mut()
                .insert(key.to_string(), value.to_owned());
            Ok(())
        }
    }

    struct Confirmer(bool);

    impl OverwriteConfirmer for Confirmer {
        fn confirm_overwrite(
            &self,
            _existing: &str,
            _discovered: &str,
        ) -> bool {
            self.0
        }
    }

    #[test]
    fn writes_into_a_section_with_no_key() {
        let config = RecordingConfig::new(&[]);
        let outcome =
            write_team_key(&config, "ENG", false, &Confirmer(false)).unwrap();
        assert_eq!(outcome, WritebackOutcome::Written);
        assert_eq!(config.value_of("linear.team_key").as_deref(), Some("ENG"));
    }

    #[test]
    fn an_equal_key_is_unchanged() {
        let config = RecordingConfig::new(&[("linear.team_key", "ENG")]);
        let outcome =
            write_team_key(&config, "ENG", false, &Confirmer(false)).unwrap();
        assert_eq!(outcome, WritebackOutcome::Unchanged);
    }

    #[test]
    fn a_differing_key_is_overwritten_when_confirmed() {
        let config = RecordingConfig::new(&[("linear.team_key", "OLD")]);
        let outcome =
            write_team_key(&config, "ENG", false, &Confirmer(true)).unwrap();
        assert_eq!(outcome, WritebackOutcome::Overwritten);
        assert_eq!(config.value_of("linear.team_key").as_deref(), Some("ENG"));
    }

    #[test]
    fn a_differing_key_is_preserved_when_declined() {
        let config = RecordingConfig::new(&[("linear.team_key", "OLD")]);
        let outcome =
            write_team_key(&config, "ENG", false, &Confirmer(false)).unwrap();
        assert_eq!(outcome, WritebackOutcome::Preserved);
        assert_eq!(config.value_of("linear.team_key").as_deref(), Some("OLD"));
    }

    #[test]
    fn force_overwrites_without_confirmation() {
        let config = RecordingConfig::new(&[("linear.team_key", "OLD")]);
        let outcome =
            write_team_key(&config, "ENG", true, &Confirmer(false)).unwrap();
        assert_eq!(outcome, WritebackOutcome::Overwritten);
        assert_eq!(config.value_of("linear.team_key").as_deref(), Some("ENG"));
    }
}
