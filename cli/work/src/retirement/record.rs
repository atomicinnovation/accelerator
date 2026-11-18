//! The durable note that a retirement has started, holding only values
//! parsed so it cannot name a path outside its own place.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use crate::id_pattern::escaped;
use crate::id_pattern::is_in_id_alphabet;
use crate::path_component::is_one_path_component;
use crate::retirement::Retirement;
use crate::retirement::RECOVERY_PARENT;

/// An ID a retirement record holds: in the ID alphabet and exactly one
/// path component, so it can name the record's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedId(String);

impl RecordedId {
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        (is_in_id_alphabet(text) && is_one_path_component(text))
            .then(|| Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A retirement's recovery directory: one directory directly under
/// [`RECOVERY_PARENT`], relative to the state directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryDir(PathBuf);

impl RecoveryDir {
    #[must_use]
    pub fn parse(path: &Path) -> Option<Self> {
        let mut components = path.components();
        let under_recovery_parent = matches!(
            components.next(),
            Some(Component::Normal(parent)) if parent == RECOVERY_PARENT
        );
        let one_directory = matches!(
            components.next(),
            Some(Component::Normal(name))
                if is_one_path_component(&name.to_string_lossy())
        );
        (under_recovery_parent && one_directory && components.next().is_none())
            .then(|| Self(path.to_owned()))
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// The durable note that a retirement has started.
///
/// Written before it applies and removed once it has finished, rolled back
/// or been refused, so a leftover record always names a retirement still in
/// progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetirementRecord {
    pub old: RecordedId,
    pub new: RecordedId,
    pub new_external_id: Option<RecordedId>,
    pub recovery_dir: RecoveryDir,
}

/// A retirement whose IDs or recovery directory cannot name a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecordableRetirement {
    pub name: String,
}

impl Display for UnrecordableRetirement {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "'{}' cannot name a retirement record",
            escaped(&self.name)
        )
    }
}

impl std::error::Error for UnrecordableRetirement {}

impl RetirementRecord {
    /// # Errors
    ///
    /// [`UnrecordableRetirement`] naming the first of `retirement`'s IDs,
    /// or its recovery directory, that a record cannot hold.
    pub fn of(
        retirement: &Retirement<'_>,
    ) -> Result<Self, UnrecordableRetirement> {
        let unrecordable = |name: String| UnrecordableRetirement { name };
        let recorded = |id: &str| {
            RecordedId::parse(id).ok_or_else(|| unrecordable(id.to_owned()))
        };
        let old = recorded(retirement.old_id)?;
        let new = recorded(retirement.new_id)?;
        let new_external_id =
            retirement.new_external_id.map(recorded).transpose()?;
        let path = retirement.recovery_dir();
        let recovery_dir = RecoveryDir::parse(&path)
            .ok_or_else(|| unrecordable(path.display().to_string()))?;
        Ok(Self {
            old,
            new,
            new_external_id,
            recovery_dir,
        })
    }

    #[must_use]
    pub fn retirement(&self) -> Retirement<'_> {
        Retirement {
            old_id: self.old.as_str(),
            new_id: self.new.as_str(),
            new_external_id: self
                .new_external_id
                .as_ref()
                .map(RecordedId::as_str),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::RecordedId;
    use super::RecoveryDir;

    #[test]
    fn an_id_in_the_id_alphabet_is_recorded_as_written() {
        for id in ["PP-760", "ENG-42", "draft-k7mq3x", "0042", "a.b_c"] {
            assert_eq!(
                RecordedId::parse(id).as_ref().map(RecordedId::as_str),
                Some(id)
            );
        }
    }

    #[test]
    fn an_id_that_would_name_a_path_is_not_recorded() {
        for id in ["", ".", "..", "../../etc", "a/b", "a\\b", "/abs"] {
            assert_eq!(RecordedId::parse(id), None, "{id:?}");
        }
    }

    #[test]
    fn an_id_outside_the_id_alphabet_is_not_recorded() {
        for id in ["PP\"1", "ENG-1\nid: x", "ENG 1", "PP'1"] {
            assert_eq!(RecordedId::parse(id), None, "{id:?}");
        }
    }

    #[test]
    fn a_directory_directly_under_the_recovery_parent_is_a_recovery_dir() {
        let path = Path::new("retirement-recovery/PP-1--ENG-1");

        assert_eq!(
            RecoveryDir::parse(path).as_ref().map(RecoveryDir::as_path),
            Some(path)
        );
    }

    #[test]
    fn a_path_leaving_the_recovery_parent_is_not_a_recovery_dir() {
        for path in [
            "../../etc",
            "/abs",
            "retirement-recovery",
            "retirement-recovery/a/b",
            "retirement-recovery/..",
            "other/x",
            "",
        ] {
            assert_eq!(RecoveryDir::parse(Path::new(path)), None, "{path:?}");
        }
    }
}
