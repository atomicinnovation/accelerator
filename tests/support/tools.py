"""External tools a suite needs on PATH.

Absence is a contributor's missing toolchain locally and a provisioning
regression in CI, so the same call has to skip in one place and fail in the
other.
"""

import os
import shutil

import pytest


def in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def require(name: str) -> None:
    """Skip locally, fail in CI: a missing tool there is a provisioning bug."""
    if shutil.which(name):
        return
    message = f"{name} not on PATH"
    if in_ci():
        pytest.fail(f"{message} — provisioning regression in CI")
    pytest.skip(message)
