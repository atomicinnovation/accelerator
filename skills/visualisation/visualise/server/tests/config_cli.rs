use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn exits_2_when_config_missing() {
    let mut cmd = Command::cargo_bin("accelerator-visualiser").unwrap();
    cmd.args(["--config", "/nonexistent/config.json"]);
    cmd.assert().code(2);
}
