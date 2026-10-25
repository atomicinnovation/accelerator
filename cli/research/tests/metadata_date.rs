#![allow(clippy::expect_used)]

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use research::sources::metadata::date::PartialDate;

fn parsed(raw: &str) -> Option<String> {
    PartialDate::parse(raw).map(|date| date.to_string())
}

#[test]
fn a_declared_date_is_kept_at_its_precision_and_its_time_dropped() {
    for (raw, expected) in [
        ("2023", "2023"),
        ("2023-05", "2023-05"),
        ("2023-05-12", "2023-05-12"),
        ("2023-05-20T17:59:59Z", "2023-05-20"),
        ("2019-03-04T23:30:00-05:00", "2019-03-04"),
        ("2021-07-08T12:00:00+0000", "2021-07-08"),
        ("2021-07-08T12:00+05", "2021-07-08"),
        ("2023-05-12T10:00:00.123Z", "2023-05-12"),
        ("2023-05-12T10:00", "2023-05-12"),
        ("2010/5/12", "2010-05-12"),
        ("2010/05/12", "2010-05-12"),
        ("2010/5", "2010-05"),
        ("2024-02-29", "2024-02-29"),
        ("2000-02-29", "2000-02-29"),
    ] {
        assert_eq!(parsed(raw).as_deref(), Some(expected), "{raw}");
    }
}

#[test]
fn a_date_outside_the_accepted_forms_is_unusable() {
    for raw in [
        "12/05/2010",
        "May 12, 2010",
        "2023-13-01",
        "2023-02-30",
        "2023-02-29",
        "1900-02-29",
        "2023-00",
        "2023-05-00",
        "2023-5",
        "2023-05-12 10:00",
        "2023-05-12T25:00Z",
        "2023-05-12T10:60Z",
        "2023-05-12T10:00+24:00",
        "2023-05-12T10:00+05:3",
        "2023-05T10:00Z",
        "2023-05-12T10Z",
        "2023-05-12T10:00:60Z",
        "2023-05-12T10:00:00.Z",
        "2010/5/12/1",
        "2010/",
        "",
    ] {
        assert_eq!(parsed(raw), None, "{raw}");
    }
}

#[test]
fn an_instant_converts_to_its_utc_calendar_date() {
    let leap_day = 1_709_164_800;
    for (seconds, expected) in [
        (0, "1970-01-01"),
        (1_800_000_000, "2027-01-15"),
        (leap_day, "2024-02-29"),
        (leap_day + 86_399, "2024-02-29"),
        (leap_day + 86_400, "2024-03-01"),
    ] {
        let at = UNIX_EPOCH + Duration::from_secs(seconds);
        assert_eq!(
            PartialDate::utc_date_of(at).to_string(),
            expected,
            "{seconds}"
        );
    }
}

#[test]
fn an_instant_before_the_epoch_clamps_to_the_epoch() {
    let before = UNIX_EPOCH - Duration::from_secs(86_400);
    assert_eq!(PartialDate::utc_date_of(before).to_string(), "1970-01-01");
    assert_eq!(
        PartialDate::utc_date_of(SystemTime::UNIX_EPOCH),
        PartialDate::parse("1970-01-01").expect("valid")
    );
}
