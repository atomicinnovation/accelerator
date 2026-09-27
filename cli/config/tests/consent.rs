//! The consent policy over in-memory ports: who may supply a consent key, how
//! refusals are ordered, and how severity is decided.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use config::catalogue::Trust;
use config::consent::{
    audit, resolve, AuditFinding, CommandKey, ConfigFileTracking, ConsentKey,
    Consented, Distrust, Environment, ExecutablePathKey, ProvenanceContext,
    Refusal, RefusalReason, Tracking, TrackingCheck, Usable,
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
