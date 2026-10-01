#![allow(clippy::expect_used, clippy::panic)]

mod support;

use std::rc::Rc;

use research::fetch::ConfirmationCache as _;
use research::request::ArxivId;
use research_adapters::confirmations::FileConfirmationCache;
use support::RecordedDiagnostics;
use support::Scratch;

fn id(raw: &str) -> ArxivId {
    ArxivId::parse(raw).unwrap_or_else(|error| panic!("{error}"))
}

fn cache(scratch: &Scratch) -> FileConfirmationCache {
    FileConfirmationCache::new(
        scratch.dir(),
        Rc::new(RecordedDiagnostics::default()),
    )
}

#[test]
fn an_unconfirmed_entry_is_not_recalled() {
    let scratch = Scratch::new();

    assert_eq!(cache(&scratch).recall(&id("2608.21129v2")), None);
}

#[test]
fn withdrawn_and_live_verdicts_are_both_recalled_by_a_later_call() {
    let scratch = Scratch::new();
    cache(&scratch).record(&id("2608.21129v2"), true);
    cache(&scratch).record(&id("2211.12792v2"), false);

    let later = cache(&scratch);

    assert_eq!(later.recall(&id("2608.21129v2")), Some(true));
    assert_eq!(later.recall(&id("2211.12792v2")), Some(false));
}

#[test]
fn a_newer_version_than_the_one_confirmed_is_confirmed_afresh() {
    let scratch = Scratch::new();
    cache(&scratch).record(&id("2211.12792v2"), false);

    assert_eq!(cache(&scratch).recall(&id("2211.12792v3")), None);
}

#[test]
fn an_unparseable_cache_counts_as_empty_and_is_replaced() {
    let scratch = Scratch::new();
    scratch.write("arxiv-withdrawals.json", "[not a cache");
    let cache = cache(&scratch);

    assert_eq!(cache.recall(&id("2608.21129v2")), None);
    cache.record(&id("2608.21129v2"), true);
    assert_eq!(cache.recall(&id("2608.21129v2")), Some(true));
}

#[test]
fn interleaved_writers_each_keep_their_own_entries() {
    let scratch = Scratch::new();
    let first = cache(&scratch);
    let second = cache(&scratch);

    first.record(&id("2608.21129v2"), true);
    second.record(&id("2211.12792v2"), false);
    first.record(&id("hep-th/9901001v1"), false);

    let later = cache(&scratch);
    assert_eq!(later.recall(&id("2608.21129v2")), Some(true));
    assert_eq!(later.recall(&id("2211.12792v2")), Some(false));
    assert_eq!(later.recall(&id("hep-th/9901001v1")), Some(false));
}

#[test]
fn a_failed_write_is_reported_without_failing_the_call() {
    let scratch = Scratch::new();
    scratch.mkdir("arxiv-withdrawals.json");
    let diagnostics = Rc::new(RecordedDiagnostics::default());
    let cache = FileConfirmationCache::new(scratch.dir(), diagnostics.clone());

    cache.record(&id("2608.21129v2"), true);

    assert_eq!(cache.recall(&id("2608.21129v2")), None);
    let reported = diagnostics.lines();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(
        reported[0].contains("arxiv-withdrawals.json"),
        "{reported:?}"
    );
}
