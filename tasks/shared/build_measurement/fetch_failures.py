import re
from collections.abc import Iterable, Mapping
from enum import StrEnum, unique
from types import MappingProxyType

from tasks.shared.build_measurement.record_model import RecordModel
from tasks.shared.build_measurement.timeline import (
    StampedLine,
    attributed_lines,
)


@unique
class ExternalInput(StrEnum):
    LAUNCHER_RELEASE = "launcher-release"
    ADVISORY_DATABASE = "advisory-database"
    NPM_REGISTRY = "npm-registry"


class FetchFailure(RecordModel):
    run: int
    task: str
    external_input: ExternalInput
    line: str


FETCH_FAILURE_PATTERNS: Mapping[ExternalInput, re.Pattern[str]] = (
    MappingProxyType(
        {
            ExternalInput.LAUNCHER_RELEASE: re.compile(r"\bcurl: \(\d+\)"),
            ExternalInput.ADVISORY_DATABASE: re.compile(
                r"\[ERROR\] failed to get 'FETCH_HEAD' metadata"
            ),
            ExternalInput.NPM_REGISTRY: re.compile(
                r"\bnpm (?:ERR!|error) code "
                r"(?:ETIMEDOUT|ECONNRESET|ENOTFOUND|EAI_AGAIN)\b"
            ),
        }
    )
)


def _failed_input(text: str) -> ExternalInput | None:
    return next(
        (
            external_input
            for external_input, pattern in FETCH_FAILURE_PATTERNS.items()
            if pattern.search(text)
        ),
        None,
    )


def fetch_failures(
    lines: Iterable[StampedLine], *, run: int
) -> tuple[FetchFailure, ...]:
    return tuple(
        FetchFailure(
            run=run, task=task, external_input=external_input, line=line.text
        )
        for task, line in attributed_lines(lines)
        if (external_input := _failed_input(line.text)) is not None
    )
