use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn exits_2_when_config_missing() {
    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.args(["--config", "/nonexistent/config.json"]);
    cmd.assert().code(2);
}

#[test]
fn parses_fixture_config_and_exits_success() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/config.valid.json");
    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.args(["--config", fixture.to_str().unwrap()]);
    cmd.assert().success();
}
