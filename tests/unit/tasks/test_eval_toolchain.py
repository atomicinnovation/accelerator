import shutil
from pathlib import Path

import pytest
from invoke.exceptions import Exit

from tasks.eval.toolchain import Toolchain, require_dev_launcher_marker

MARKER = ".accelerator-dev-launcher"


def test_a_missing_marker_refuses(tmp_path: Path) -> None:
    with pytest.raises(Exit) as refused:
        require_dev_launcher_marker(tmp_path)

    for named in (
        "touch .accelerator-dev-launcher",
        "rm .accelerator-dev-launcher",
        "CONTRIBUTING.md",
    ):
        assert named in str(refused.value.message)


def test_a_present_marker_passes_and_stays(tmp_path: Path) -> None:
    (tmp_path / MARKER).touch()

    require_dev_launcher_marker(tmp_path)

    assert (tmp_path / MARKER).exists()


def test_the_marker_is_never_created(tmp_path: Path) -> None:
    with pytest.raises(Exit):
        require_dev_launcher_marker(tmp_path)

    assert list(tmp_path.iterdir()) == []


def test_the_toolchain_is_the_workspace_s_debug_builds(tmp_path: Path) -> None:
    toolchain = Toolchain.debug(tmp_path)

    debug = tmp_path / "cli/target/debug"
    assert toolchain == Toolchain(
        bootstrap=tmp_path / "bin/accelerator",
        launcher=debug / "accelerator",
        corpus=debug / "accelerator-corpus",
        research=debug / "accelerator-research",
    )


def test_a_session_env_points_the_launcher_at_the_debug_builds(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("ACCELERATOR_RESEARCH_BIN", "/elsewhere")
    monkeypatch.setenv("PATH", "/usr/bin")
    toolchain = Toolchain.debug(tmp_path)

    env = toolchain.session_env()

    assert env["ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER"] == "1"
    assert env["ACCELERATOR_LAUNCHER_BIN"] == str(toolchain.launcher)
    assert env["ACCELERATOR_CORPUS_BIN"] == str(toolchain.corpus)
    assert env["ACCELERATOR_RESEARCH_BIN"] == str(toolchain.research)
    assert env["PATH"] == f"{tmp_path / 'bin'}:/usr/bin"


def test_a_session_resolves_accelerator_to_this_checkout_s_bootstrap(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    installed = tmp_path / "installed"
    installed.mkdir()
    (installed / "accelerator").touch(mode=0o755)
    checkout = tmp_path / "checkout"
    (checkout / "bin").mkdir(parents=True)
    (checkout / "bin/accelerator").touch(mode=0o755)
    monkeypatch.setenv("PATH", str(installed))

    env = Toolchain.debug(checkout).session_env()

    assert shutil.which("accelerator", path=env["PATH"]) == str(
        checkout / "bin/accelerator"
    )
