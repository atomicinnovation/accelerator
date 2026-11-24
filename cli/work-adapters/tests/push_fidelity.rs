//! Whether a tracker stored a pushed body with the meaning it was sent with,
//! and which lines show the difference when it did not.

mod captures;

use captures::fixture;
use captures::fragments;
use captures::read_back;
use captures::TestError;
use corpus_adapters::CommonMarkBody;
use jira_client::JiraBodyDialect;
use tracker::PlainDialect;
use work::section_diff::SectionDiff;
use work_adapters::sync::body_comparison::BodyComparison;
use work_adapters::sync::push_fidelity::changed_lines;
use work_adapters::sync::push_fidelity::meaningful_hunks;
use work_adapters::sync::push_fidelity::ChangedLine;
use work_adapters::sync::push_fidelity::PushFidelity;
use work_adapters::sync::push_fidelity::RewriteNotice;
use work_adapters::sync::push_fidelity::RewrittenLines;

const CONTENT_FRONTMATTER: &str = "---\ntitle: \"t\"\nstatus: ready\n---\n\n";

fn plain() -> BodyComparison<'static> {
    BodyComparison::new(&PlainDialect, &CommonMarkBody)
}

fn jira() -> BodyComparison<'static> {
    BodyComparison::new(&JiraBodyDialect, &CommonMarkBody)
}

fn content(body: &str) -> String {
    format!("{CONTENT_FRONTMATTER}{body}")
}

fn projected(description: &str) -> String {
    format!("t\n{description}")
}

fn fidelity(
    comparison: &BodyComparison<'_>,
    sent: &str,
    stored: &str,
) -> PushFidelity {
    comparison.push_fidelity(&content(sent), Some(&projected(stored)))
}

fn jira_description(markdown: &str) -> Result<String, TestError> {
    Ok(serde_json::to_string(&adf::markdown_to_document(
        markdown, None,
    )?)?)
}

fn section(name: &str, sent: &str, stored: &str) -> SectionDiff {
    SectionDiff {
        name: name.to_owned(),
        local: sent.to_owned(),
        remote: stored.to_owned(),
    }
}

fn changed(section: &str, sent: &str, stored: &str) -> ChangedLine {
    ChangedLine {
        section: section.to_owned(),
        sent: sent.to_owned(),
        stored: stored.to_owned(),
    }
}

const fn rewritten(changes: Vec<ChangedLine>) -> PushFidelity {
    PushFidelity::Rewritten { changes }
}

struct Capture {
    pushed: captures::Fragments,
    read_back: captures::Fragments,
}

fn linear_capture() -> Result<Capture, TestError> {
    Ok(Capture {
        pushed: fragments(&fixture("linear-rewrites/pushed.md")?, "## "),
        read_back: fragments(
            &read_back("linear-rewrites/read-back.json")?,
            "## ",
        ),
    })
}

impl Capture {
    fn pair(&self, code: &str) -> Result<(String, String), TestError> {
        let pushed = self.pushed.get(code).ok_or("no pushed fragment")?;
        let stored = self.read_back.get(code).ok_or("no stored fragment")?;
        Ok((format!("{pushed}\n"), format!("{stored}\n")))
    }
}

#[test]
fn a_cosmetic_read_back_is_preserved() {
    assert_eq!(
        fidelity(&plain(), "* one\n* two\n", "- one\n- two\n"),
        PushFidelity::Preserved
    );
    assert_eq!(
        fidelity(&plain(), "See crates\\.io.\n", "See crates.io.\n"),
        PushFidelity::Preserved
    );
}

#[test]
fn a_linked_dotted_token_is_a_rewrite() {
    assert_eq!(
        fidelity(
            &plain(),
            "See crates.io.\n",
            "See [crates.io](<http://crates.io>).\n"
        ),
        rewritten(vec![changed(
            "(preamble)",
            "See crates.io.",
            "See [crates.io](<http://crates.io>)."
        )])
    );
}

#[test]
fn a_stray_list_is_a_rewrite() -> Result<(), TestError> {
    let (sent, stored) = linear_capture()?.pair("X09")?;

    let PushFidelity::Rewritten { changes } =
        fidelity(&plain(), &sent, &stored)
    else {
        return Err("X09's read-back should be a rewrite".into());
    };
    assert_eq!(changes.len(), 1);
    Ok(())
}

#[test]
fn an_unread_remote_is_unjudged() {
    assert_eq!(
        plain().push_fidelity(&content("See crates.io.\n"), None),
        PushFidelity::Unjudged
    );
}

#[test]
fn an_unrenderable_remote_is_unjudged() {
    let panel = "{\"content\":[{\"attrs\":{\"panelType\":\"info\"},\
        \"content\":[],\"type\":\"panel\"}],\"type\":\"doc\",\"version\":1}\n";

    assert_eq!(fidelity(&jira(), "Body\n", panel), PushFidelity::Unjudged);
}

#[test]
fn a_rejected_local_body_is_unjudged() -> Result<(), TestError> {
    let description = jira_description("Other text\n")?;

    assert_eq!(
        fidelity(&jira(), "Body \u{1e} text\n", &description),
        PushFidelity::Unjudged
    );
    Ok(())
}

#[test]
fn only_lines_that_read_differently_are_reported() {
    let sent = "* one\n* two\n\nBetween the lists.\n\nSee crates.io.\n";
    let stored = "- one\n- two\n\nBetween the lists.\n\n\
                  See [crates.io](<http://crates.io>).\n";

    assert_eq!(
        changed_lines(&section("Notes", sent, stored), &plain()),
        vec![changed(
            "Notes",
            "See crates.io.",
            "See [crates.io](<http://crates.io>)."
        )]
    );
}

#[test]
fn an_adjacent_cosmetic_line_rides_with_its_hunk() {
    let sent = "* one\nSee crates.io.\n";
    let stored = "- one\nSee [crates.io](<http://crates.io>).\n";

    assert_eq!(
        changed_lines(&section("Notes", sent, stored), &plain()),
        vec![changed(
            "Notes",
            "* one\nSee crates.io.",
            "- one\nSee [crates.io](<http://crates.io>)."
        )]
    );
}

#[test]
fn a_kept_escape_is_not_reported() {
    let sent = "See crates.io first.\n\nThen foo.io.\n";
    let stored = "See crates\\.io first.\n\n\
                  Then [foo.io](<http://foo.io>).\n";

    assert_eq!(
        changed_lines(&section("Notes", sent, stored), &plain()),
        vec![changed(
            "Notes",
            "Then foo.io.",
            "Then [foo.io](<http://foo.io>)."
        )]
    );
}

#[test]
fn a_stray_list_hunk_is_judged_in_context() -> Result<(), TestError> {
    let (sent, stored) = linear_capture()?.pair("X09")?;

    assert_eq!(
        changed_lines(&section("X09", &sent, &stored), &plain()),
        vec![changed(
            "X09",
            "- The tier infrastructure and the migration of existing evals \
             — both owned by\n  0160.\n- Running evals on every CI build.",
            "* The tier infrastructure and the migration of existing evals \
             — both owned by\n  160. \n* Running evals on every CI build."
        )]
    );
    Ok(())
}

#[test]
fn a_blockquoted_stray_list_is_judged_in_context() {
    assert_eq!(
        changed_lines(
            &section("Quote", "> owned by\n> 0160.\n", "> owned by\n> 160.\n"),
            &plain()
        ),
        vec![changed("Quote", "> 0160.", "> 160.")]
    );
}

#[test]
fn a_rewrite_with_no_surviving_hunk_reports_its_hunks() {
    let cosmetic = section("Notes", "* one\n", "- one\n");

    assert_eq!(meaningful_hunks(&cosmetic, &plain()), Vec::new());
    assert_eq!(
        changed_lines(&cosmetic, &plain()),
        vec![changed("Notes", "* one", "- one")]
    );
}

#[test]
fn the_filter_alone_finds_every_captured_rewrite() -> Result<(), TestError> {
    let capture = linear_capture()?;
    let comparison = plain();
    let mut judged_rewritten = 0;
    for code in capture.pushed.keys() {
        let (sent, stored) = capture.pair(code)?;
        if !matches!(
            fidelity(&comparison, &sent, &stored),
            PushFidelity::Rewritten { .. }
        ) {
            continue;
        }
        judged_rewritten += 1;
        let surviving: usize = comparison
            .differing_sections_as_written(&sent, &stored)
            .iter()
            .map(|section| meaningful_hunks(section, &comparison).len())
            .sum();
        assert!(surviving > 0, "{code}: no hunk reads differently");
    }
    assert!(judged_rewritten > 0);
    Ok(())
}

#[test]
fn a_rewritten_level_two_heading_is_one_change() {
    let sent = "Intro.\n\n## Changes to config.rs\n\nText.\n";
    let stored = "Intro.\n\n## Changes to [config.rs](<http://config.rs>)\n\n\
                  Text.\n";

    assert_eq!(
        fidelity(&plain(), sent, stored),
        rewritten(vec![changed(
            "Changes to config.rs",
            "## Changes to config.rs",
            "## Changes to [config.rs](<http://config.rs>)"
        )])
    );
}

#[test]
fn a_pure_delete_or_insert_is_a_hunk() {
    assert_eq!(
        changed_lines(
            &section("Notes", "Kept.\nDropped.\n", "Kept.\n"),
            &plain()
        ),
        vec![changed("Notes", "Dropped.", "")]
    );
    assert_eq!(
        changed_lines(
            &section("Notes", "Kept.\n", "Kept.\nAdded.\n"),
            &plain()
        ),
        vec![changed("Notes", "", "Added.")]
    );
}

#[test]
fn a_stray_list_without_a_leading_zero_reads_alike() {
    assert_eq!(
        fidelity(
            &plain(),
            "- owned by\n  42.\n- next\n",
            "* owned by\n  42. \n* next\n"
        ),
        PushFidelity::Preserved
    );
}

#[test]
fn jira_cosmetic_lines_are_not_reported() -> Result<(), TestError> {
    let sent = "| a | b |\n|:-|-:|\n| 1 | 2 |\n\nSee crates.io.\n";
    let description = jira_description(
        "| a | b |\n|:-|-:|\n| 1 | 2 |\n\nSee [crates.io](http://crates.io).\n",
    )?;

    let PushFidelity::Rewritten { changes } =
        fidelity(&jira(), sent, &description)
    else {
        return Err("the linked token should be a rewrite".into());
    };
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].sent, "See crates.io.");
    Ok(())
}

#[test]
fn rewritten_lines_are_bounded() {
    let side = |length: usize| "a".repeat(length);
    let across_the_limit = format!("{}—tail", side(199));
    let bounded = RewrittenLines::bounded(vec![
        changed("A", &side(200), &side(201)),
        changed("A", &across_the_limit, "short"),
    ]);
    assert_eq!(bounded.lines[0].sent, side(200));
    assert_eq!(bounded.lines[0].stored, format!("{}…", side(200)));
    assert_eq!(bounded.lines[1].sent, format!("{}…", side(199)));
    assert_eq!(bounded.more, 0);

    let twelve = (0..12)
        .map(|index| changed(&format!("S{}", index % 2), "sent", "stored"))
        .collect();
    let bounded = RewrittenLines::bounded(twelve);
    assert_eq!(bounded.lines.len(), 5);
    assert_eq!(bounded.more, 7);
    assert_eq!(bounded.sections, vec!["S0".to_owned(), "S1".to_owned()]);
}

#[test]
fn a_rewritten_push_is_noticed_and_any_other_is_not() {
    let path = std::path::Path::new("meta/work/0001-a.md");
    let key = tracker::ExternalId::new("TES-1".to_owned());
    let change = changed("Notes", "See crates.io.", "See [crates.io](<x>).");

    let notice =
        RewriteNotice::on_push(rewritten(vec![change.clone()]), path, &key);
    assert_eq!(
        notice,
        Some(RewriteNotice {
            path: path.to_path_buf(),
            external_id: key.clone(),
            lines: RewrittenLines::bounded(vec![change]),
        })
    );
    assert_eq!(
        RewriteNotice::on_push(PushFidelity::Preserved, path, &key),
        None
    );
    assert_eq!(
        RewriteNotice::on_push(PushFidelity::Unjudged, path, &key),
        None
    );
}
