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
fn a_date_carrying_non_ascii_where_digits_belong_is_unusable() {
    for raw in [
        "2023-05-12T10:00+1é1",
        "2023-05-12T10:00+é1",
        "2023-05-12T10:00+0é:00",
        "2023-05-12T1é:00Z",
        "2023-0é-12",
        "2é23",
        "2010/é/12",
        "May 1é, 2010",
    ] {
        assert_eq!(parsed(raw), None, "{raw}");
    }
}

#[test]
fn a_date_naming_its_month_is_kept_at_its_precision() {
    for (raw, expected) in [
        ("May 12, 2010", "2010-05-12"),
        ("Mar 21, 2007", "2007-03-21"),
        ("March 21, 2007", "2007-03-21"),
        ("Mar. 21, 2007", "2007-03-21"),
        ("Mar. 21 2007", "2007-03-21"),
        ("Sept 3, 2019", "2019-09-03"),
        ("Mar 21 2007", "2007-03-21"),
        ("21 March 2007", "2007-03-21"),
        ("21 Mar 2007", "2007-03-21"),
        ("03 Mar 2007", "2007-03-03"),
        ("MARCH 21, 2007", "2007-03-21"),
        ("march  21,  2007", "2007-03-21"),
        ("March 2007", "2007-03"),
        ("Dec 2007", "2007-12"),
        ("Feb 29, 2024", "2024-02-29"),
    ] {
        assert_eq!(parsed(raw).as_deref(), Some(expected), "{raw}");
    }
}

#[test]
fn a_month_named_date_outside_the_accepted_forms_is_unusable() {
    for raw in [
        "Feb 29, 2023",
        "Apr 31, 2010",
        "Mar 0, 2007",
        "Mar 021, 2007",
        "Mar 21, 07",
        "Mar 21,2007",
        "21, March 2007",
        "Mars 21, 2007",
        "Ma 21, 2007",
        "Marc 21, 2007",
        "March. 21, 2007",
        "Mar 21",
        "21 Mar",
        "March",
        "2007 March 21",
        "Wed, 21 Mar 2007",
        "21 Mar 2007 10:00:00 GMT",
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
