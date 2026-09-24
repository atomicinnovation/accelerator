//! The run base a migration run is guarded against, built from the commits
//! the working copy is based on and read back from what an earlier run
//! recorded.

use migrate::run_base::RunBase;

fn ids(listed: &[&str]) -> Vec<String> {
    listed.iter().map(|id| (*id).to_owned()).collect()
}

#[test]
fn a_working_copy_based_on_nothing_has_no_run_base() {
    assert_eq!(RunBase::from_base_commits(&[]), None);
}

#[test]
fn a_single_base_commit_is_the_run_base() {
    let run_base = RunBase::from_base_commits(&ids(&["abc"]));

    assert_eq!(
        run_base.map(|base| base.to_string()),
        Some("abc".to_owned())
    );
}

#[test]
fn merge_parents_join_in_sorted_order_whatever_their_order() {
    let forwards = RunBase::from_base_commits(&ids(&["a", "b"]));
    let backwards = RunBase::from_base_commits(&ids(&["b", "a"]));

    assert_eq!(forwards, backwards);
    assert_eq!(
        forwards.map(|base| base.to_string()),
        Some("a+b".to_owned())
    );
}

#[test]
fn an_empty_record_has_no_run_base() {
    assert_eq!(RunBase::recorded(""), None);
    assert_eq!(RunBase::recorded("  \n"), None);
}

#[test]
fn every_earlier_record_reads_back_as_it_was_written() {
    for recorded in [
        "4b825dc642cb6eb9a060e54bf8d69288fbee4904",
        "5a1f0c2d9e8b7a6c5d4e3f2a1b0c9d8e7f6a5b4c",
        "stale-revision-sentinel",
    ] {
        let run_base = RunBase::recorded(&format!("{recorded}\n"));

        assert_eq!(
            run_base.map(|base| base.to_string()),
            Some(recorded.to_owned())
        );
    }
}

#[test]
fn a_recorded_run_base_matches_the_one_it_was_built_from() {
    let built = RunBase::from_base_commits(&ids(&["b", "a"]));

    assert_eq!(RunBase::recorded("a+b\n"), built);
}
