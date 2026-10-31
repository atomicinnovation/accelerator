//! The shape of a Jira key or Linear identifier.

use tracker::is_tracker_key;

#[test]
fn a_tracker_key_is_a_letter_led_key_a_hyphen_and_digits() {
    assert!(is_tracker_key("PP-760"));
    assert!(is_tracker_key("eng-42"));
    assert!(is_tracker_key("MY_PROJ-7"));
    assert!(!is_tracker_key("0230"));
    assert!(!is_tracker_key("PP-"));
    assert!(!is_tracker_key("-760"));
    assert!(!is_tracker_key("1PP-760"));
    assert!(!is_tracker_key("PP-76a"));
    assert!(!is_tracker_key("draft-k7mq3x"));
    assert!(is_tracker_key("PRO_J-1"));
    assert!(is_tracker_key("p1-7"));
    assert!(!is_tracker_key("PROJ"));
    assert!(!is_tracker_key(""));
}
