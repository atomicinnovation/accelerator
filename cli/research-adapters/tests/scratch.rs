#![allow(clippy::expect_used)]

mod support;

use support::Scratch;

#[test]
fn a_transient_replace_is_whole_and_refuses_names_outside_the_directory() {
    let scratch = Scratch::new();
    scratch.write("state", "old");
    let dir = scratch.dir();

    dir.replace_transient("state", b"new").expect("replaced");
    let escaped = dir.replace_transient("../escaped", b"x");

    assert_eq!(scratch.read("state").as_deref(), Some("new"));
    assert!(escaped.is_err());
    assert!(!dir.path("../escaped").exists());
}

#[test]
fn removing_a_name_that_escapes_the_directory_is_refused() {
    let scratch = Scratch::new();
    scratch.write("kept", "x");
    let queue = scratch.dir().nested("arxiv-queue");
    std::fs::create_dir_all(queue.path("")).expect("mkdir queue");

    let removed = queue.remove("../kept");

    assert!(removed.is_err());
    assert_eq!(scratch.read("kept").as_deref(), Some("x"));
}

#[test]
fn removing_a_missing_name_succeeds() {
    let scratch = Scratch::new();

    assert_eq!(scratch.dir().remove("absent"), Ok(()));
}

#[test]
fn names_of_a_directory_not_yet_created_is_empty() {
    let scratch = Scratch::new();

    assert_eq!(scratch.dir().nested("arxiv-queue").names(), Ok(Vec::new()));
}

#[test]
fn a_nested_directory_refuses_writes_outside_the_project_root() {
    let scratch = Scratch::new();
    scratch.write("existing", "x");
    let outside = scratch.dir().nested("../../..");
    let escaped = format!("escaped-{}", std::process::id());

    assert!(outside.replace_transient(&escaped, b"x").is_err());
    assert!(!outside.path(&escaped).exists());
}

#[test]
fn a_file_s_modification_time_is_read_and_a_missing_one_is_none() {
    let scratch = Scratch::new();
    scratch.write("dated", "x");
    let dir = scratch.dir();

    assert!(dir.modified("dated").is_some());
    assert_eq!(dir.modified("absent"), None);
}
