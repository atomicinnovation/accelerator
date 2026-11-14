import shutil
import subprocess
from pathlib import Path

import pytest

SUITE = Path(__file__).resolve().parent / "stop-evidence.test.js"


@pytest.mark.skipif(shutil.which("node") is None, reason="needs node")
def test_the_stop_evidence_model_passes_its_node_suite():
    run = subprocess.run(
        ["node", "--test", str(SUITE)],
        capture_output=True,
        text=True,
        check=False,
    )

    assert run.returncode == 0, run.stdout + run.stderr
