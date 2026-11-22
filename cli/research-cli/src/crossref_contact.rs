//! Which Crossref pool a DOI call uses, decided by `crossref.mailto`.

use std::fmt;

use config::credentials::Environment;
use config::open_setting::open_value;
use config::open_setting::OpenKey;
use config::open_setting::SettingSource;
use config::open_setting::UnreadableLevel;
use config::ConfigAccess;
use research::sources::crossref::ContactAddress;
use research::sources::service::CrossrefPool;

const KEY: &str = "crossref.mailto";

/// How a DOI call identifies itself to Crossref. The setting never fails a
/// call: anything short of a plausible address calls anonymously.
#[derive(Debug)]
pub enum CrossrefContact {
    Polite(ContactAddress),
    Anonymous,
    AnonymousBecause(ContactWarning),
}

/// Why a configured contact could not be used.
#[derive(Debug)]
pub enum ContactWarning {
    Implausible {
        value: String,
        source: SettingSource,
    },
    Unreadable(UnreadableLevel),
}

impl CrossrefContact {
    pub fn resolved(
        environment: &dyn Environment,
        config: &dyn ConfigAccess,
    ) -> Self {
        match open_value(environment, config, &contact_key()) {
            Ok(None) => Self::Anonymous,
            Ok(Some(setting)) => match ContactAddress::parse(&setting.value) {
                Ok(Some(address)) => Self::Polite(address),
                Ok(None) => Self::Anonymous,
                Err(_) => Self::AnonymousBecause(ContactWarning::Implausible {
                    value: setting.value,
                    source: setting.source,
                }),
            },
            Err(unreadable) => {
                Self::AnonymousBecause(ContactWarning::Unreadable(unreadable))
            }
        }
    }

    pub const fn pool(&self) -> CrossrefPool {
        match self {
            Self::Polite(_) => CrossrefPool::Polite,
            Self::Anonymous | Self::AnonymousBecause(_) => CrossrefPool::Public,
        }
    }

    pub const fn address(&self) -> Option<&ContactAddress> {
        match self {
            Self::Polite(address) => Some(address),
            Self::Anonymous | Self::AnonymousBecause(_) => None,
        }
    }

    pub const fn warning(&self) -> Option<&ContactWarning> {
        match self {
            Self::AnonymousBecause(warning) => Some(warning),
            Self::Polite(_) | Self::Anonymous => None,
        }
    }
}

#[allow(clippy::expect_used)]
fn contact_key() -> OpenKey {
    OpenKey::declared(KEY).expect("crossref.mailto is a declared open setting")
}

impl fmt::Display for ContactWarning {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Implausible { value, source } => write!(
                formatter,
                "warning: {KEY} from {source} ('{value}') is not a plausible \
                 address; calling Crossref anonymously without checking \
                 lower-precedence settings; correct or unset it (a blank \
                 value falls through to the next setting)"
            ),
            Self::Unreadable(UnreadableLevel { level, reason }) => {
                let file = SettingSource::Level(*level);
                write!(
                    formatter,
                    "warning: {KEY} could not be read from {file} ({reason}); \
                     calling Crossref anonymously; fix or remove {file} to \
                     use its settings"
                )
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use config::credentials::Environment;
    use config::open_setting::SettingSource;
    use config::ConfigAccess;
    use config::ConfigError;
    use config::Key;
    use config::Level;
    use config::PersonalFile;
    use config::Resolved;
    use config::Scalar;
    use config::Value;
    use research::sources::service::CrossrefPool;

    use super::ContactWarning;
    use super::CrossrefContact;

    const VARIABLE: &str = "ACCELERATOR_CROSSREF_MAILTO";

    #[derive(Default)]
    struct Project {
        environment: Option<String>,
        personal: Option<String>,
        personal_unreadable: bool,
        team: Option<String>,
    }

    impl Environment for Project {
        fn read(&self, name: &str) -> Option<String> {
            (name == VARIABLE)
                .then(|| self.environment.clone())
                .flatten()
        }
    }

    impl ConfigAccess for Project {
        fn get(
            &self,
            key: &Key,
            level: Option<Level>,
        ) -> Result<Resolved, ConfigError> {
            assert_eq!(key.to_string(), "crossref.mailto");
            let value = match level {
                Some(Level::Personal) if self.personal_unreadable => {
                    return Err(ConfigError::Io {
                        path: Level::Personal.filename().to_owned(),
                        detail: "permission denied".to_owned(),
                    })
                }
                Some(Level::Personal) => &self.personal,
                Some(Level::Team) => &self.team,
                None => panic!("the ladder always names a level"),
            };
            Ok(value.as_ref().map_or(Resolved::Absent, |value| {
                Resolved::Found(Value::Scalar(Scalar::String(value.clone())))
            }))
        }

        fn set(
            &self,
            _key: &Key,
            _value: &str,
            _level: Level,
        ) -> Result<(), ConfigError> {
            panic!("resolving a contact never writes")
        }

        fn personal_file(&self) -> &PersonalFile {
            &PersonalFile::Readable
        }
    }

    fn resolved(project: &Project) -> CrossrefContact {
        CrossrefContact::resolved(project, project)
    }

    #[test]
    fn a_plausible_address_selects_the_polite_pool() {
        let contact = resolved(&Project {
            team: Some(" research@example.org ".to_owned()),
            ..Project::default()
        });

        let CrossrefContact::Polite(address) = &contact else {
            panic!("{contact:?} is not polite");
        };
        assert_eq!(address.as_str(), "research@example.org");
        assert_eq!(contact.pool(), CrossrefPool::Polite);
        assert!(contact.warning().is_none());
    }

    #[test]
    fn no_address_anywhere_calls_anonymously_without_a_warning() {
        let contact = resolved(&Project::default());

        assert!(matches!(contact, CrossrefContact::Anonymous));
        assert_eq!(contact.pool(), CrossrefPool::Public);
        assert!(contact.warning().is_none());
        assert!(contact.address().is_none());
    }

    #[test]
    fn an_implausible_address_calls_anonymously_without_falling_through() {
        let contact = resolved(&Project {
            environment: Some("not-an-address".to_owned()),
            team: Some("research@example.org".to_owned()),
            ..Project::default()
        });

        assert_eq!(contact.pool(), CrossrefPool::Public);
        assert!(contact.address().is_none());
        assert!(matches!(
            contact.warning(),
            Some(ContactWarning::Implausible {
                source: SettingSource::Environment(VARIABLE),
                ..
            })
        ));
    }

    #[test]
    fn an_unreadable_level_calls_anonymously() {
        let contact = resolved(&Project {
            personal_unreadable: true,
            team: Some("research@example.org".to_owned()),
            ..Project::default()
        });

        assert_eq!(contact.pool(), CrossrefPool::Public);
        assert!(matches!(
            contact.warning(),
            Some(ContactWarning::Unreadable(_))
        ));
    }

    #[test]
    fn an_implausible_warning_names_its_source_and_value() {
        for (project, source) in [
            (
                Project {
                    environment: Some("a@b".to_owned()),
                    ..Project::default()
                },
                VARIABLE,
            ),
            (
                Project {
                    personal: Some("a@b".to_owned()),
                    ..Project::default()
                },
                "config.local.md",
            ),
            (
                Project {
                    team: Some("a@b".to_owned()),
                    ..Project::default()
                },
                "config.md",
            ),
        ] {
            let contact = resolved(&project);

            assert_eq!(
                contact.warning().map(ToString::to_string),
                Some(format!(
                    "warning: crossref.mailto from {source} ('a@b') is not a \
                     plausible address; calling Crossref anonymously without \
                     checking lower-precedence settings; correct or unset it \
                     (a blank value falls through to the next setting)"
                ))
            );
        }
    }

    #[test]
    fn an_unreadable_warning_names_the_file_and_its_reason() {
        let contact = resolved(&Project {
            personal_unreadable: true,
            ..Project::default()
        });

        assert_eq!(
            contact.warning().map(ToString::to_string),
            Some(
                "warning: crossref.mailto could not be read from \
                 config.local.md (I/O error on '.accelerator/config.local.md': \
                 permission denied); calling Crossref anonymously; fix or \
                 remove config.local.md to use its settings"
                    .to_owned()
            )
        );
    }
}
