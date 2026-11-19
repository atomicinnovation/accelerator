from enum import StrEnum


class Family(StrEnum):
    MATRIX = "matrix"

    @property
    def handover_variable(self) -> str:
        return {Family.MATRIX: "ACCELERATOR_SHARED_MATRIX"}[self]

    @property
    def heading(self) -> str:
        return {Family.MATRIX: "matrix"}[self]

    @property
    def noun(self) -> str:
        return {Family.MATRIX: "the matrix"}[self]


SHARED_FAMILIES: tuple[Family, ...] = tuple(Family)
WORKBENCHES_VARIABLE = "ACCELERATOR_SHARED_WORKBENCHES"
FIXTURE_LOG_VARIABLE = "ACCELERATOR_FIXTURE_LOG"
HOLD_VARIABLE = "ACCELERATOR_SHARED_FIXTURES_HOLD"
