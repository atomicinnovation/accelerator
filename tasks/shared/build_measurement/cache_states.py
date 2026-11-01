from enum import StrEnum, unique


@unique
class BuildCacheState(StrEnum):
    WARM_NO_OP = "warm-no-op"

    @property
    def primes(self) -> bool:
        return True
