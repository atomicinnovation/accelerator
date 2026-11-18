//! The shape of each tracker's issue keys.

use tracker::TrackerKeys;

const NEVER_KEYS: [&str; 9] = [
    "0230",
    "PP-",
    "-760",
    "1PP-760",
    "PP-76a",
    "draft-k7mq3x",
    "PROJ",
    "",
    "PP-7-8",
];

#[test]
fn a_jira_key_is_a_letter_led_project_key_a_hyphen_and_digits() {
    for key in ["PP-760", "eng-42", "MY_PROJ-7", "PRO_J-1", "p1-7"] {
        assert!(TrackerKeys::Jira.is_key(key), "{key}");
    }
    for refused in NEVER_KEYS {
        assert!(!TrackerKeys::Jira.is_key(refused), "{refused}");
    }
}

#[test]
fn a_linear_identifier_has_a_letter_led_team_key_without_underscores() {
    for key in ["PP-760", "eng-42", "p1-7"] {
        assert!(TrackerKeys::Linear.is_key(key), "{key}");
    }
    for refused in NEVER_KEYS.iter().chain(&["MY_PROJ-7"]) {
        assert!(!TrackerKeys::Linear.is_key(refused), "{refused}");
    }
}

#[test]
fn any_supported_tracker_accepts_what_one_tracker_accepts() {
    assert!(TrackerKeys::AnySupported.is_key("MY_PROJ-7"));
    assert!(TrackerKeys::AnySupported.is_key("PP-760"));
    for refused in NEVER_KEYS {
        assert!(!TrackerKeys::AnySupported.is_key(refused), "{refused}");
    }
}

#[test]
fn an_integration_names_its_own_keys() {
    assert_eq!(TrackerKeys::of_integration("jira"), TrackerKeys::Jira);
    assert_eq!(TrackerKeys::of_integration("linear"), TrackerKeys::Linear);
    assert_eq!(TrackerKeys::of_integration(""), TrackerKeys::AnySupported);
    assert_eq!(
        TrackerKeys::of_integration("github"),
        TrackerKeys::AnySupported
    );
}
