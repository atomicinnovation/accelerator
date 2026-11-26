import dataclasses
from collections.abc import Mapping
from enum import StrEnum


class Family(StrEnum):
    MATRIX = "matrix"
    STATUS_LOG = "status-log"
    STALLED_MIGRATION = "stalled-migration"

    @property
    def handover_variable(self) -> str:
        return _TRAITS[self].handover_variable

    @property
    def flag(self) -> str:
        return _TRAITS[self].flag

    @property
    def heading(self) -> str:
        return _TRAITS[self].heading

    @property
    def noun(self) -> str:
        return _TRAITS[self].noun

    @property
    def executables(self) -> Mapping[str, str]:
        """Each bin target the build runs, by the variable that names it."""
        return _TRAITS[self].executables


@dataclasses.dataclass(frozen=True)
class _Traits:
    handover_variable: str
    flag: str
    heading: str
    noun: str
    executables: Mapping[str, str] = dataclasses.field(default_factory=dict)


_TRAITS = {
    Family.MATRIX: _Traits(
        handover_variable="ACCELERATOR_SHARED_MATRIX",
        flag="--matrix",
        heading="matrix",
        noun="the matrix",
    ),
    Family.STATUS_LOG: _Traits(
        handover_variable="ACCELERATOR_SHARED_STATUS_LOG",
        flag="--status-log",
        heading="status-log set",
        noun="the status-log set",
    ),
    Family.STALLED_MIGRATION: _Traits(
        handover_variable="ACCELERATOR_SHARED_STALLED_MIGRATION",
        flag="--stalled-migration",
        heading="stalled-migration set",
        noun="the stalled-migration set",
        executables={
            "ACCELERATOR_SHARED_FIXTURES_MIGRATE": "accelerator-migrate"
        },
    ),
}


class PrivateSets(StrEnum):
    PRIVATE_SETS = "private-sets"


PRIVATE_SETS = PrivateSets.PRIVATE_SETS

type BuiltFixtures = Family | PrivateSets


def fixtures_named(name: str) -> BuiltFixtures:
    return PRIVATE_SETS if name == PRIVATE_SETS else Family(name)


SHARED_FAMILIES: tuple[Family, ...] = tuple(Family)
WORKBENCHES_VARIABLE = "ACCELERATOR_SHARED_WORKBENCHES"
FIXTURE_LOG_VARIABLE = "ACCELERATOR_FIXTURE_LOG"
HOLD_VARIABLE = "ACCELERATOR_SHARED_FIXTURES_HOLD"
HANDOVER_VARIABLES = frozenset(
    {
        *(family.handover_variable for family in Family),
        WORKBENCHES_VARIABLE,
        FIXTURE_LOG_VARIABLE,
    }
)
