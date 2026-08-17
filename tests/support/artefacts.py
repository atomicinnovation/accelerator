"""Binaries built from `cli/` for a suite to run.

Cargo unlinks and re-hardlinks `cli/target/debug/<bin>` at the end of every
build, a no-op one included, so that path is intermittently absent whenever
anything else builds in the same workspace — which several suites and build
tasks do concurrently under a full `mise run`. A suite therefore never runs the
uplifted binary directly: it claims its own copy, which no later build can pull
out from under it.
"""

import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

import pytest

from tests.support.tools import require

REPO_ROOT = Path(__file__).resolve().parents[2]
CLI_TARGET_DIR = REPO_ROOT / "cli/target"
CLI_MANIFEST = REPO_ROOT / "cli/Cargo.toml"

# How long a claim keeps retrying while cargo's uplift path is absent. The gap
# between the unlink and the re-link is milliseconds; anything approaching this
# bound means the binary was never built at all.
CLAIM_TIMEOUT_SECONDS = 5.0

# Every claimed binary lives here for the life of the process, out of reach of
# any concurrent cargo invocation.
_ARTEFACTS = tempfile.TemporaryDirectory(prefix="accelerator-artefacts-")
_CLAIMED: dict[str, Path] = {}


def claim_artefact(built: Path) -> Path:
    """Copy a freshly built binary out of cargo's uplift path.

    Only the `open` has to land while the link is live, so a source that has
    vanished into the re-link window is retried rather than fatal.
    """
    claimed = Path(_ARTEFACTS.name) / built.name
    deadline = time.monotonic() + CLAIM_TIMEOUT_SECONDS
    while True:
        try:
            shutil.copy2(built, claimed)
        except FileNotFoundError:
            if time.monotonic() >= deadline:
                pytest.fail(f"not built: {built}")
            time.sleep(0.05)
            continue
        return claimed


def _cargo_build(package: str, binary: str) -> Path:
    if binary in _CLAIMED:
        return _CLAIMED[binary]
    require("cargo")
    subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            *package.split(),
            "--manifest-path",
            str(CLI_MANIFEST),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    claimed = claim_artefact(CLI_TARGET_DIR / "debug" / binary)
    if not os.access(claimed, os.X_OK):
        pytest.fail(f"not executable: {claimed}")
    _CLAIMED[binary] = claimed
    return claimed


def build_shim() -> Path:
    """Build and return the real `accelerator-verify` shim from `cli/`."""
    return _cargo_build("-p accelerator-verify", "accelerator-verify")


def build_launcher() -> Path:
    """Build and return the real launcher from `cli/`.

    Built here rather than behind a `mise` build edge so a suite using it still
    runs standalone under a bare `uv run pytest`. A suite that calls this must
    therefore *not* also gain a `build:cli:dev` dependency: the two would
    contend on cargo's target lock and the asserted edge would be inert.
    """
    return _cargo_build("--bin accelerator", "accelerator")
