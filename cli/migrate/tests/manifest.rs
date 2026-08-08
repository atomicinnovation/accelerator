//! Ownership classification, table-driven over every documented case.

use migrate::manifest::classify;
use migrate::manifest::is_session_artefact;
use migrate::manifest::is_session_log;
use migrate::manifest::Ownership;
use migrate::manifest::RunnerPaths;

const fn runner() -> RunnerPaths<'static> {
    RunnerPaths {
        applied: ".accelerator/state/migrations-applied",
        skipped: ".accelerator/state/migrations-skipped",
        run_paths: ".accelerator/state/migrations-run-paths.txt",
        run_id: ".accelerator/state/migrations-run.id",
    }
}

#[test]
fn each_runner_managed_bookkeeping_path_is_owned_regardless_of_manifest() {
    let runner = runner();
    for path in [
        runner.applied,
        runner.skipped,
        runner.run_paths,
        runner.run_id,
    ] {
        assert_eq!(
            classify(path, &runner, &[], false),
            Ownership::RunnerManaged
        );
    }
}

#[test]
fn a_session_artefact_is_owned_by_pattern_only_when_the_base_revision_matches()
{
    let runner = runner();
    let path = ".accelerator/state/migrations-0099-session.jsonl";

    assert_eq!(
        classify(path, &runner, &[], true),
        Ownership::SessionArtefact
    );
    assert_eq!(classify(path, &runner, &[], false), Ownership::Foreign);
}

#[test]
fn the_three_session_artefact_suffixes_are_all_recognised() {
    for suffix in ["-session.jsonl", "-stderr.log", "-resume-state.tmp"] {
        let path = format!(".accelerator/state/migrations-0099{suffix}");
        assert!(is_session_artefact(&path), "{path} should be an artefact");
    }
}

#[test]
fn only_the_session_log_suffix_is_a_session_log() {
    assert!(is_session_log(
        ".accelerator/state/migrations-0099-session.jsonl"
    ));
    assert!(!is_session_log(
        ".accelerator/state/migrations-0099-stderr.log"
    ));
    assert!(!is_session_log(
        ".accelerator/state/migrations-0099-resume-state.tmp"
    ));
}

#[test]
fn a_manifested_path_is_owned() {
    let runner = runner();
    let manifest = vec!["meta/work/0001-foo.md".to_owned()];

    assert_eq!(
        classify("meta/work/0001-foo.md", &runner, &manifest, false),
        Ownership::Manifested
    );
}

#[test]
fn an_empty_manifest_with_a_matching_revision_still_refuses_an_unmanifested_path(
) {
    let runner = runner();

    assert_eq!(
        classify("meta/work/0002-bar.md", &runner, &[], true),
        Ownership::Foreign
    );
}

#[test]
fn a_path_outside_every_class_is_foreign() {
    let runner = runner();
    assert_eq!(
        classify("meta/unrelated.md", &runner, &[], true),
        Ownership::Foreign
    );
}
