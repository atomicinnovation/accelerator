"""Isolation of the binaries these suites build from cargo's uplift path.

Cargo unlinks and re-hardlinks `cli/target/debug/<bin>` at the end of every
build — including a no-op one — so the shared path is intermittently absent
whenever any other cargo invocation touches the workspace. Several suites and
build tasks do exactly that, concurrently, under a full `mise run`.
"""

import os
import shutil
import subprocess
from pathlib import Path

from tests.support import artefacts
from tests.support.artefacts import CLI_TARGET_DIR, build_shim, claim_artefact


def test_a_built_shim_is_claimed_out_of_cargos_uplift_path() -> None:
    shim = build_shim()

    assert CLI_TARGET_DIR not in shim.parents
    assert os.access(shim, os.X_OK)
    assert subprocess.run(
        [str(shim)], check=False, capture_output=True, text=True
    ).stderr.startswith("accelerator-verify: usage:")


def test_a_claimed_shim_is_stable_across_calls() -> None:
    assert build_shim() == build_shim()


def test_claiming_retries_while_the_uplift_path_is_absent(
    mocker, tmp_path: Path
) -> None:
    built = tmp_path / "artefact"
    built.write_bytes(b"payload")
    built.chmod(0o755)
    copy = shutil.copy2
    attempts: list[Path] = []

    def vanishing(source: Path, destination: Path) -> object:
        attempts.append(source)
        if len(attempts) < 3:
            raise FileNotFoundError(source)
        return copy(source, destination)

    mocker.patch.object(artefacts.shutil, "copy2", side_effect=vanishing)

    claimed = claim_artefact(built)

    assert claimed.read_bytes() == b"payload"
    assert len(attempts) == 3
