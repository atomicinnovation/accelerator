"""The `test-loopback` release guard against real stripped release binaries.

The unit tests in `tests/unit/tasks/test_build.py` prove the guard spots the
marker in planted bytes; these prove a real release build carrying the feature
still contains those bytes after `strip` and thin LTO.
"""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

from tasks.build import _assert_no_test_loopback
from tasks.shared.dev_builds import LAUNCHER, SUBBINARIES, DevBinary
from tasks.shared.paths import CLI_DIR, CLI_WORKSPACE_CARGO_TOML
from tests.support.tools import require

_LOOPBACK_SEAMED: tuple[DevBinary, ...] = tuple(
    binary for binary in (LAUNCHER, *SUBBINARIES) if binary.loopback_seam
)
_LOOPBACK_SEAMED_NAMES = [binary.name for binary in _LOOPBACK_SEAMED]

# Kept apart from cli/target/release so a binary built with a test seam never
# lands where another task measures or runs release binaries.
_RELEASE_GUARD_TARGET_DIR = CLI_DIR / "target" / "release-guard"


def _build_release(*, with_test_loopback: bool, destination: Path) -> Path:
    destination.mkdir()
    binaries = [
        argument
        for binary in _LOOPBACK_SEAMED
        for argument in ("--bin", binary.name)
    ]
    features = (
        [
            "--features",
            ",".join(f for binary in _LOOPBACK_SEAMED for f in binary.features),
        ]
        if with_test_loopback
        else []
    )
    subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "--manifest-path",
            str(CLI_WORKSPACE_CARGO_TOML),
            *binaries,
            *features,
        ],
        check=True,
        env={
            **os.environ,
            "CARGO_TARGET_DIR": str(_RELEASE_GUARD_TARGET_DIR),
            # A release build that turns debug assertions back on slips past
            # the feature's compile_error!, leaving the byte scan as the only
            # backstop. Both variants share this profile so the second build
            # reuses the first's dependency graph.
            "CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS": "true",
        },
    )
    for binary in _LOOPBACK_SEAMED:
        shutil.copy2(
            _RELEASE_GUARD_TARGET_DIR / "release" / binary.name, destination
        )
    return destination


@pytest.fixture(scope="module")
def release_builds(tmp_path_factory) -> dict[str, Path]:
    require("cargo")
    root = tmp_path_factory.mktemp("release-guard")
    return {
        "default": _build_release(
            with_test_loopback=False, destination=root / "default"
        ),
        "test-loopback": _build_release(
            with_test_loopback=True, destination=root / "test-loopback"
        ),
    }


@pytest.mark.parametrize("binary", _LOOPBACK_SEAMED_NAMES)
def test_a_release_binary_built_with_test_loopback_is_rejected(
    release_builds, binary
):
    with pytest.raises(RuntimeError, match="test-loopback"):
        _assert_no_test_loopback(release_builds["test-loopback"] / binary)


@pytest.mark.parametrize("binary", _LOOPBACK_SEAMED_NAMES)
def test_a_default_release_binary_passes_the_loopback_guard(
    release_builds, binary
):
    _assert_no_test_loopback(release_builds["default"] / binary)
