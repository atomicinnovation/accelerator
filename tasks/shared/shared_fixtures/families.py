import dataclasses
from enum import StrEnum


class Family(StrEnum):
    MATRIX = "matrix"

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


@dataclasses.dataclass(frozen=True)
class _Traits:
    handover_variable: str
    flag: str
    heading: str
    noun: str


_TRAITS = {
    Family.MATRIX: _Traits(
        handover_variable="ACCELERATOR_SHARED_MATRIX",
        flag="--matrix",
        heading="matrix",
        noun="the matrix",
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
