"""The working tree's debug CLI builds, as the evals run and judge them."""

import os
from dataclasses import dataclass
from pathlib import Path

from invoke.exceptions import Exit

from tasks.shared.dev_builds import LAUNCHER, SUBBINARIES

DEV_LAUNCHER_MARKER = ".accelerator-dev-launcher"
DEV_LAUNCHER_REMINDER = (
    f"When you have finished running evals, rm {DEV_LAUNCHER_MARKER} "
    "(mise run fails while it exists)."
)


def _subbinary(name: str) -> str:
    return next(binary.name for binary in SUBBINARIES if binary.name == name)


@dataclass(frozen=True)
class Toolchain:
    launcher: Path
    corpus: Path
    research: Path

    @classmethod
    def debug(cls, root: Path) -> Toolchain:
        debug = root / "cli/target/debug"
        return cls(
            launcher=debug / LAUNCHER.name,
            corpus=debug / _subbinary("accelerator-corpus"),
            research=debug / _subbinary("accelerator-research"),
        )

    def session_env(self) -> dict[str, str]:
        """Return this environment, resolving ``accelerator`` to these builds.

        An ``ACCELERATOR_*_BIN`` override already set is replaced, unlike
        ``accelerator_env``: an eval exists to judge the working tree's builds.
        """
        return os.environ | {
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(self.launcher),
            "ACCELERATOR_CORPUS_BIN": str(self.corpus),
            "ACCELERATOR_RESEARCH_BIN": str(self.research),
        }


def require_dev_launcher_marker(root: Path) -> None:
    """Refuse unless the operator has marked ``root`` as a dev launcher.

    The marker is the operator's consent to unverified binaries, so this
    never creates or removes it.
    """
    if (root / DEV_LAUNCHER_MARKER).exists():
        return
    raise Exit(
        "The eval runs this checkout's unverified CLI builds, which the "
        f"launcher allows only beside a {DEV_LAUNCHER_MARKER} marker. Run "
        f"touch {DEV_LAUNCHER_MARKER} to allow them, then "
        f"rm {DEV_LAUNCHER_MARKER} when you have finished running evals; "
        "CONTRIBUTING.md describes the dev launcher.",
        code=1,
    )
