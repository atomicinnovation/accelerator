"""The hidden marker that identifies what a guard issue reports.

An issue carries one marker, on its body's last line, so no other text in a
body — an echoed feed value, a listing of would-be findings — can be read as
the identity of the issue.
"""

import re
from dataclasses import dataclass
from enum import StrEnum
from typing import Self

_PART = re.compile(r"[A-Za-z0-9._-]+")
_RENDERED = re.compile(
    r"<!-- (?P<kind>[a-z-]+): (?P<parts>[A-Za-z0-9._ -]+) -->"
)


class MarkerKind(StrEnum):
    FINDING = "finding"
    FEED_FAILURE = "feed-failure"
    GUARD_TRIPPED = "guard-tripped"


@dataclass(frozen=True, order=True, slots=True)
class IssueMarker:
    kind: MarkerKind
    parts: tuple[str, ...]

    def __post_init__(self) -> None:
        if not self.parts:
            raise ValueError("a marker needs at least one part")
        for part in self.parts:
            if not _PART.fullmatch(part):
                raise ValueError(
                    f"marker part {part!r} must match {_PART.pattern}"
                )

    def render(self) -> str:
        return f"<!-- {self.kind}: {' '.join(self.parts)} -->"

    @classmethod
    def of_body(cls, body: str) -> Self | None:
        """Return the marker on ``body``'s last non-empty line, if any."""
        lines = [line.strip() for line in body.splitlines() if line.strip()]
        if not lines:
            return None
        match = _RENDERED.fullmatch(lines[-1])
        if match is None:
            return None
        try:
            return cls(MarkerKind(match["kind"]), tuple(match["parts"].split()))
        except ValueError:
            return None
