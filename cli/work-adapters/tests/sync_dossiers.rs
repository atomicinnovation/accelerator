//! Conflict dossiers list only the sections whose meaning differs, and name
//! both titles when they differ.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use serde_json::json;
use serde_json::Value;
use work::sync::Action;
use work::sync::SyncState;
use work_adapters::sync::run::render_dossier;
use work_adapters::sync::run::ConflictDossier;
use work_adapters::sync::run::DossierRender;
use work_adapters::sync::run::RunReport;

use crate::support::repo::issue;
use crate::support::repo::outcome_of;
use crate::support::repo::work_item;
use crate::support::repo::Recorded;
use crate::support::repo::Repo;
use crate::support::repo::Tracker;
use crate::support::repo::MOVED_STAMP;

const ORIGINAL: &str = "# Item\n\nOriginal intro\n\n## List\n\n- a\n- b\n";
const LOCAL: &str = "# Item\n\nLocal intro\n\n## List\n\n+ a\n+ b\n";
const LINEAR_REMOTE: &str = "# Item\n\nRemote intro\n\n## List\n\n* a\n* b\n";

fn adf(content: &[Value]) -> String {
    json!({"type": "doc", "version": 1, "content": content}).to_string()
}

fn text(text: &str) -> Value {
    json!({"type": "text", "text": text})
}

fn heading(level: u8, title: &str) -> Value {
    json!({"type": "heading", "attrs": {"level": level}, "content": [text(title)]})
}

fn paragraph(content: &str) -> Value {
    json!({"type": "paragraph", "content": [text(content)]})
}

fn bullets(items: &[&str]) -> Value {
    let items: Vec<Value> = items
        .iter()
        .map(|item| json!({"type": "listItem", "content": [paragraph(item)]}))
        .collect();
    json!({"type": "bulletList", "content": items})
}

fn table(rows: &[&[&str]]) -> Value {
    let rows: Vec<Value> = rows
        .iter()
        .enumerate()
        .map(|(index, cells)| {
            let kind = if index == 0 {
                "tableHeader"
            } else {
                "tableCell"
            };
            let cells: Vec<Value> = cells
                .iter()
                .map(|cell| json!({"type": kind, "content": [paragraph(cell)]}))
                .collect();
            json!({"type": "tableRow", "content": cells})
        })
        .collect();
    json!({"type": "table", "content": rows})
}

fn jira_description(intro: &str) -> String {
    adf(&[
        heading(1, "Item"),
        paragraph(intro),
        heading(2, "List"),
        bullets(&["a", "b"]),
    ])
}

fn projected(title: &str, description: &str) -> String {
    format!("{title}\n{description}\n")
}

/// Item 9, recorded at `original`, now edited on both sides.
fn conflicted(
    tracker: Tracker,
    local: &str,
    original: (&str, &str),
    remote: &str,
) -> Repo {
    let (original_local, original_projection) = original;
    Repo::with_item(
        tracker,
        9,
        local,
        issue(9, remote, MOVED_STAMP),
        &Recorded::split(original_local, original_projection),
    )
}

fn linear_conflict(local_title: &str, remote_title: &str) -> Repo {
    conflicted(
        Tracker::Linear,
        &work_item(9, local_title, "", LOCAL),
        (
            &work_item(9, "Item", "", ORIGINAL),
            &projected("Item", ORIGINAL),
        ),
        &projected(remote_title, LINEAR_REMOTE),
    )
}

fn only_dossier(report: &RunReport) -> &ConflictDossier {
    assert_eq!(outcome_of(report, 9), (SyncState::Conflict, Action::Prompt));
    assert_eq!(report.dossiers.len(), 1);
    &report.dossiers[0]
}

fn section_names(dossier: &ConflictDossier) -> Vec<&str> {
    dossier
        .sections
        .iter()
        .map(|section| section.name.as_str())
        .collect()
}

fn rendered(dossier: &ConflictDossier) -> String {
    let DossierRender::Renderable(text) =
        render_dossier(dossier, &|_| String::new())
    else {
        panic!("the dossier renders");
    };
    text
}

fn assert_only_the_edited_section_is_listed(dossier: &ConflictDossier) {
    assert_eq!(section_names(dossier), vec!["(preamble)"]);
    let section = &dossier.sections[0];
    assert!(section.local.starts_with("# Item\n"), "{}", section.local);
    assert!(section.remote.starts_with("# Item\n"), "{}", section.remote);
    assert!(
        section.remote.contains("Remote intro"),
        "{}",
        section.remote
    );
}

#[test]
fn a_real_linear_conflict_lists_only_the_edited_section() {
    let repo = linear_conflict("Item", "Item");

    let report = repo.sync_once();

    assert_only_the_edited_section_is_listed(only_dossier(&report));
}

#[test]
fn a_real_jira_conflict_lists_only_the_edited_section() {
    let original = projected("Item", &jira_description("Original intro"));
    let repo = conflicted(
        Tracker::Jira,
        &work_item(9, "Item", "", LOCAL),
        (&work_item(9, "Item", "", ORIGINAL), &original),
        &projected("Item", &jira_description("Remote intro")),
    );

    let report = repo.sync_once();

    assert_only_the_edited_section_is_listed(only_dossier(&report));
}

#[test]
fn a_jira_section_equivalent_only_after_the_push_round_trip_is_omitted() {
    let description = |notes: &str| {
        adf(&[
            heading(2, "Table"),
            table(&[&["a"], &["1"]]),
            heading(2, "Notes"),
            paragraph(notes),
        ])
    };
    let body = |notes: &str, alignment: &str| {
        format!(
            "## Table\n\n| a |\n|{alignment}|\n| 1 |\n\n## Notes\n\n{notes}\n"
        )
    };
    let original = projected("Item", &description("Original"));
    let repo = conflicted(
        Tracker::Jira,
        &work_item(9, "Item", "", &body("Local", ":--")),
        (
            &work_item(9, "Item", "", &body("Original", "---")),
            &original,
        ),
        &projected("Item", &description("Remote")),
    );

    let report = repo.sync_once();

    assert_eq!(section_names(only_dossier(&report)), vec!["Notes"]);
}

#[test]
fn a_title_difference_renders_local_arrow_remote() {
    let repo = linear_conflict("Local title", "Remote title");

    let header = rendered(only_dossier(&repo.sync_once()));

    assert!(
        header.contains("\ntitle: Local title → Remote title\n"),
        "{header}"
    );
}

#[test]
fn matching_titles_render_the_local_title_alone() {
    let repo = linear_conflict("Item", "Item");

    let header = rendered(only_dossier(&repo.sync_once()));

    assert!(header.contains("\ntitle: Item\n"), "{header}");
    assert!(!header.contains('→'), "{header}");
}
