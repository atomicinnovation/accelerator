//! The open-setting ladder over in-memory ports: which rung wins, what counts
//! as unset, and which keys may climb it at all.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;

use config::credentials::Environment;
use config::open_setting::{
    open_value, OpenKey, SettingSource, SettingValue, UnreadableLevel,
};
use config::{
    ConfigAccess, ConfigError, Key, Level, PersonalFile, Resolved, Scalar,
    Value,
};

const KEY: &str = "crossref.mailto";
const VARIABLE: &str = "ACCELERATOR_CROSSREF_MAILTO";

#[derive(Default)]
struct Project {
    environment: BTreeMap<String, String>,
    personal: Option<BTreeMap<String, String>>,
    team: BTreeMap<String, String>,
}

impl Project {
    fn with_environment(mut self, value: &str) -> Self {
        self.environment
            .insert(VARIABLE.to_owned(), value.to_owned());
        self
    }

    fn with_personal(mut self, value: &str) -> Self {
        self.personal
            .get_or_insert_with(BTreeMap::new)
            .insert(KEY.to_owned(), value.to_owned());
        self
    }

    fn with_unreadable_personal(mut self) -> Self {
        self.personal = None;
        self
    }

    fn with_team(mut self, value: &str) -> Self {
        self.team.insert(KEY.to_owned(), value.to_owned());
        self
    }

    fn resolved(&self) -> Result<Option<SettingValue>, UnreadableLevel> {
        let key = OpenKey::declared(KEY).expect("crossref.mailto is open");
        open_value(self, self, &key)
    }
}

impl Environment for Project {
    fn read(&self, name: &str) -> Option<String> {
        self.environment.get(name).cloned()
    }
}

impl ConfigAccess for Project {
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        let map = match level {
            Some(Level::Personal) => {
                self.personal.as_ref().ok_or_else(|| ConfigError::Io {
                    path: Level::Personal.filename().to_owned(),
                    detail: "boom".to_owned(),
                })?
            }
            Some(Level::Team) => &self.team,
            None => unreachable!("the ladder always names a level"),
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
        unreachable!("the ladder never writes")
    }

    fn personal_file(&self) -> &PersonalFile {
        &PersonalFile::Readable
    }
}

fn found(value: &str, source: SettingSource) -> SettingValue {
    SettingValue {
        value: value.to_owned(),
        source,
    }
}

fn readable() -> Project {
    Project {
        personal: Some(BTreeMap::new()),
        ..Project::default()
    }
}

#[test]
fn the_environment_outranks_both_config_files() {
    let project = readable()
        .with_environment("env@example.org")
        .with_personal("personal@example.org")
        .with_team("team@example.org");

    assert_eq!(
        project.resolved(),
        Ok(Some(found(
            "env@example.org",
            SettingSource::Environment(VARIABLE)
        )))
    );
}

#[test]
fn the_personal_file_outranks_the_team_file() {
    let project = readable()
        .with_personal("personal@example.org")
        .with_team("team@example.org");

    assert_eq!(
        project.resolved(),
        Ok(Some(found(
            "personal@example.org",
            SettingSource::Level(Level::Personal)
        )))
    );
}

#[test]
fn the_team_file_is_read_when_no_higher_rung_holds_a_value() {
    let project = readable().with_team("team@example.org");

    assert_eq!(
        project.resolved(),
        Ok(Some(found(
            "team@example.org",
            SettingSource::Level(Level::Team)
        )))
    );
}

#[test]
fn blank_values_count_as_unset() {
    let project = readable()
        .with_environment(" ")
        .with_personal("")
        .with_team(" team@example.org ");

    assert_eq!(
        project.resolved(),
        Ok(Some(found(
            "team@example.org",
            SettingSource::Level(Level::Team)
        )))
    );
}

#[test]
fn nothing_set_anywhere_is_no_value() {
    assert_eq!(readable().resolved(), Ok(None));
}

#[test]
fn the_chosen_value_is_returned_whatever_it_holds() {
    let project = readable()
        .with_environment("not-an-address")
        .with_team("team@example.org");

    assert_eq!(
        project.resolved(),
        Ok(Some(found(
            "not-an-address",
            SettingSource::Environment(VARIABLE)
        )))
    );
}

#[test]
fn an_unreadable_level_is_reported_with_its_reason() {
    let project = Project::default()
        .with_unreadable_personal()
        .with_team("team@example.org");

    let unreadable = project.resolved().expect_err("personal is unreadable");

    assert_eq!(unreadable.level, Level::Personal);
    assert!(matches!(unreadable.reason, ConfigError::Io { .. }));
}

#[test]
fn each_source_displays_as_where_a_user_would_set_it() {
    assert_eq!(SettingSource::Environment(VARIABLE).to_string(), VARIABLE);
    assert_eq!(
        SettingSource::Level(Level::Personal).to_string(),
        "config.local.md"
    );
    assert_eq!(SettingSource::Level(Level::Team).to_string(), "config.md");
}

#[test]
fn only_an_open_key_that_is_no_credential_may_climb_the_ladder() {
    for refused in [
        "jira.allowed_sites",
        "openalex.api_key",
        "openalex.api_key_cmd",
        "no.such.key",
    ] {
        assert!(
            matches!(
                OpenKey::declared(refused),
                Err(ConfigError::Invalid { .. })
            ),
            "{refused} must be refused"
        );
    }
    assert!(OpenKey::declared(KEY).is_ok());
}
