import pytest
from invoke import Context, Exit

from tasks.build import cli_dev
from tasks.shared.dev_builds import (
    DEV_BUILDS,
    LAUNCHER,
    SUBBINARIES,
    DevBinary,
    cargo_build_command,
)
from tasks.shared.paths import (
    CLI_WORKSPACE_CARGO_TOML,
    DISPATCHED_SUBBINARIES,
    cli_member_manifests,
    load_toml,
)

_SERVER = "visualiser"


def _manifests_by_package() -> dict[str, dict]:
    manifests = (
        load_toml(path)
        for path in cli_member_manifests(CLI_WORKSPACE_CARGO_TOML)
    )
    return {manifest["package"]["name"]: manifest for manifest in manifests}


def _binary_names(manifest: dict) -> set[str]:
    declared = {target["name"] for target in manifest.get("bin", [])}
    return declared or {manifest["package"]["name"]}


@pytest.mark.parametrize(
    "token", [t for t in DISPATCHED_SUBBINARIES if t != _SERVER]
)
def test_every_dispatched_subbinary_but_the_server_has_a_dev_build(token):
    assert [b.name for b in DEV_BUILDS[token]] == [f"accelerator-{token}"]


def test_the_launcher_build_is_the_launcher_alone():
    assert DEV_BUILDS["launcher"] == (LAUNCHER,)
    assert LAUNCHER.name == "accelerator"


def test_the_characterisation_build_is_the_launcher_and_every_subbinary():
    assert DEV_BUILDS["characterisation"] == (LAUNCHER, *SUBBINARIES)


def test_the_server_is_not_a_dev_subbinary():
    assert "accelerator-visualiser" not in {b.name for b in SUBBINARIES}


@pytest.mark.parametrize(
    "binary", (LAUNCHER, *SUBBINARIES), ids=lambda b: b.name
)
def test_each_binary_is_declared_by_its_package(binary):
    manifest = _manifests_by_package()[binary.package]
    assert binary.name in _binary_names(manifest)


@pytest.mark.parametrize(
    "binary", (LAUNCHER, *SUBBINARIES), ids=lambda b: b.name
)
def test_a_binary_carries_the_loopback_seam_exactly_when_its_package_has_it(
    binary,
):
    features = _manifests_by_package()[binary.package].get("features", {})
    assert binary.loopback_seam == ("test-loopback" in features)


def test_a_seamed_binary_builds_with_its_packages_loopback_feature():
    seamed = DevBinary(
        "accelerator-jira", package="jira-cli", loopback_seam=True
    )
    assert seamed.features == ("jira-cli/test-loopback",)


def test_an_unseamed_binary_builds_with_no_features():
    assert (
        DevBinary("accelerator-vcs", package="accelerator-vcs").features == ()
    )


def test_one_cargo_build_covers_every_binary_and_feature():
    command = cargo_build_command(
        (
            DevBinary("accelerator", package="accelerator"),
            DevBinary(
                "accelerator-jira", package="jira-cli", loopback_seam=True
            ),
            DevBinary(
                "accelerator-research",
                package="accelerator-research",
                loopback_seam=True,
            ),
        )
    )
    assert command == (
        f"cargo build --manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "--bin accelerator --bin accelerator-jira --bin accelerator-research "
        "--features jira-cli/test-loopback,accelerator-research/test-loopback"
    )


def test_a_build_without_seams_passes_no_feature_flag():
    command = cargo_build_command((LAUNCHER,))
    assert command == (
        f"cargo build --manifest-path {CLI_WORKSPACE_CARGO_TOML} "
        "--bin accelerator"
    )


class TestCliDevTask:
    @pytest.fixture
    def ctx(self, mocker):
        return mocker.MagicMock(spec=Context)

    def test_runs_the_named_build(self, ctx):
        cli_dev(ctx, group="characterisation")
        ctx.run.assert_called_once_with(
            cargo_build_command(DEV_BUILDS["characterisation"])
        )

    def test_defaults_to_the_launcher(self, ctx):
        cli_dev(ctx)
        ctx.run.assert_called_once_with(cargo_build_command((LAUNCHER,)))

    def test_refuses_an_unknown_build_naming_the_known_ones(self, ctx):
        with pytest.raises(Exit, match="characterisation"):
            cli_dev(ctx, group="nonesuch")
        ctx.run.assert_not_called()
