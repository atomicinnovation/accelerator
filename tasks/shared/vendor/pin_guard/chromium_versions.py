"""Chromium's four-part browser versions, compared part by part."""

import re
from dataclasses import dataclass
from typing import Self, override

_FOUR_PART = re.compile(r"(\d+)\.(\d+)\.(\d+)\.(\d+)", re.ASCII)


class ChromiumVersionError(ValueError):
    """The text is not a four-part Chromium version."""


@dataclass(frozen=True, order=True, slots=True)
class ChromiumVersion:
    parts: tuple[int, int, int, int]

    @classmethod
    def parse(cls, text: str) -> Self:
        match = _FOUR_PART.fullmatch(text)
        if match is None:
            raise ChromiumVersionError(
                f"not a four-part Chromium version: {text!r}"
            )
        major, minor, build, patch = (int(part) for part in match.groups())
        return cls((major, minor, build, patch))

    @property
    def major(self) -> int:
        return self.parts[0]

    @override
    def __str__(self) -> str:
        return ".".join(str(part) for part in self.parts)
