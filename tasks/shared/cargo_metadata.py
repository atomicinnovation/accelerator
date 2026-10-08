"""The ``cli/`` workspace members as ``cargo metadata`` declares them."""

import json
import subprocess
from typing import Any

from tasks.shared.paths import CLI_DIR

Package = dict[str, Any]


def workspace_packages() -> list[Package]:
    """Read the workspace members, without resolving third-party crates."""
    completed = subprocess.run(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--locked",
            "--manifest-path",
            str(CLI_DIR / "Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)["packages"]
