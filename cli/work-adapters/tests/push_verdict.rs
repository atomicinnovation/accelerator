//! What a push's read-back means for the entry's rewrite mark and for the
//! notice the push reports, by where the pushed body came from.

use std::path::Path;

use corpus_adapters::CommonMarkBody;
use tracker::ExternalId;
use tracker::PlainDialect;
use work_adapters::sync::body_comparison::BodyComparison;
use work_adapters::sync::push_fidelity::ChangedLine;
use work_adapters::sync::push_fidelity::RewriteKind;
use work_adapters::sync::push_fidelity::RewrittenLines;
use work_adapters::sync::push_verdict::PushOrigin;
use work_adapters::sync::push_verdict::PushVerdict;
use work_adapters::sync::rewrite_mark::MarkState;
use work_adapters::sync::rewrite_mark::RewriteMark;

const SENT: &str = "---\ntitle: \"t\"\n---\n\n## Notes\n\nSee crates.io.\n";
const PRESERVED: &str = "t\n## Notes\n\nSee crates.io.\n";
const REWRITTEN: &str = "t\n## Notes\n\nSee [crates.io](http://crates.io).\n";

fn plain() -> BodyComparison<'static> {
    BodyComparison::new(&PlainDialect, &CommonMarkBody)
}

fn verdict(origin: PushOrigin, read_back: Option<&str>) -> PushVerdict {
    PushVerdict::judged(origin, &plain(), SENT, read_back)
}

fn earlier_mark() -> MarkState {
    MarkState::Marked(Box::new(RewriteMark::from(RewrittenLines::bounded(
        vec![ChangedLine {
            section: "Earlier".to_owned(),
            sent: "a".to_owned(),
            stored: "b".to_owned(),
        }],
    ))))
}

fn sections_marked(
    origin: PushOrigin,
    read_back: Option<&str>,
) -> Option<Vec<String>> {
    let verdict = verdict(origin, read_back);
    earlier_mark()
        .after(verdict.mark_change())
        .mark()
        .map(|mark| mark.rewritten_lines().sections)
}

fn notice_kind(
    origin: PushOrigin,
    read_back: Option<&str>,
) -> Option<RewriteKind> {
    verdict(origin, read_back)
        .into_notice(
            Path::new("meta/work/0001.md"),
            &ExternalId::new("PP-1".to_owned()),
        )
        .map(|notice| notice.kind)
}

#[test]
fn a_rewritten_push_marks_the_entry_and_notices_the_rewrite() {
    for origin in [PushOrigin::Pushed, PushOrigin::Created] {
        assert_eq!(
            sections_marked(origin, Some(REWRITTEN)),
            Some(vec!["Notes".to_owned()]),
            "{origin:?}"
        );
        assert_eq!(
            notice_kind(origin, Some(REWRITTEN)),
            Some(RewriteKind::OnPush),
            "{origin:?}"
        );
    }
}

#[test]
fn a_preserved_push_clears_the_mark_and_notices_nothing() {
    for origin in [PushOrigin::Pushed, PushOrigin::Created] {
        assert_eq!(
            sections_marked(origin, Some(PRESERVED)),
            None,
            "{origin:?}"
        );
        assert_eq!(notice_kind(origin, Some(PRESERVED)), None, "{origin:?}");
    }
}

#[test]
fn an_unread_push_keeps_the_mark() {
    assert_eq!(
        sections_marked(PushOrigin::Pushed, None),
        Some(vec!["Earlier".to_owned()])
    );
    assert_eq!(notice_kind(PushOrigin::Pushed, None), None);
}

#[test]
fn a_create_never_inherits_a_mark() {
    assert_eq!(sections_marked(PushOrigin::Created, None), None);
}

#[test]
fn an_issue_created_in_an_earlier_run_is_not_judged() {
    assert_eq!(
        sections_marked(PushOrigin::CreatedEarlier, Some(REWRITTEN)),
        None
    );
    assert_eq!(
        notice_kind(PushOrigin::CreatedEarlier, Some(REWRITTEN)),
        None
    );
}
