//! The consent policy over in-memory ports: who may supply a consent key, how
//! refusals are ordered, and how severity is decided.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use config::catalogue::Trust;
use config::consent::{
    audit, resolve, resolve_command, resolve_executable_path,
    vet_executable_path, AuditFinding, CommandExecution, CommandFailure,
    CommandKey, CommandPolicy, CommandRunner, ConfigFileTracking, ConsentKey,
    Consented, Distrust, Environment, ExecutablePathKey, ExecutablePaths,
    FailureCause, Ladder, ProvenanceContext, Refusal, RefusalReason,
    RepositoryRoots, Rung, Runner, StartFailure, Tracking, TrackingCheck,
    Usable, SYMLINK_HOP_LIMIT,
};
use config::{
    ConfigAccess, ConfigError, Key, Level, PersonalFile, Resolved, Scalar,
    Value,
};

const PERSONAL: &str = "/project/.accelerator/config.local.md";

enum PersonalLevel {
    Values(BTreeMap<String, String>),
    Failing,
}

struct FixedConfig {
    team: BTreeMap<String, String>,
    personal: PersonalLevel,
    personal_file: PersonalFile,
    personal_reads: RefCell<usize>,
}

impl ConfigAccess for FixedConfig {
    fn get(
        &self,
        key: &Key,
        level: Option<Level>,
    ) -> Result<Resolved, ConfigError> {
        let map = match level {
            Some(Level::Team) => &self.team,
            Some(Level::Personal) => {
                *self.personal_reads.borrow_mut() += 1;
                match &self.personal {
                    PersonalLevel::Values(values) => values,
                    PersonalLevel::Failing => {
                        return Err(ConfigError::Io {
                            path: PERSONAL.to_owned(),
                            detail: "boom".to_owned(),
                        })
                    }
                }
            }
            None => unreachable!("the policy always names a level"),
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
        unreachable!("the policy never writes")
    }

    fn personal_file(&self) -> &PersonalFile {
        &self.personal_file
    }
}

struct FixedTracking {
    answer: Tracking,
    unchecked: bool,
    asked: RefCell<Vec<PathBuf>>,
}

impl ConfigFileTracking for FixedTracking {
    fn tracking(&self, path: &Path) -> Tracking {
        self.asked.borrow_mut().push(path.to_path_buf());
        self.answer
    }

    fn check(&self, path: &Path) -> TrackingCheck {
        if self.unchecked {
            self.asked.borrow_mut().push(path.to_path_buf());
            return TrackingCheck::Unchecked;
        }
        TrackingCheck::Known(self.tracking(path))
    }
}

struct FixedEnvironment(BTreeMap<String, String>);

impl Environment for FixedEnvironment {
    fn read(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

struct Project {
    config: FixedConfig,
    tracking: FixedTracking,
    environment: FixedEnvironment,
}

impl Project {
    const fn new() -> Self {
        Self {
            config: FixedConfig {
                team: BTreeMap::new(),
                personal: PersonalLevel::Values(BTreeMap::new()),
                personal_file: PersonalFile::Readable,
                personal_reads: RefCell::new(0),
            },
            tracking: FixedTracking {
                answer: Tracking::Untracked,
                unchecked: false,
                asked: RefCell::new(Vec::new()),
            },
            environment: FixedEnvironment(BTreeMap::new()),
        }
    }

    fn team(mut self, key: &str, value: &str) -> Self {
        self.config.team.insert(key.to_owned(), value.to_owned());
        self
    }

    fn personal(mut self, key: &str, value: &str) -> Self {
        if let PersonalLevel::Values(values) = &mut self.config.personal {
            values.insert(key.to_owned(), value.to_owned());
        }
        self
    }

    fn failing_personal_read(mut self) -> Self {
        self.config.personal = PersonalLevel::Failing;
        self
    }

    fn env(mut self, name: &str, value: &str) -> Self {
        self.environment.0.insert(name.to_owned(), value.to_owned());
        self
    }

    const fn tracking(mut self, answer: Tracking) -> Self {
        self.tracking.answer = answer;
        self
    }

    const fn unchecked(mut self) -> Self {
        self.tracking.unchecked = true;
        self
    }

    fn personal_file(mut self, fact: PersonalFile) -> Self {
        self.config.personal_file = fact;
        self
    }

    fn ignored_personal_file(self) -> Self {
        self.personal_file(PersonalFile::Ignored {
            path: PathBuf::from(PERSONAL),
            mode: 0o644,
        })
    }

    fn context(&self) -> ProvenanceContext<'_> {
        ProvenanceContext {
            config: &self.config,
            tracking: &self.tracking,
            environment: &self.environment,
            personal_config: PathBuf::from(PERSONAL),
        }
    }

    fn resolve(&self, name: &str) -> Consented<String> {
        self.try_resolve(name)
            .unwrap_or_else(|_| panic!("{name} aborted"))
    }

    fn try_resolve(
        &self,
        name: &str,
    ) -> Result<Consented<String>, config::consent::Aborted> {
        resolve(&self.context(), &ConsentKey::declared(name).unwrap())
    }

    fn personal_reads(&self) -> usize {
        *self.config.personal_reads.borrow()
    }
}

const ALLOWLIST: &str = "jira.allowed_sites";

fn codes(refusals: &[Refusal]) -> Vec<String> {
    refusals
        .iter()
        .map(|refusal| {
            refusal
                .to_string()
                .split(':')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

#[test]
fn a_team_value_alone_is_refused_naming_the_key_and_the_personal_route() {
    let consented = Project::new()
        .team(ALLOWLIST, "jira.example.com")
        .resolve(ALLOWLIST);

    assert_eq!(consented.admitted, None);
    assert_eq!(codes(&consented.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    let rendered = consented.refusals[0].to_string();
    assert!(rendered.contains(ALLOWLIST), "{rendered}");
    assert!(
        rendered.contains(".accelerator/config.local.md"),
        "{rendered}"
    );
}

#[test]
fn a_blank_personal_value_does_not_mask_a_team_value() {
    for blank in ["", "   "] {
        let consented = Project::new()
            .team(ALLOWLIST, "jira.example.com")
            .personal(ALLOWLIST, blank)
            .resolve(ALLOWLIST);

        assert_eq!(consented.admitted, None, "{blank:?}");
        assert_eq!(codes(&consented.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    }
}

#[test]
fn a_usable_personal_value_wins_beside_a_team_refusal() {
    let consented = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .personal(ALLOWLIST, "mine.example.com")
        .resolve(ALLOWLIST);

    assert_eq!(consented.admitted.as_deref(), Some("mine.example.com"));
    assert_eq!(codes(&consented.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    assert!(consented.notice.is_none());
}

#[test]
fn a_tracked_or_unknown_personal_file_is_distrusted() {
    for (answer, distrust, code) in [
        (
            Tracking::Tracked,
            Distrust::Tracked,
            "E_CONSENT_KEY_TRACKED",
        ),
        (
            Tracking::Unknown,
            Distrust::Unknown,
            "E_CONSENT_KEY_TRACKING_UNKNOWN",
        ),
    ] {
        let consented = Project::new()
            .personal(ALLOWLIST, "mine.example.com")
            .tracking(answer)
            .resolve(ALLOWLIST);

        assert_eq!(consented.admitted, None);
        assert!(matches!(
            &consented.refusals[..],
            [Refusal::UntrustedPersonalFile { distrust: found, path, .. }]
                if *found == distrust && path == Path::new(PERSONAL)
        ));
        assert_eq!(codes(&consented.refusals), [code]);
    }
}

#[test]
fn an_untracked_personal_file_is_trusted_and_an_absent_one_never_asked_about() {
    let project = Project::new().personal(ALLOWLIST, "mine.example.com");
    assert_eq!(
        project.resolve(ALLOWLIST).admitted.as_deref(),
        Some("mine.example.com")
    );

    let absent = Project::new()
        .personal(ALLOWLIST, "mine.example.com")
        .personal_file(PersonalFile::Absent);
    let consented = absent.resolve(ALLOWLIST);
    assert_eq!(consented.admitted, None);
    assert!(consented.refusals.is_empty());
    assert_eq!(absent.personal_reads(), 0);
    assert!(absent.tracking.asked.borrow().is_empty());
}

#[test]
fn an_environment_value_wins_without_reading_the_personal_level() {
    let project = Project::new()
        .personal(ALLOWLIST, "mine.example.com")
        .tracking(Tracking::Tracked)
        .failing_personal_read()
        .team(ALLOWLIST, "team.example.com")
        .env("ACCELERATOR_JIRA_ALLOWED_SITES", "env.example.com");

    let consented = project.resolve(ALLOWLIST);

    assert_eq!(consented.admitted.as_deref(), Some("env.example.com"));
    assert_eq!(codes(&consented.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    assert_eq!(project.personal_reads(), 0);
    assert!(project.tracking.asked.borrow().is_empty());
}

#[test]
fn a_blank_environment_value_falls_through_to_the_personal_level() {
    let consented = Project::new()
        .env("ACCELERATOR_JIRA_ALLOWED_SITES", "  ")
        .personal(ALLOWLIST, "mine.example.com")
        .resolve(ALLOWLIST);

    assert_eq!(consented.admitted.as_deref(), Some("mine.example.com"));
    assert!(consented.notice.is_none());
}

#[test]
fn a_failed_personal_read_aborts_carrying_the_team_refusal() {
    let Err(aborted) = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .failing_personal_read()
        .try_resolve(ALLOWLIST)
    else {
        panic!("a failed personal read must abort");
    };

    assert!(matches!(aborted.error, ConfigError::Io { .. }));
    assert_eq!(codes(&aborted.warnings), ["E_CONSENT_KEY_TEAM_LEVEL"]);
}

#[test]
fn refusals_run_in_precedence_order_with_the_team_level_last() {
    let consented = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .personal(ALLOWLIST, "mine.example.com")
        .tracking(Tracking::Tracked)
        .resolve(ALLOWLIST);

    assert_eq!(
        codes(&consented.refusals),
        ["E_CONSENT_KEY_TRACKED", "E_CONSENT_KEY_TEAM_LEVEL"]
    );

    let Usable::Refused(rejection) = consented.or_fallback(None) else {
        panic!("no usable value remains");
    };
    assert_eq!(codes(&[rejection.fatal]), ["E_CONSENT_KEY_TRACKED"]);
    assert_eq!(codes(&rejection.warnings), ["E_CONSENT_KEY_TEAM_LEVEL"]);
}

#[test]
fn a_fallback_keeps_every_refusal_as_a_warning() {
    let consented = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .personal(ALLOWLIST, "mine.example.com")
        .tracking(Tracking::Unknown)
        .resolve(ALLOWLIST);

    let Usable::Value {
        value,
        warnings,
        notice,
    } = consented.or_fallback(Some("fallback".to_owned()))
    else {
        panic!("the fallback is usable");
    };
    assert_eq!(value, "fallback");
    assert_eq!(
        codes(&warnings),
        ["E_CONSENT_KEY_TRACKING_UNKNOWN", "E_CONSENT_KEY_TEAM_LEVEL"]
    );
    assert!(notice.is_none());
}

#[test]
fn nothing_set_and_no_fallback_is_absent() {
    assert!(matches!(
        Project::new().resolve(ALLOWLIST).or_fallback(None),
        Usable::Absent
    ));
}

#[test]
fn an_environment_winner_carries_a_notice_naming_the_variable_and_value() {
    let consented = Project::new()
        .env("ACCELERATOR_JIRA_ALLOWED_SITES", "env.example.com")
        .resolve(ALLOWLIST);

    let Usable::Value { notice, .. } = consented.or_fallback(None) else {
        panic!("the environment value is usable");
    };
    let rendered = notice.expect("an environment winner notices").to_string();
    assert_eq!(
        rendered,
        "notice: jira.allowed_sites taken from \
         ACCELERATOR_JIRA_ALLOWED_SITES: env.example.com"
    );
}

#[test]
fn map_transforms_the_admitted_value_and_keeps_refusals_and_notice() {
    let consented = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .env("ACCELERATOR_JIRA_ALLOWED_SITES", "a.example.com")
        .resolve(ALLOWLIST)
        .map(|value| value.len());

    assert_eq!(consented.admitted, Some("a.example.com".len()));
    assert_eq!(codes(&consented.refusals), ["E_CONSENT_KEY_TEAM_LEVEL"]);
    assert!(consented.notice.is_some());
}

#[test]
fn an_ignored_personal_file_is_refused_once_without_being_read() {
    let project = Project::new()
        .personal(ALLOWLIST, "mine.example.com")
        .ignored_personal_file();

    let consented = project.resolve(ALLOWLIST);

    assert_eq!(consented.admitted, None);
    assert_eq!(codes(&consented.refusals), ["E_LOCAL_PERMS_INSECURE"]);
    assert_eq!(consented.refusals[0].reason(), RefusalReason::PersonalFile);
    assert!(consented.refusals[0].key().is_none());
    assert_eq!(project.personal_reads(), 0);
}

#[test]
fn an_ignored_personal_file_is_fatal_only_when_nothing_usable_remains() {
    let consented = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .ignored_personal_file()
        .resolve(ALLOWLIST);
    let Usable::Refused(rejection) = consented.or_fallback(None) else {
        panic!("nothing usable remains");
    };
    assert_eq!(codes(&[rejection.fatal]), ["E_LOCAL_PERMS_INSECURE"]);
    assert_eq!(codes(&rejection.warnings), ["E_CONSENT_KEY_TEAM_LEVEL"]);

    let consented = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .ignored_personal_file()
        .resolve(ALLOWLIST);
    let Usable::Value { warnings, .. } =
        consented.or_fallback(Some(String::new()))
    else {
        panic!("the fallback is usable");
    };
    assert_eq!(
        codes(&warnings),
        ["E_LOCAL_PERMS_INSECURE", "E_CONSENT_KEY_TEAM_LEVEL"]
    );
}

#[test]
fn an_environment_winner_reports_no_ignored_personal_file() {
    let consented = Project::new()
        .ignored_personal_file()
        .env("ACCELERATOR_JIRA_ALLOWED_SITES", "env.example.com")
        .resolve(ALLOWLIST);

    assert!(consented.refusals.is_empty());
}

#[test]
fn the_insecure_file_refusal_names_the_routes_out() {
    let consented = Project::new().ignored_personal_file().resolve(ALLOWLIST);
    let rendered = consented.refusals[0].to_string();

    assert!(
        rendered.starts_with(
            "E_LOCAL_PERMS_INSECURE: /project/.accelerator/config.local.md \
             is mode 0644; ignored"
        ),
        "{rendered}"
    );
    assert!(rendered.contains("chmod 600"), "{rendered}");
    assert!(rendered.contains("config.md"), "{rendered}");
    assert!(rendered.contains("ACCELERATOR_*"), "{rendered}");
}

#[test]
fn audit_reports_each_team_level_consent_key() {
    let project = Project::new()
        .team("jira.token_cmd", "op read x")
        .team("design.browser_path", "/opt/chrome")
        .team("jira.token", "plaintext");

    let findings = audit(&project.context()).unwrap();

    let keys: Vec<&str> = findings
        .iter()
        .filter_map(|finding| match finding {
            AuditFinding::Key(Refusal::TeamLevel { key }) => Some(key.name),
            _ => None,
        })
        .collect();
    assert_eq!(keys, ["jira.token_cmd", "design.browser_path"]);
}

#[test]
fn audit_reports_a_distrusted_personal_file_that_sets_no_keys() {
    for (answer, distrust) in [
        (Tracking::Tracked, Distrust::Tracked),
        (Tracking::Unknown, Distrust::Unknown),
    ] {
        let findings =
            audit(&Project::new().tracking(answer).context()).unwrap();
        assert!(
            matches!(&findings[..], [AuditFinding::PersonalFile(found)] if *found == distrust)
        );
    }
}

#[test]
fn audit_is_silent_for_an_untracked_file_and_never_asks_about_an_absent_one() {
    assert!(audit(&Project::new().context()).unwrap().is_empty());

    let absent = Project::new()
        .tracking(Tracking::Tracked)
        .personal_file(PersonalFile::Absent);
    assert!(audit(&absent.context()).unwrap().is_empty());
    assert!(absent.tracking.asked.borrow().is_empty());
}

#[test]
fn audit_reports_an_ignored_tracked_file_both_ways() {
    let findings = audit(
        &Project::new()
            .ignored_personal_file()
            .tracking(Tracking::Tracked)
            .context(),
    )
    .unwrap();

    assert!(findings.iter().any(|finding| matches!(
        finding,
        AuditFinding::PersonalFileIgnored(Refusal::InsecurePersonalFile { .. })
    )));
    assert!(findings.iter().any(|finding| matches!(
        finding,
        AuditFinding::PersonalFile(Distrust::Tracked)
    )));
}

#[test]
fn audit_tells_an_unchecked_file_apart_from_an_unknown_one() {
    let findings = audit(&Project::new().unchecked().context()).unwrap();

    assert!(matches!(
        &findings[..],
        [AuditFinding::PersonalFileUnchecked]
    ));
}

#[test]
fn the_default_check_wraps_the_tracking_answer() {
    struct Plain;
    impl ConfigFileTracking for Plain {
        fn tracking(&self, _path: &Path) -> Tracking {
            Tracking::Tracked
        }
    }

    assert!(matches!(
        Plain.check(Path::new(PERSONAL)),
        TrackingCheck::Known(Tracking::Tracked)
    ));
}

#[test]
fn distrust_owns_the_tracking_codes() {
    assert_eq!(Distrust::Tracked.to_string(), "E_CONSENT_KEY_TRACKED");
    assert_eq!(
        Distrust::Unknown.to_string(),
        "E_CONSENT_KEY_TRACKING_UNKNOWN"
    );
    assert_eq!(Tracking::Untracked.distrust(), None);
    assert_eq!(Tracking::Tracked.distrust(), Some(Distrust::Tracked));
    assert_eq!(Tracking::Unknown.distrust(), Some(Distrust::Unknown));
}

#[test]
fn an_unknown_tracking_refusal_names_each_consent_keys_recovery_hint() {
    for (name, hint) in [
        ("jira.allowed_sites", "ACCELERATOR_JIRA_ALLOWED_SITES"),
        ("jira.token_cmd", "ACCELERATOR_JIRA_TOKEN_CMD"),
        ("linear.token_cmd", "ACCELERATOR_LINEAR_TOKEN_CMD"),
        ("github.token_cmd", "GH_TOKEN"),
        ("openalex.api_key_cmd", "ACCELERATOR_OPENALEX_API_KEY_CMD"),
        ("design.browser_path", "ACCELERATOR_DESIGN_BROWSER_PATH"),
    ] {
        let key = config::catalogue::declared(name).unwrap();
        let rendered = Refusal::UntrustedPersonalFile {
            key,
            path: PathBuf::from(PERSONAL),
            distrust: Distrust::Unknown,
        }
        .to_string();
        assert!(
            rendered.ends_with(&format!("set {hint} in the environment")),
            "{rendered}"
        );
    }
}

#[test]
fn every_refusal_renders_its_code_and_debug_redacts_the_path() {
    let key = config::catalogue::declared(ALLOWLIST).unwrap();
    for (refusal, code) in [
        (Refusal::TeamLevel { key }, "E_CONSENT_KEY_TEAM_LEVEL"),
        (
            Refusal::UntrustedPersonalFile {
                key,
                path: PathBuf::from("/secret/place/config.local.md"),
                distrust: Distrust::Tracked,
            },
            "E_CONSENT_KEY_TRACKED",
        ),
        (
            Refusal::InsecurePersonalFile {
                path: PathBuf::from("/secret/place/config.local.md"),
                mode: 0o640,
            },
            "E_LOCAL_PERMS_INSECURE",
        ),
    ] {
        assert!(refusal.to_string().starts_with(code), "{refusal}");
        let debugged = format!("{refusal:?}");
        assert!(!debugged.contains("/secret/place"), "{debugged}");
        if refusal.key().is_some() {
            assert!(debugged.contains(ALLOWLIST), "{debugged}");
        }
    }
}

#[test]
fn control_characters_in_paths_and_values_render_escaped() {
    let key = config::catalogue::declared(ALLOWLIST).unwrap();
    let rendered = Refusal::UntrustedPersonalFile {
        key,
        path: PathBuf::from("/p\r\n\x1b[2Kforged"),
        distrust: Distrust::Tracked,
    }
    .to_string();
    assert!(!rendered.contains(['\r', '\n', '\x1b']), "{rendered:?}");

    let consented = Project::new()
        .env(
            "ACCELERATOR_JIRA_ALLOWED_SITES",
            "a.example.com\n\x1b[31mfake",
        )
        .resolve(ALLOWLIST);
    let notice = consented.notice.unwrap().to_string();
    assert!(!notice.contains(['\r', '\n', '\x1b']), "{notice:?}");
}

#[test]
fn a_command_key_notice_never_shows_the_command() {
    let key = config::catalogue::declared("jira.token_cmd").unwrap();
    let notice = config::consent::Notice::new(
        key,
        "ACCELERATOR_JIRA_TOKEN_CMD",
        "op read op://vault/secret",
    );

    assert_eq!(
        notice.to_string(),
        "notice: jira.token_cmd taken from ACCELERATOR_JIRA_TOKEN_CMD"
    );
}

#[test]
fn a_path_key_notice_shows_the_path() {
    let key = config::catalogue::declared("design.browser_path").unwrap();
    let notice = config::consent::Notice::new(
        key,
        "ACCELERATOR_DESIGN_BROWSER_PATH",
        "/opt/chrome",
    );

    assert_eq!(
        notice.to_string(),
        "notice: design.browser_path taken from \
         ACCELERATOR_DESIGN_BROWSER_PATH: /opt/chrome"
    );
}

#[test]
fn kind_specific_constructors_refuse_other_kinds_and_undeclared_names() {
    assert!(ConsentKey::declared(ALLOWLIST).is_ok());
    assert!(ConsentKey::declared("jira.token_cmd").is_err());
    assert!(ConsentKey::declared("jira.site").is_err());
    assert!(ConsentKey::declared("no.such.key").is_err());

    assert!(CommandKey::declared("github.token_cmd").is_ok());
    assert!(CommandKey::declared(ALLOWLIST).is_err());
    assert!(CommandKey::declared("design.browser_path").is_err());
    assert!(CommandKey::declared("no.such.key").is_err());

    assert!(ExecutablePathKey::declared("design.browser_path").is_ok());
    assert!(ExecutablePathKey::declared("jira.token_cmd").is_err());
    assert!(ExecutablePathKey::declared(ALLOWLIST).is_err());
    assert!(ExecutablePathKey::declared("no.such.key").is_err());
}

#[test]
fn a_test_consent_key_is_only_ever_a_plain_consent_key() {
    let key = ConsentKey::for_test("example.hatch");

    assert_eq!(key.descriptor().trust, Trust::Consent);
    assert_eq!(key.descriptor().name, "example.hatch");
}

/// The one policy: every plain consent key is refused by the same code for
/// the same reason, differing only in the key it names.
#[test]
fn every_plain_consent_key_is_refused_by_one_policy() {
    let keys = [
        ConsentKey::for_test("example.hatch"),
        ConsentKey::declared(ALLOWLIST).unwrap(),
    ];
    for key in keys {
        let name = key.descriptor().name;
        let cases: [(Project, &str); 3] = [
            (
                Project::new().team(name, "value"),
                "E_CONSENT_KEY_TEAM_LEVEL",
            ),
            (
                Project::new()
                    .personal(name, "value")
                    .tracking(Tracking::Tracked),
                "E_CONSENT_KEY_TRACKED",
            ),
            (
                Project::new()
                    .personal(name, "value")
                    .tracking(Tracking::Unknown),
                "E_CONSENT_KEY_TRACKING_UNKNOWN",
            ),
        ];
        for (project, code) in cases {
            let consented = resolve(&project.context(), &key).unwrap();
            assert_eq!(consented.admitted, None, "{name}");
            assert_eq!(codes(&consented.refusals), [code], "{name}");
            assert_eq!(
                consented.refusals[0].key().map(|key| key.name),
                Some(name)
            );
            assert_eq!(
                consented.refusals[0].reason(),
                RefusalReason::Provenance
            );
        }
    }
}

#[test]
fn a_consumer_leaves_the_ignored_file_to_its_composition_root() {
    let consented = Project::new()
        .team(ALLOWLIST, "team.example.com")
        .ignored_personal_file()
        .resolve(ALLOWLIST);

    let reported: Vec<Refusal> =
        config::consent::reportable(&consented.refusals)
            .cloned()
            .collect();

    assert_eq!(codes(&reported), ["E_CONSENT_KEY_TEAM_LEVEL"]);
}

#[test]
fn each_command_refusal_renders_its_code_and_cause() {
    let key = config::catalogue::declared("jira.token_cmd").unwrap();
    for (refusal, rendered) in [
        (
            Refusal::CommandFailed {
                key,
                cause: FailureCause::CouldNotStart(StartFailure::NoBashOnPath),
            },
            "E_TOKEN_CMD_FAILED: jira.token_cmd could not start: no bash on \
             the filtered PATH",
        ),
        (
            Refusal::CommandFailed {
                key,
                cause: FailureCause::CouldNotStart(StartFailure::SpawnFailed),
            },
            "E_TOKEN_CMD_FAILED: jira.token_cmd could not start",
        ),
        (
            Refusal::CommandFailed {
                key,
                cause: FailureCause::Exited(3),
            },
            "E_TOKEN_CMD_FAILED: jira.token_cmd exited with status 3",
        ),
        (
            Refusal::CommandTimedOut {
                key,
                after: Duration::from_secs(9),
            },
            "E_COMMAND_TIMED_OUT: jira.token_cmd did not finish within 9s",
        ),
        (
            Refusal::CommandTimedOut {
                key,
                after: Duration::from_millis(2_500),
            },
            "E_COMMAND_TIMED_OUT: jira.token_cmd did not finish within 2.5s",
        ),
        (
            Refusal::CommandOutputExceeded {
                key,
                limit: CommandPolicy::OUTPUT_LIMIT,
            },
            "E_COMMAND_OUTPUT_EXCEEDED: jira.token_cmd printed more than \
             65536 bytes",
        ),
    ] {
        assert_eq!(refusal.to_string(), rendered);
        assert_eq!(refusal.reason(), RefusalReason::Command);
        assert_eq!(refusal.key(), Some(key));
        assert!(format!("{refusal:?}").contains("jira.token_cmd"));
    }
}

#[test]
fn repository_roots_contain_every_path_beneath_a_root() {
    let roots = RepositoryRoots::complete(vec![
        PathBuf::from("/work/repo"),
        PathBuf::from("/work/main"),
    ]);

    assert!(roots.contains(Path::new("/work/repo")));
    assert!(roots.contains(Path::new("/work/main/bin/tool")));
    assert!(!roots.contains(Path::new("/work/repository")));
    assert!(!roots.contains(Path::new("/usr/bin")));
    assert!(roots.is_complete());
    assert!(!RepositoryRoots::incomplete(Vec::new()).is_complete());
}

#[test]
fn a_command_policy_for_a_test_carries_its_bounds() {
    let policy =
        CommandPolicy::for_test(Duration::from_secs(4), &["PATH", "GH_HOST"]);

    assert_eq!(policy.timeout(), Duration::from_secs(4));
    assert_eq!(policy.admitted_environment(), ["PATH", "GH_HOST"]);
}

#[derive(Clone)]
struct RecordingRunner {
    outcome: std::rc::Rc<RefCell<Result<String, CommandFailure>>>,
    runs: std::rc::Rc<RefCell<Vec<(String, CommandPolicy)>>>,
}

impl RecordingRunner {
    fn answering(outcome: Result<String, CommandFailure>) -> Self {
        Self {
            outcome: std::rc::Rc::new(RefCell::new(outcome)),
            runs: std::rc::Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn commands(&self) -> Vec<String> {
        self.runs
            .borrow()
            .iter()
            .map(|(command, _)| command.clone())
            .collect()
    }
}

impl CommandRunner for RecordingRunner {
    fn run(
        &self,
        command: &str,
        policy: &CommandPolicy,
    ) -> Result<String, CommandFailure> {
        self.runs
            .borrow_mut()
            .push((command.to_owned(), policy.clone()));
        self.outcome.borrow().clone()
    }
}

const fn execution(runner: &Runner) -> CommandExecution<'_> {
    CommandExecution {
        runner,
        timeout: Duration::from_secs(7),
    }
}

fn plaintext() -> &'static config::catalogue::ExtraKey {
    config::catalogue::declared("jira.token").unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    First,
    Second,
    Third,
}

#[test]
fn a_consented_command_runs_under_its_keys_policy_and_the_given_timeout() {
    let recording = RecordingRunner::answering(Ok("token".to_owned()));
    let runner = Runner::new(Box::new(recording.clone()));
    let project = Project::new().personal("github.token_cmd", "gh auth token");
    let context = project.context();
    let candidates = resolve_command(
        &context,
        CommandKey::declared("github.token_cmd").unwrap(),
    )
    .unwrap();

    let Rung::Candidate(command) = candidates.personal().unwrap() else {
        panic!("an untracked personal command is a candidate");
    };
    let value = command.run(&execution(&runner)).unwrap();

    assert_eq!(value, "token");
    let runs = recording.runs.borrow();
    assert_eq!(runs[0].0, "gh auth token");
    assert_eq!(runs[0].1.timeout(), Duration::from_secs(7));
    assert_eq!(
        runs[0].1.admitted_environment(),
        [
            config::catalogue::BASE_COMMAND_ENVIRONMENT,
            &["GH_HOST", "GH_CONFIG_DIR"]
        ]
        .concat()
    );
}

#[test]
fn a_consented_command_maps_each_failure_to_its_refusal() {
    let key = config::catalogue::declared("jira.token_cmd").unwrap();
    for (failure, code) in [
        (
            CommandFailure::Failed(FailureCause::Exited(1)),
            "E_TOKEN_CMD_FAILED",
        ),
        (CommandFailure::TimedOut, "E_COMMAND_TIMED_OUT"),
        (CommandFailure::OutputExceeded, "E_COMMAND_OUTPUT_EXCEEDED"),
    ] {
        let runner =
            Runner::new(Box::new(RecordingRunner::answering(Err(failure))));
        let project = Project::new().env("ACCELERATOR_JIRA_TOKEN_CMD", "x");
        let context = project.context();
        let candidates = resolve_command(
            &context,
            CommandKey::declared("jira.token_cmd").unwrap(),
        )
        .unwrap();
        let Rung::Candidate(command) = candidates.environment() else {
            panic!("the environment command is a candidate");
        };

        let refusal = command.run(&execution(&runner)).unwrap_err();

        assert_eq!(codes(std::slice::from_ref(&refusal)), [code]);
        assert_eq!(refusal.key(), Some(key));
    }
}

#[test]
fn command_candidates_carry_the_team_refusal_and_read_the_personal_level_lazily(
) {
    let project = Project::new()
        .team("jira.token_cmd", "op read team")
        .personal("jira.token_cmd", "op read mine")
        .env("ACCELERATOR_JIRA_TOKEN_CMD", "op read env");
    let context = project.context();

    let candidates = resolve_command(
        &context,
        CommandKey::declared("jira.token_cmd").unwrap(),
    )
    .unwrap();

    assert_eq!(
        codes(candidates.team_level_refusals()),
        ["E_CONSENT_KEY_TEAM_LEVEL"]
    );
    assert!(matches!(candidates.environment(), Rung::Candidate(_)));
    assert_eq!(project.personal_reads(), 0);
    assert!(matches!(candidates.personal(), Ok(Rung::Candidate(_))));
    assert_eq!(project.personal_reads(), 1);
}

#[test]
fn a_distrusted_personal_command_is_refused_without_running() {
    for (answer, code) in [
        (Tracking::Tracked, "E_CONSENT_KEY_TRACKED"),
        (Tracking::Unknown, "E_CONSENT_KEY_TRACKING_UNKNOWN"),
    ] {
        let project = Project::new()
            .personal("linear.token_cmd", "op read mine")
            .tracking(answer);
        let context = project.context();
        let candidates = resolve_command(
            &context,
            CommandKey::declared("linear.token_cmd").unwrap(),
        )
        .unwrap();

        let Ok(Rung::Refused(refusal)) = candidates.personal() else {
            panic!("a distrusted personal command is refused");
        };
        assert_eq!(codes(&[refusal]), [code]);
    }
}

#[test]
fn the_first_offered_value_wins_and_later_rungs_are_never_consulted() {
    let recording = RecordingRunner::answering(Ok("from-helper".to_owned()));
    let runner = Runner::new(Box::new(recording.clone()));
    let project = Project::new().personal("jira.token_cmd", "op read mine");
    let context = project.context();
    let candidates = resolve_command(
        &context,
        CommandKey::declared("jira.token_cmd").unwrap(),
    )
    .unwrap();
    let mut ladder = Ladder::new(Vec::new());

    ladder
        .offer(Step::First, plaintext(), || {
            Ok(Rung::Candidate("first".to_owned()))
        })
        .unwrap();
    ladder
        .offer(Step::Second, plaintext(), || {
            panic!("a later offer is never read")
        })
        .unwrap();
    ladder
        .attempt(Step::Third, || candidates.personal(), &execution(&runner))
        .unwrap();

    let consented = ladder.finish();
    let admitted = consented.admitted.unwrap();
    assert_eq!(admitted.value, "first");
    assert_eq!(admitted.source, Step::First);
    assert!(recording.commands().is_empty());
    assert_eq!(project.personal_reads(), 0);
}

#[test]
fn a_failed_attempt_records_its_refusal_and_falls_through() {
    let runner = Runner::new(Box::new(RecordingRunner::answering(Err(
        CommandFailure::TimedOut,
    ))));
    let project = Project::new().env("ACCELERATOR_JIRA_TOKEN_CMD", "slow");
    let context = project.context();
    let candidates = resolve_command(
        &context,
        CommandKey::declared("jira.token_cmd").unwrap(),
    )
    .unwrap();
    let mut ladder = Ladder::new(Vec::new());

    ladder
        .attempt(
            Step::First,
            || Ok(candidates.environment()),
            &execution(&runner),
        )
        .unwrap();
    ladder
        .offer(Step::Second, plaintext(), || {
            Ok(Rung::Candidate("second".to_owned()))
        })
        .unwrap();

    let consented = ladder.finish();
    assert_eq!(consented.admitted.unwrap().source, Step::Second);
    assert_eq!(codes(&consented.refusals), ["E_COMMAND_TIMED_OUT"]);
    assert!(consented.notice.is_none());
}

#[test]
fn finish_orders_rung_refusals_before_the_seeded_team_level_ones() {
    let team = Refusal::TeamLevel {
        key: config::catalogue::declared("jira.token_cmd").unwrap(),
    };
    let mut ladder: Ladder<Step> = Ladder::new(vec![team]);

    ladder
        .offer(Step::First, plaintext(), || {
            Ok(Rung::Refused(Refusal::PlaintextFromUntrustedFile {
                key: plaintext(),
                path: PathBuf::from(PERSONAL),
                distrust: Distrust::Tracked,
            }))
        })
        .unwrap();

    let Usable::Refused(rejection) = ladder.finish().or_fallback(None) else {
        panic!("nothing usable remains");
    };
    assert_eq!(codes(&[rejection.fatal]), ["E_TOKEN_FROM_TRACKED_FILE"]);
    assert_eq!(codes(&rejection.warnings), ["E_CONSENT_KEY_TEAM_LEVEL"]);
}

#[test]
fn an_environment_command_carries_a_notice_and_an_environment_value_none() {
    let runner =
        Runner::new(Box::new(RecordingRunner::answering(Ok("tok".to_owned()))));
    let project = Project::new().env("ACCELERATOR_JIRA_TOKEN_CMD", "op read x");
    let context = project.context();
    let candidates = resolve_command(
        &context,
        CommandKey::declared("jira.token_cmd").unwrap(),
    )
    .unwrap();

    let mut commanded: Ladder<Step> = Ladder::new(Vec::new());
    commanded
        .attempt(
            Step::First,
            || Ok(candidates.environment()),
            &execution(&runner),
        )
        .unwrap();
    let notice = commanded.finish().notice.unwrap().to_string();
    assert_eq!(
        notice,
        "notice: jira.token_cmd taken from ACCELERATOR_JIRA_TOKEN_CMD"
    );

    let mut offered: Ladder<Step> = Ladder::new(Vec::new());
    offered
        .offer(Step::First, plaintext(), || {
            Ok(Rung::Candidate("tok".to_owned()))
        })
        .unwrap();
    assert!(offered.finish().notice.is_none());
}

#[test]
fn a_value_carrying_a_control_character_is_refused_and_falls_through() {
    let runner = Runner::new(Box::new(RecordingRunner::answering(Ok(
        "tok\u{1}".to_owned(),
    ))));
    let project = Project::new().env("ACCELERATOR_JIRA_TOKEN_CMD", "op read x");
    let context = project.context();
    let candidates = resolve_command(
        &context,
        CommandKey::declared("jira.token_cmd").unwrap(),
    )
    .unwrap();
    let mut ladder = Ladder::new(Vec::new());

    ladder
        .offer(Step::First, plaintext(), || {
            Ok(Rung::Candidate("abc\r\ndef".to_owned()))
        })
        .unwrap();
    ladder
        .attempt(
            Step::Second,
            || Ok(candidates.environment()),
            &execution(&runner),
        )
        .unwrap();
    ladder
        .offer(Step::Third, plaintext(), || {
            Ok(Rung::Candidate("clean".to_owned()))
        })
        .unwrap();

    let consented = ladder.finish();
    assert_eq!(consented.admitted.unwrap().value, "clean");
    assert_eq!(
        codes(&consented.refusals),
        ["E_TOKEN_MALFORMED", "E_TOKEN_MALFORMED"]
    );
    assert_eq!(consented.refusals[0].key(), Some(plaintext()));
    assert_eq!(consented.refusals[0].reason(), RefusalReason::Malformed);
    assert!(consented.notice.is_none());
}

#[test]
fn a_failed_rung_read_aborts_carrying_every_refusal_so_far() {
    let team = Refusal::TeamLevel {
        key: config::catalogue::declared("jira.token_cmd").unwrap(),
    };
    let mut ladder: Ladder<Step> = Ladder::new(vec![team]);
    ladder
        .offer(Step::First, plaintext(), || {
            Ok(Rung::Refused(Refusal::MalformedToken { key: plaintext() }))
        })
        .unwrap();

    let aborted = ladder
        .offer(Step::Second, plaintext(), || {
            Err(ConfigError::Io {
                path: PERSONAL.to_owned(),
                detail: "boom".to_owned(),
            })
        })
        .unwrap_err();

    assert!(matches!(aborted.error, ConfigError::Io { .. }));
    assert_eq!(
        codes(&aborted.warnings),
        ["E_TOKEN_MALFORMED", "E_CONSENT_KEY_TEAM_LEVEL"]
    );
}

#[test]
fn a_plaintext_refusal_renders_its_code_and_the_keys_recovery_hint() {
    for (distrust, ending) in [
        (Distrust::Tracked, "untrack it"),
        (
            Distrust::Unknown,
            "set ACCELERATOR_JIRA_TOKEN in the environment",
        ),
    ] {
        let rendered = Refusal::PlaintextFromUntrustedFile {
            key: plaintext(),
            path: PathBuf::from(PERSONAL),
            distrust,
        }
        .to_string();

        assert!(
            rendered.starts_with(&format!(
                "E_TOKEN_FROM_TRACKED_FILE: jira.token in {PERSONAL} is \
                 refused"
            )),
            "{rendered}"
        );
        assert!(rendered.ends_with(ending), "{rendered}");
    }
    let github = config::catalogue::declared("github.token").unwrap();
    let rendered = Refusal::PlaintextFromUntrustedFile {
        key: github,
        path: PathBuf::from(PERSONAL),
        distrust: Distrust::Unknown,
    }
    .to_string();
    assert!(
        rendered.ends_with("set GH_TOKEN in the environment"),
        "{rendered}"
    );
}

/// The one policy, for command keys: every command key is refused at team
/// level, tracked and unknown by the same code path, and reaches the runner
/// through `resolve_command` and `Ladder::attempt` under its own policy.
#[test]
fn every_command_key_is_refused_and_run_by_one_policy() {
    let keys = [
        CommandKey::for_test("example.hatch_cmd", &["EXAMPLE_HOST"]),
        CommandKey::declared("jira.token_cmd").unwrap(),
        CommandKey::declared("linear.token_cmd").unwrap(),
        CommandKey::declared("github.token_cmd").unwrap(),
        CommandKey::declared("openalex.api_key_cmd").unwrap(),
    ];
    for key in keys {
        let name = key.descriptor().name;
        for (project, code) in [
            (Project::new().team(name, "cmd"), "E_CONSENT_KEY_TEAM_LEVEL"),
            (
                Project::new()
                    .personal(name, "cmd")
                    .tracking(Tracking::Tracked),
                "E_CONSENT_KEY_TRACKED",
            ),
            (
                Project::new()
                    .personal(name, "cmd")
                    .tracking(Tracking::Unknown),
                "E_CONSENT_KEY_TRACKING_UNKNOWN",
            ),
        ] {
            let recording = RecordingRunner::answering(Ok("tok".to_owned()));
            let runner = Runner::new(Box::new(recording.clone()));
            let context = project.context();
            let candidates = resolve_command(&context, key).unwrap();
            let mut ladder: Ladder<Step> =
                Ladder::new(candidates.team_level_refusals().to_vec());
            ladder
                .attempt(
                    Step::First,
                    || candidates.personal(),
                    &execution(&runner),
                )
                .unwrap();

            let consented = ladder.finish();
            assert_eq!(consented.admitted, None, "{name}");
            assert_eq!(codes(&consented.refusals), [code], "{name}");
            assert_eq!(consented.refusals[0].key(), Some(key.descriptor()));
            assert!(recording.commands().is_empty(), "{name}");
        }

        let recording = RecordingRunner::answering(Ok("tok".to_owned()));
        let runner = Runner::new(Box::new(recording.clone()));
        let project = Project::new().personal(name, "cmd");
        let context = project.context();
        let candidates = resolve_command(&context, key).unwrap();
        let mut ladder: Ladder<Step> = Ladder::new(Vec::new());
        ladder
            .attempt(Step::First, || candidates.personal(), &execution(&runner))
            .unwrap();

        assert_eq!(ladder.finish().admitted.unwrap().value, "tok");
        let Trust::CommandConsent {
            admitted_environment: declared,
        } = key.descriptor().trust
        else {
            unreachable!("a command key")
        };
        assert_eq!(
            recording.runs.borrow()[0].1.admitted_environment(),
            [config::catalogue::BASE_COMMAND_ENVIRONMENT, declared].concat(),
            "{name}"
        );
    }
}

const BROWSER: &str = "design.browser_path";

/// An in-memory filesystem: entries that exist, and links whose targets are
/// stored exactly as written.
#[derive(Default)]
struct FakePaths {
    entries: BTreeSet<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
    uncanonicalisable: bool,
}

impl FakePaths {
    fn with_entry(mut self, path: &str) -> Self {
        self.entries.insert(PathBuf::from(path));
        self
    }

    fn with_link(mut self, link: &str, target: &str) -> Self {
        self.links
            .insert(PathBuf::from(link), PathBuf::from(target));
        self
    }

    const fn uncanonicalisable(mut self) -> Self {
        self.uncanonicalisable = true;
        self
    }
}

impl FakePaths {
    fn present(&self, path: &Path) -> bool {
        path == Path::new("/")
            || self
                .entries
                .iter()
                .chain(self.links.keys())
                .any(|entry| entry.starts_with(path))
    }

    /// Resolves `..` as the kernel does: only beneath a directory that exists.
    fn resolved(&self, path: &Path) -> Option<PathBuf> {
        let mut walked = PathBuf::new();
        for component in path.components() {
            if component == Component::ParentDir {
                walked.pop();
            } else {
                walked.push(component);
                if !self.present(&walked) {
                    return None;
                }
            }
        }
        Some(walked)
    }
}

impl ExecutablePaths for FakePaths {
    fn canonicalise(&self, existing: &Path) -> Option<PathBuf> {
        if self.uncanonicalisable {
            return None;
        }
        self.resolved(existing)
    }

    fn link_target(&self, path: &Path) -> Option<PathBuf> {
        self.resolved(path)
            .and_then(|resolved| self.links.get(&resolved).cloned())
    }

    fn exists(&self, path: &Path) -> bool {
        self.resolved(path).is_some()
    }
}

fn three_roots() -> RepositoryRoots {
    RepositoryRoots::complete(vec![
        PathBuf::from("/project"),
        PathBuf::from("/work/ws"),
        PathBuf::from("/work/main"),
    ])
}

fn browser_key() -> ExecutablePathKey {
    ExecutablePathKey::declared(BROWSER).unwrap()
}

fn vetted(
    value: &str,
    roots: &RepositoryRoots,
    paths: &FakePaths,
) -> Result<PathBuf, Refusal> {
    vet_executable_path(&browser_key(), value, roots, paths)
}

fn code_of(refused: Result<PathBuf, Refusal>) -> String {
    codes(&[refused.expect_err("the value is refused")]).remove(0)
}

const INSIDE: &str = "E_EXECUTABLE_PATH_INSIDE_REPOSITORY";

#[test]
fn a_relative_value_is_refused_whatever_it_would_resolve_to() {
    let paths = FakePaths::default().with_entry("/opt/chromium");
    for value in ["./tools/chromium", "chromium", "../opt/chromium"] {
        let refusal = vetted(value, &three_roots(), &paths).unwrap_err();

        assert_eq!(
            codes(std::slice::from_ref(&refusal)),
            ["E_EXECUTABLE_PATH_RELATIVE"]
        );
        assert_eq!(refusal.reason(), RefusalReason::Value);
        assert_eq!(refusal.key().map(|key| key.name), Some(BROWSER));
    }
}

#[test]
fn an_absolute_path_inside_any_root_is_refused_however_it_gets_there() {
    for root in ["/project", "/work/ws", "/work/main"] {
        let inside = format!("{root}/bin/chromium");
        let paths = FakePaths::default()
            .with_entry(&inside)
            .with_link("/opt/pointing-in", &inside);
        for value in [
            inside.clone(),
            "/opt/pointing-in".to_owned(),
            format!("{root}/bin/missing"),
        ] {
            assert_eq!(
                code_of(vetted(&value, &three_roots(), &paths)),
                INSIDE,
                "{value}"
            );
        }
    }
}

#[test]
fn an_outside_symlink_resolves_to_its_canonical_target() {
    let paths = FakePaths::default()
        .with_entry("/opt/chrome/chrome")
        .with_link("/usr/local/bin/chrome", "/opt/chrome/chrome");

    assert_eq!(
        vetted("/usr/local/bin/chrome", &three_roots(), &paths),
        Ok(PathBuf::from("/opt/chrome/chrome"))
    );
}

#[test]
fn a_relative_link_target_is_joined_onto_the_links_directory() {
    let paths = FakePaths::default()
        .with_entry("/opt/Caskroom/chromium/chromium")
        .with_link("/opt/bin/chromium", "../Caskroom/chromium/chromium");

    assert_eq!(
        vetted("/opt/bin/chromium", &three_roots(), &paths),
        Ok(PathBuf::from("/opt/Caskroom/chromium/chromium"))
    );
}

#[test]
fn with_only_a_config_root_a_path_inside_it_is_refused() {
    let roots = RepositoryRoots::complete(vec![PathBuf::from("/project")]);
    let paths = FakePaths::default().with_entry("/project/chromium");

    assert_eq!(code_of(vetted("/project/chromium", &roots, &paths)), INSIDE);
}

#[test]
fn a_value_that_cannot_be_shown_to_be_outside_is_refused() {
    let base = FakePaths::default().with_entry("/project/bin");
    let cases: [(&str, FakePaths, RepositoryRoots); 6] = [
        ("/project/a/b/chromium", FakePaths::default(), three_roots()),
        (
            "/opt/dangling",
            FakePaths::default()
                .with_entry("/project/bin")
                .with_link("/opt/dangling", "/project/bin/gone"),
            three_roots(),
        ),
        (
            "/opt/missing/../chromium",
            FakePaths::default().with_entry("/opt/other"),
            three_roots(),
        ),
        (
            "/opt/chromium",
            FakePaths::default()
                .with_entry("/opt/chromium")
                .uncanonicalisable(),
            three_roots(),
        ),
        (
            "/opt/chromium",
            FakePaths::default().with_entry("/opt/chromium"),
            RepositoryRoots::incomplete(vec![PathBuf::from("/project")]),
        ),
        (
            "/opt/dir/chromium",
            base.with_link("/opt/dir", "/project/bin/gone-dir"),
            three_roots(),
        ),
    ];
    for (value, paths, roots) in cases {
        let refusal = vetted(value, &roots, &paths).unwrap_err();

        assert_eq!(codes(std::slice::from_ref(&refusal)), [INSIDE], "{value}");
        assert!(
            refusal.to_string().contains(
                "is inside, or cannot be shown to be outside, the repository"
            ),
            "{refusal}"
        );
    }
}

#[test]
fn a_symlink_cycle_is_refused_within_the_hop_limit() {
    for paths in [
        FakePaths::default().with_link("/opt/self", "/opt/self"),
        FakePaths::default()
            .with_link("/opt/self", "/opt/other")
            .with_link("/opt/other", "/opt/self"),
    ] {
        assert_eq!(
            code_of(vetted("/opt/self", &three_roots(), &paths)),
            INSIDE
        );
    }
    assert_eq!(SYMLINK_HOP_LIMIT, 40);
}

#[test]
fn a_nonexistent_path_outside_every_root_is_admitted_beneath_its_ancestor() {
    let paths = FakePaths::default().with_entry("/opt");

    assert_eq!(
        vetted("/opt/chrome/chrome", &three_roots(), &paths),
        Ok(PathBuf::from("/opt/chrome/chrome"))
    );
}

#[test]
fn path_refusals_render_their_codes_and_debug_redacts_the_path() {
    let key = browser_key().descriptor();
    let relative = Refusal::PathRelative { key };
    let inside = Refusal::PathInsideRepository {
        key,
        path: PathBuf::from("/secret/place/chromium"),
    };

    assert!(relative
        .to_string()
        .starts_with("E_EXECUTABLE_PATH_RELATIVE: design.browser_path"));
    assert!(inside.to_string().contains("/secret/place/chromium"));
    for refusal in [relative, inside] {
        let debugged = format!("{refusal:?}");
        assert!(!debugged.contains("/secret/place"), "{debugged}");
        assert!(debugged.contains(BROWSER), "{debugged}");
        assert!(
            refusal
                .to_string()
                .contains("absolute path outside the repository"),
            "{refusal}"
        );
    }
}

impl Project {
    fn resolve_path(
        &self,
        key: ExecutablePathKey,
        paths: &FakePaths,
    ) -> Consented<PathBuf> {
        resolve_executable_path(&self.context(), &key, &three_roots(), paths)
            .unwrap_or_else(|_| panic!("the path resolution aborted"))
    }
}

#[test]
fn a_refused_environment_path_falls_through_to_a_valid_personal_one() {
    let paths = FakePaths::default().with_entry("/opt/chrome");
    let consented = Project::new()
        .env("ACCELERATOR_DESIGN_BROWSER_PATH", "./chromium")
        .personal(BROWSER, "/opt/chrome")
        .team(BROWSER, "/opt/team-chrome")
        .resolve_path(browser_key(), &paths);

    assert_eq!(consented.admitted, Some(PathBuf::from("/opt/chrome")));
    assert_eq!(
        codes(&consented.refusals),
        ["E_EXECUTABLE_PATH_RELATIVE", "E_CONSENT_KEY_TEAM_LEVEL"]
    );
    assert!(consented.notice.is_none());
}

#[test]
fn an_environment_path_wins_with_a_notice_and_never_reads_the_personal_level() {
    let paths = FakePaths::default().with_entry("/opt/chrome");
    let project = Project::new()
        .env("ACCELERATOR_DESIGN_BROWSER_PATH", "/opt/chrome")
        .ignored_personal_file();
    let consented = project.resolve_path(browser_key(), &paths);

    assert_eq!(consented.admitted, Some(PathBuf::from("/opt/chrome")));
    assert!(consented.refusals.is_empty());
    assert_eq!(
        consented.notice.unwrap().to_string(),
        "notice: design.browser_path taken from \
         ACCELERATOR_DESIGN_BROWSER_PATH: /opt/chrome"
    );
    assert_eq!(project.personal_reads(), 0);
}

#[test]
fn a_personal_path_refused_on_its_value_leaves_nothing_admitted() {
    let paths = FakePaths::default().with_entry("/project/chromium");
    let consented = Project::new()
        .personal(BROWSER, "/project/chromium")
        .resolve_path(browser_key(), &paths);

    assert_eq!(consented.admitted, None);
    assert_eq!(codes(&consented.refusals), [INSIDE]);
}

/// The one policy, for path keys: every path key is refused on provenance
/// and on its value by the same codes, and yields its canonical path.
#[test]
fn every_path_key_is_refused_and_vetted_by_one_policy() {
    let paths = FakePaths::default()
        .with_entry("/opt/chrome/chrome")
        .with_entry("/project/chromium")
        .with_link("/opt/bin/chrome", "/opt/chrome/chrome");
    for key in [ExecutablePathKey::for_test("example.path"), browser_key()] {
        let name = key.descriptor().name;
        for (project, code) in [
            (
                Project::new().team(name, "/opt/bin/chrome"),
                "E_CONSENT_KEY_TEAM_LEVEL",
            ),
            (
                Project::new()
                    .personal(name, "/opt/bin/chrome")
                    .tracking(Tracking::Tracked),
                "E_CONSENT_KEY_TRACKED",
            ),
            (
                Project::new()
                    .personal(name, "/opt/bin/chrome")
                    .tracking(Tracking::Unknown),
                "E_CONSENT_KEY_TRACKING_UNKNOWN",
            ),
            (
                Project::new().personal(name, "bin/chrome"),
                "E_EXECUTABLE_PATH_RELATIVE",
            ),
            (Project::new().personal(name, "/project/chromium"), INSIDE),
        ] {
            let consented = project.resolve_path(key, &paths);
            assert_eq!(consented.admitted, None, "{name}");
            assert_eq!(codes(&consented.refusals), [code], "{name}");
            assert_eq!(consented.refusals[0].key(), Some(key.descriptor()));
        }

        let consented = Project::new()
            .personal(name, "/opt/bin/chrome")
            .resolve_path(key, &paths);
        assert_eq!(
            consented.admitted,
            Some(PathBuf::from("/opt/chrome/chrome")),
            "{name}"
        );
        assert_eq!(
            vet_executable_path(
                &key,
                "/opt/bin/chrome",
                &three_roots(),
                &paths
            ),
            Ok(PathBuf::from("/opt/chrome/chrome"))
        );
    }
}
