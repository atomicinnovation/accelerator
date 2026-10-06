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
