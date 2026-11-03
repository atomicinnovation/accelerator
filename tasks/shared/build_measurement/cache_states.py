from enum import StrEnum, unique
from pathlib import Path

from tasks.shared.build_measurement.locations import (
    COLD_DELETIONS,
    INCREMENTAL_TOUCH,
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
