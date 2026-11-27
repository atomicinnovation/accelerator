//! A set type only this test declares behaves as the built-in set types do,
//! end to end: nothing in the server names a set type to make it work.

use std::path::{Path, PathBuf};

use corpus::doc_type::{
    DocType, Facet, LibraryPhase, LibraryPlacement, Listing,
};
use corpus::set::{
    MemberKind, MemberSlot, Presentation, SetDeclaration, SetShape,
};
use corpus::DocTypeKey;
use serde_json::Value;

mod common;
mod live_server;

use live_server::{
    base_cfg, read_until, start_server, write_files, StartedServer,
};

static NOTEBOOKS: DocType = DocType {
    wire_token: "notebooks",
    config_path_key: Some("notebooks"),
    label: "Notebooks",
    participates_in_lifecycle: true,
    library_placement: Some(LibraryPlacement {
        phase: LibraryPhase::Remember,
        position: 9,
    }),
    facets: &[Facet::Status],
    listing: Listing::Set,
    ..DocType::FLAT_DEFAULTS
};

static NOTEBOOKS_SET: SetDeclaration = SetDeclaration {
    shape: SetShape {
        root: "index.md",
        member_slots: &[
            MemberSlot {
                pattern: "docs/*.md",
                kind: MemberKind::Document,
            },
            MemberSlot {
                pattern: "img/*.png",
                kind: MemberKind::Asset,
            },
        ],
    },
    presentation: Presentation::SetPage,
};

const NOTEBOOK: &str = "meta/notebooks/lab";

fn notebook_cfg(tmp: &Path) -> accelerator_visualiser::config::Config {
    let mut cfg = base_cfg(tmp);
    cfg.doc_paths
        .insert("notebooks".into(), tmp.join("meta/notebooks"));
    cfg.doc_types = Some(common::catalogue_with(
        &[&NOTEBOOKS],
        &[(DocTypeKey::declared(&NOTEBOOKS), &NOTEBOOKS_SET)],
    ));
    cfg
}

fn in_notebook(file: &str) -> String {
    format!("{NOTEBOOK}/{file}")
}

fn set_mtime_ms(path: &Path, ms: i64) {
    common::set_mtime_ms(path, ms).unwrap();
}

struct Notebook {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    server: StartedServer,
    events: reqwest::Response,
}

async fn notebook(files: &[(&str, &str)]) -> Notebook {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let files: Vec<(String, &str)> = files
        .iter()
        .map(|(file, content)| (in_notebook(file), *content))
        .collect();
    let files: Vec<(&str, &str)> =
        files.iter().map(|(f, c)| (f.as_str(), *c)).collect();
    write_files(&root, &files);
    for (file, _) in &files {
        set_mtime_ms(&root.join(file), 1_000);
    }
    let mut server = start_server(notebook_cfg(&root), &root).await;
    let events = server.events().await;
    Notebook {
        _tmp: tmp,
        root,
        server,
        events,
    }
}

const ROOT: &str =
    "---\nid: \"lab\"\ntitle: Lab notebook\nstatus: draft\n---\nThe lab.\n";

async fn lab() -> Notebook {
    notebook(&[
        ("index.md", ROOT),
        (
            "docs/first.md",
            "---\ntitle: First\nstatus: final\n---\n\
             A phrase only quokkas would write.\n",
        ),
        (
            "docs/second.md",
            "---\ntitle: Second\nstatus: final\n---\nThe second.\n",
        ),
        ("img/a.png", "png"),
        (
            "other/z.md",
            "---\ntitle: Z\nstatus: final\n---\nUnslotted wombat prose.\n",
        ),
        (
            ".hidden.md",
            "---\ntitle: Hidden\nstatus: final\n---\nHidden wombat prose.\n",
        ),
    ])
    .await
}

fn member_paths(set: &Value) -> Vec<&str> {
    set["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["setRelativePath"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn a_declared_set_type_serves_its_root_and_members() {
    let lab = lab().await;

    let (status, set) = lab.server.get("/api/sets/notebooks/lab").await;

    assert_eq!(status, 200, "{set}");
    assert_eq!(set["root"]["setRelativePath"], "index.md");
    assert_eq!(
        member_paths(&set),
        ["docs/first.md", "docs/second.md", "img/a.png"]
    );
}

#[tokio::test]
async fn a_declared_set_types_member_is_found_by_its_first_paragraph() {
    let lab = lab().await;

    let (_, found) = lab.server.get("/api/search?q=only%20quokkas").await;

    let rows = found["results"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{found}");
    assert_eq!(rows[0]["setId"], "lab");
    assert_eq!(rows[0]["setRelativePath"], "docs/first.md");
    assert_eq!(rows[0]["title"], "First");
}

#[tokio::test]
async fn a_declared_set_type_lists_one_entry_and_a_member_change_one_event() {
    let mut lab = lab().await;
    let (_, listing) = lab.server.get("/api/docs?type=notebooks").await;
    let docs = listing["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 1, "{listing}");
    assert_eq!(docs[0]["setId"], "lab");

    let second = lab.root.join(in_notebook("docs/second.md"));
    std::fs::write(&second, "---\ntitle: Second, again\n---\n").unwrap();
    lab.server.changed(&second);

    let frames = read_until(&mut lab.events, "doc-changed").await;
    let changes = live_server::doc_changes_within(
        &mut lab.events,
        std::time::Duration::from_millis(300),
    )
    .await;
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(frames.matches("doc-changed").count(), 1, "{frames}");
    assert!(frames.contains(&in_notebook("docs/second.md")), "{frames}");
}

#[tokio::test]
async fn a_declared_set_takes_one_activity_row_and_one_library_item() {
    let mut lab = lab().await;
    let second = lab.root.join(in_notebook("docs/second.md"));
    std::fs::write(&second, "---\ntitle: Second, again\n---\n").unwrap();
    set_mtime_ms(&second, 9_000);
    lab.server.changed(&second);
    read_until(&mut lab.events, "doc-changed").await;

    let (_, activity) = lab.server.get("/api/activity").await;
    let rows = activity["events"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{activity}");
    assert_eq!(rows[0]["path"], in_notebook("index.md"));

    let (_, library) = lab.server.get("/api/library/structure").await;
    let notebooks = library["phases"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|phase| phase["docTypes"].as_array().unwrap())
        .find(|doc_type| doc_type["id"] == "notebooks")
        .unwrap_or_else(|| panic!("no notebooks in {library}"));
    assert_eq!(notebooks["count"], 1);
    assert_eq!(notebooks["latest"]["modifiedAt"], 9_000);
    let status = notebooks["filterFacets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|facet| facet["id"] == "status")
        .unwrap();
    assert_eq!(status["options"].as_array().unwrap().len(), 1, "{status}");
    assert_eq!(status["options"][0]["id"], "draft");
    assert_eq!(status["options"][0]["count"], 1);
}

#[tokio::test]
async fn no_cluster_holds_a_declared_set_types_member() {
    let lab = lab().await;

    let (_, lifecycle) = lab.server.get("/api/lifecycle").await;

    let clustered: Vec<&Value> = lifecycle["clusters"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|cluster| cluster["entries"].as_array().unwrap())
        .collect();
    assert!(
        clustered
            .iter()
            .any(|entry| entry["setRelativePath"] == "index.md"),
        "{lifecycle}"
    );
    assert!(
        clustered
            .iter()
            .all(|entry| entry["setRelativePath"] != "docs/first.md"
                && entry["setRelativePath"] != "docs/second.md"),
        "{lifecycle}"
    );
    let paths = lifecycle.to_string();
    assert!(
        !paths.contains("other/z.md") && !paths.contains(".hidden.md"),
        "{lifecycle}"
    );
}

#[tokio::test]
async fn writes_to_files_a_declared_shape_does_not_place_emit_nothing() {
    let mut lab = lab().await;
    for file in ["other/z.md", ".hidden.md"] {
        let path = lab.root.join(in_notebook(file));
        std::fs::write(&path, "---\ntitle: Rewritten\n---\n").unwrap();
        lab.server.changed(&path);
    }
    let second = lab.root.join(in_notebook("docs/second.md"));
    std::fs::write(&second, "---\ntitle: Second, again\n---\n").unwrap();
    lab.server.changed(&second);

    let frames = read_until(&mut lab.events, "doc-changed").await;

    assert_eq!(frames.matches("doc-changed").count(), 1, "{frames}");
    assert!(frames.contains(&in_notebook("docs/second.md")), "{frames}");
    let (_, activity) = lab.server.get("/api/activity").await;
    let rows = activity["events"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{activity}");
    assert_eq!(rows[0]["path"], in_notebook("index.md"));
    let (_, listing) = lab.server.get("/api/docs?type=notebooks").await;
    assert_eq!(listing["docs"].as_array().unwrap().len(), 1, "{listing}");
}

#[tokio::test]
async fn files_a_declared_shape_does_not_place_are_nowhere() {
    let lab = lab().await;

    for file in ["other/z.md", ".hidden.md"] {
        let (status, _) = lab
            .server
            .get(&format!("/api/docs/{}", in_notebook(file)))
            .await;
        assert_eq!(status, 404, "{file}");
    }
    let (_, found) = lab.server.get("/api/search?q=wombat").await;
    assert_eq!(found["results"].as_array().unwrap().len(), 0, "{found}");
    let (_, set) = lab.server.get("/api/sets/notebooks/lab").await;
    assert!(!member_paths(&set)
        .iter()
        .any(|p| p.contains('z') || p.contains("hidden")));
}

#[tokio::test]
async fn a_declared_set_type_is_described_as_a_set() {
    let lab = lab().await;

    let (_, types) = lab.server.get("/api/types").await;

    let notebooks = types["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|doc_type| doc_type["key"] == "notebooks")
        .unwrap_or_else(|| panic!("no notebooks in {types}"));
    assert_eq!(notebooks["set"]["root"], "index.md");
    assert_eq!(notebooks["set"]["presentation"], "set-page");
}

#[tokio::test]
async fn sixteen_members_come_back_in_one_response_without_bodies() {
    let mut files: Vec<(String, String)> = vec![
        ("index.md".into(), ROOT.into()),
        (
            "docs/unclosed.md".into(),
            "---\ntitle: \"open\n---\n".into(),
        ),
        ("docs/plain.md".into(), "No frontmatter here.\n".into()),
    ];
    files.extend((1..=8).map(|n| {
        (
            format!("docs/d{n}.md"),
            format!("---\ntitle: D{n}\n---\nBody.\n"),
        )
    }));
    files.extend((1..=6).map(|n| (format!("img/s{n}.png"), "x".repeat(n))));
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(f, c)| (f.as_str(), c.as_str()))
        .collect();
    let lab = notebook(&files).await;

    let (status, set) = lab.server.get("/api/sets/notebooks/lab").await;

    assert_eq!(status, 200, "{set}");
    assert_eq!(set["root"]["setRelativePath"], "index.md");
    assert_eq!(set["root"]["frontmatter"]["title"], "Lab notebook");
    assert!(set["root"].get("body").is_none(), "{set}");
    let members = set["members"].as_array().unwrap();
    assert_eq!(members.len(), 16);
    for member in members {
        assert!(member.get("body").is_none(), "{member}");
        assert_eq!(member["mtimeMs"], 1_000, "{member}");
        match member["kind"].as_str().unwrap() {
            "document" => assert!(member["frontmatterState"].is_string()),
            "asset" => assert!(member["size"].is_u64()),
            other => panic!("unexpected kind {other}"),
        }
    }
    let state_of = |path: &str| {
        members
            .iter()
            .find(|m| m["setRelativePath"] == path)
            .map(|m| m["frontmatterState"].clone())
    };
    assert_eq!(state_of("docs/d1.md"), Some("parsed".into()));
    let d1 = members
        .iter()
        .find(|m| m["setRelativePath"] == "docs/d1.md")
        .unwrap();
    assert_eq!(d1["frontmatter"]["title"], "D1");
    assert_eq!(state_of("docs/unclosed.md"), Some("malformed".into()));
    assert_eq!(state_of("docs/plain.md"), Some("absent".into()));
    let s6 = members
        .iter()
        .find(|m| m["setRelativePath"] == "img/s6.png")
        .unwrap();
    assert_eq!(s6["size"], 6);
}
