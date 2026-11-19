from enum import StrEnum


class Family(StrEnum):
    MATRIX = "matrix"

    @property
    def handover_variable(self) -> str:
        return {Family.MATRIX: "ACCELERATOR_SHARED_MATRIX"}[self]

    @property
    def flag(self) -> str:
        return {Family.MATRIX: "--matrix"}[self]

    @property
    def heading(self) -> str:
        return {Family.MATRIX: "matrix"}[self]

    @property
    def noun(self) -> str:
        return {Family.MATRIX: "the matrix"}[self]


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
