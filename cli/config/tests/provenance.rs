#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use config::consent::{
    ConfigFileTracking, Environment, ProvenanceContext, Tracking,
};
use config::provenance::{path_source, PathSource};
use config::{
    ConfigAccess, ConfigError, Key, Level, PersonalFile, Resolved, Scalar,
    Value,
};

const PERSONAL: &str = "/project/.accelerator/config.local.md";
const KEY: &str = "paths.integrations";

#[derive(Default)]
struct Levels {
    team: BTreeMap<String, String>,
    personal: BTreeMap<String, String>,
}

impl ConfigAccess for Levels {
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        let map = match level {
            Some(Level::Team) => &self.team,
            Some(Level::Personal) => &self.personal,
            None => unreachable!("the query always names a level"),
        };
        Ok(map.get(&key.to_string()).map_or(Resolved::Absent, |value| {
            Resolved::Found(Value::Scalar(Scalar::String(value.clone())))
        }))
    }

    fn set(
        &self,
        _key: &Key,
        _value: &str,
        _level: Level,
    ) -> Result<(), ConfigError> {
        unreachable!("the query never writes")
    }

    fn personal_file(&self) -> &PersonalFile {
        &PersonalFile::Readable
    }
}

struct Answering {
    answer: Tracking,
    asked: RefCell<Vec<PathBuf>>,
}

impl ConfigFileTracking for Answering {
    fn tracking(&self, path: &Path) -> Tracking {
        self.asked.borrow_mut().push(path.to_path_buf());
        self.answer
    }
}

struct NoEnvironment;

impl Environment for NoEnvironment {
    fn read(&self, _name: &str) -> Option<String> {
        None
    }
}

fn source_of(levels: &Levels, tracking: Tracking) -> PathSource {
    let tracking = Answering {
        answer: tracking,
        asked: RefCell::new(Vec::new()),
    };
    let context = ProvenanceContext {
        config: levels,
        tracking: &tracking,
        environment: &NoEnvironment,
        personal_config: PathBuf::from(PERSONAL),
    };
    path_source(&context, &Key::parse(KEY).unwrap()).unwrap()
}

fn with(team: Option<&str>, personal: Option<&str>) -> Levels {
    let mut levels = Levels::default();
    if let Some(value) = team {
        levels.team.insert(KEY.to_owned(), value.to_owned());
    }
    if let Some(value) = personal {
        levels.personal.insert(KEY.to_owned(), value.to_owned());
    }
    levels
}

#[test]
fn an_unset_path_comes_from_the_default() {
    assert_eq!(
        source_of(&with(None, None), Tracking::Untracked),
        PathSource::Default
    );
}

#[test]
fn a_team_value_comes_from_the_team() {
    assert_eq!(
        source_of(&with(Some("/elsewhere"), None), Tracking::Untracked),
        PathSource::Team
    );
}

#[test]
fn a_personal_value_carries_its_files_tracking() {
    for tracking in [Tracking::Untracked, Tracking::Tracked, Tracking::Unknown]
    {
        assert_eq!(
            source_of(&with(None, Some("/elsewhere")), tracking),
            PathSource::Personal(tracking)
        );
    }
}

#[test]
fn a_personal_value_shadowing_a_team_one_is_personal() {
    assert_eq!(
        source_of(
            &with(Some("/team-choice"), Some("/personal-choice")),
            Tracking::Untracked
        ),
        PathSource::Personal(Tracking::Untracked)
    );
}

#[test]
fn the_personal_file_asked_about_is_the_contexts() {
    let tracking = Answering {
        answer: Tracking::Untracked,
        asked: RefCell::new(Vec::new()),
    };
    let levels = with(None, Some("/elsewhere"));
    let context = ProvenanceContext {
        config: &levels,
        tracking: &tracking,
        environment: &NoEnvironment,
        personal_config: PathBuf::from(PERSONAL),
    };

    path_source(&context, &Key::parse(KEY).unwrap()).unwrap();

    assert_eq!(*tracking.asked.borrow(), vec![PathBuf::from(PERSONAL)]);
}
