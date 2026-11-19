use std::fs;
use std::path::Path;
use std::thread;

use serde_json::Value;
use vcs_test_support::family::Family;
use vcs_test_support::family::Fixtures;
use vcs_test_support::fixture_log::Event;
use vcs_test_support::fixture_log::FixtureLog;
use vcs_test_support::fixture_log::TestName;
use vcs_test_support::fixture_log::FIXTURE_LOG_VARIABLE;
use vcs_test_support::handover::SHARED_WORKBENCHES_VARIABLE;

type TestError = Box<dyn std::error::Error>;

const LOG_FORMAT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/contract/fixture-log.jsonl");

const VARIABLES: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/contract/variables.json");

const FLAGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/contract/flags.json");

const KINDS_THE_TASK_WRITES: [&str; 2] = ["tree-digest", "tree-changes"];

fn classify_test() -> TestName {
    TestName::from_variables(
        Some("vcs-adapters::classify"),
        Some("every_fixture_classifies_as_expected"),
    )
}

fn lines(path: &Path) -> Result<Vec<Value>, TestError> {
    fs::read_to_string(path)?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

#[test]
fn a_located_log_appends_one_line_per_record() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("run.jsonl");
    let log = FixtureLog::at(&path);
    let test = classify_test();
    let event = Event::Adoption {
        family: Family::Matrix,
        test: &test,
        hermetic_root: Path::new("/workbench"),
    };

    log.record(&event)?;
    log.record(&event)?;

    let recorded = lines(&path)?;
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0]["event"], "adoption");
    assert_eq!(recorded[0]["hermetic-root"], "/workbench");
    Ok(())
}

#[test]
fn a_disabled_log_has_no_location_and_records_nothing() -> Result<(), TestError>
{
    let log = FixtureLog::disabled();
    let test = classify_test();

    log.record(&Event::Adoption {
        family: Family::Matrix,
        test: &test,
        hermetic_root: Path::new("/workbench"),
    })?;

    assert_eq!(log.location(), None);
    Ok(())
}

#[test]
fn concurrent_writers_never_interleave_their_lines() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("run.jsonl");
    let log = FixtureLog::at(&path);
    let test = classify_test();

    let write_fifty = || {
        for _ in 0..50 {
            log.record(&Event::PrivateBuild {
                fixtures: Fixtures::Shared(Family::Matrix),
                test: &test,
                hermetic_root: Path::new("/a/long/workbench"),
            })
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    };
    let outcomes: Vec<Result<(), String>> = thread::scope(|scope| {
        let mut writers = Vec::new();
        for _ in 0..8 {
            writers.push(scope.spawn(write_fifty));
        }
        writers
            .into_iter()
            .map(|writer| {
                writer
                    .join()
                    .unwrap_or_else(|_| Err("a writer panicked".into()))
            })
            .collect()
    });

    assert_eq!(outcomes, vec![Ok(()); 8]);
    let recorded = lines(&path)?;
    assert_eq!(recorded.len(), 400);
    assert!(recorded.iter().all(|line| line["event"] == "private-build"));
    Ok(())
}

#[test]
fn a_test_is_named_by_its_binary_and_its_name() {
    let name = serde_json::to_value(classify_test()).ok();

    assert_eq!(
        name,
        Some(Value::from(
            "vcs-adapters::classify every_fixture_classifies_as_expected"
        ))
    );
}

#[test]
fn a_test_outside_nextest_is_named_by_its_thread() {
    let name = serde_json::to_value(TestName::from_variables(None, None)).ok();

    assert_eq!(
        name,
        thread::current().name().map(Value::from),
        "the libtest harness names each test's thread after the test"
    );
}

#[test]
fn the_private_sets_are_logged_under_their_own_name() {
    let names = [Fixtures::Shared(Family::Matrix), Fixtures::PrivateSets]
        .map(|fixtures| serde_json::to_value(fixtures).ok());

    assert_eq!(
        names,
        [
            Some(Value::from("matrix")),
            Some(Value::from("private-sets"))
        ]
    );
}

#[test]
fn every_event_matches_the_shared_log_format() -> Result<(), TestError> {
    let test = classify_test();
    let events = [
        Event::SharedBuild {
            family: Family::Matrix,
            tree: Path::new("/tmp/accelerator-shared-fixtures-501/run/matrix"),
            git: "git version 2.50.1",
            jj: "jj 0.43.0",
        },
        Event::Adoption {
            family: Family::Matrix,
            test: &test,
            hermetic_root: Path::new(
                "/tmp/accelerator-shared-fixtures-501/run/workbenches/vcs-workbench-a",
            ),
        },
        Event::PrivateBuild {
            fixtures: Fixtures::Shared(Family::Matrix),
            test: &test,
            hermetic_root: Path::new("/tmp/vcs-workbench-b"),
        },
    ];
    let serialised = events
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;

    let format = fs::read_to_string(LOG_FORMAT)?;
    let lines_the_crate_writes: Vec<&str> = format
        .lines()
        .filter(|line| {
            !KINDS_THE_TASK_WRITES
                .iter()
                .any(|kind| line.contains(&format!("\"event\":\"{kind}\"")))
        })
        .collect();
    assert_eq!(lines_the_crate_writes, serialised);
    Ok(())
}

#[test]
fn every_variable_matches_the_shared_names() -> Result<(), TestError> {
    let names: Value = serde_json::from_str(&fs::read_to_string(VARIABLES)?)?;

    for family in Family::ALL {
        let key = serde_json::to_value(family)?;
        let key = key.as_str().ok_or("a family serialises as a string")?;
        assert_eq!(names["handover"][key], family.handover_variable());
    }
    assert_eq!(
        names["handover"].as_object().map(serde_json::Map::len),
        Some(Family::ALL.len())
    );
    assert_eq!(names["workbenches"], SHARED_WORKBENCHES_VARIABLE);
    assert_eq!(names["log"], FIXTURE_LOG_VARIABLE);
    Ok(())
}

#[test]
fn every_builder_flag_matches_the_shared_flags() -> Result<(), TestError> {
    let flags: Value = serde_json::from_str(&fs::read_to_string(FLAGS)?)?;

    for family in Family::ALL {
        assert_eq!(flags[family.name()], family.flag());
    }
    assert_eq!(
        flags.as_object().map(serde_json::Map::len),
        Some(Family::ALL.len())
    );
    Ok(())
}
