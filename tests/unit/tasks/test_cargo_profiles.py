from tasks.shared.paths import CLI_WORKSPACE_CARGO_TOML, load_toml


def test_dev_builds_verify_signatures_at_release_speed():
    manifest = load_toml(CLI_WORKSPACE_CARGO_TOML)
    dev_packages = manifest["profile"]["dev"]["package"]

    assert "minisign-verify" in manifest["workspace"]["dependencies"], (
        "the dev-profile override names a package the workspace no longer "
        "depends on, and cargo only warns about that"
    )
    assert dev_packages["minisign-verify"]["opt-level"] == 3
