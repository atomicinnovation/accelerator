from enum import StrEnum, unique
from pathlib import Path

from tasks.shared.paths import CLI_DIR, CLI_TARGET_DIR, FRONTEND, REPO_ROOT

INCREMENTAL_TOUCH = (CLI_DIR / "kernel" / "src" / "lib.rs").relative_to(
    REPO_ROOT
)
COLD_DELETIONS = (
    CLI_TARGET_DIR.relative_to(REPO_ROOT),
    (FRONTEND / "node_modules").relative_to(REPO_ROOT),
)


@unique
class BuildCacheState(StrEnum):
    WARM_NO_OP = "warm-no-op"
    INCREMENTAL = "incremental"
    COLD = "cold"

    @property
    def primes(self) -> bool:
        return self is not BuildCacheState.COLD

    @property
    def touches(self) -> tuple[Path, ...]:
        if self is BuildCacheState.INCREMENTAL:
            return (INCREMENTAL_TOUCH,)
        return ()

    @property
    def deletions(self) -> tuple[Path, ...]:
        if self is BuildCacheState.COLD:
            return COLD_DELETIONS
        return ()
