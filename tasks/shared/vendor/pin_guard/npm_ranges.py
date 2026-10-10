"""Version ranges in the subset of npm syntax that Node's ``vuln/core`` uses.

Anything outside that subset is refused rather than guessed at, so new
syntax in the feed surfaces as a parse failure instead of a silent miss.
"""

import re
from dataclasses import dataclass
from enum import StrEnum
from typing import Self

import semver

_TOKEN = re.compile(r"(?P<operator>>=|<=|>|<|\^)?(?P<version>[0-9x.]+)")
_PARTIAL_VERSION = re.compile(
    r"(?P<major>\d+)(?:\.(?P<minor>\d+)(?:\.(?P<patch>\d+))?)?(?:\.x)*"
)
_OPERATOR_GAP = re.compile(r"(>=|<=|>|<|\^)\s+")


class RangeSyntaxError(ValueError):
    """The range uses syntax outside the supported subset."""


class Operator(StrEnum):
    AT_LEAST = ">="
    BELOW = "<"


@dataclass(frozen=True, slots=True)
class Comparator:
    operator: Operator
    bound: semver.Version

    def admits(self, version: semver.Version) -> bool:
        match self.operator:
            case Operator.AT_LEAST:
                return version >= self.bound
            case Operator.BELOW:
                return version < self.bound


@dataclass(frozen=True, slots=True)
class _PartialVersion:
    """A version whose trailing parts may be left open, as in ``10.x``."""

    major: int
    minor: int | None
    patch: int | None

    @classmethod
    def parse(cls, text: str) -> Self:
        match = _PARTIAL_VERSION.fullmatch(text)
        if match is None or text.count(".") > 2:
            raise RangeSyntaxError(f"unsupported version {text!r}")
        return cls(
            int(match["major"]),
            _number(match["minor"]),
            _number(match["patch"]),
        )

    @property
    def is_complete(self) -> bool:
        return self.patch is not None

    @property
    def lowest(self) -> semver.Version:
        return semver.Version(self.major, self.minor or 0, self.patch or 0)

    @property
    def first_beyond(self) -> semver.Version:
        """The lowest version above every version this one covers."""
        if self.minor is None:
            return semver.Version(self.major + 1, 0, 0)
        if self.patch is None:
            return semver.Version(self.major, self.minor + 1, 0)
        return self.lowest.bump_patch()

    @property
    def caret_ceiling(self) -> semver.Version:
        if self.major > 0:
            return self.lowest.bump_major()
        if self.minor:
            return self.lowest.bump_minor()
        return self.lowest.bump_patch()


@dataclass(frozen=True, slots=True)
class VersionRange:
    alternatives: tuple[tuple[Comparator, ...], ...]

    @classmethod
    def parse(cls, text: str) -> Self:
        return cls(
            tuple(
                _comparator_set(alternative) for alternative in text.split("||")
            )
        )

    def contains(self, version: semver.Version) -> bool:
        return any(
            all(comparator.admits(version) for comparator in alternative)
            for alternative in self.alternatives
        )


def _number(text: str | None) -> int | None:
    return None if text is None else int(text)


def _comparator_set(alternative: str) -> tuple[Comparator, ...]:
    tokens = _OPERATOR_GAP.sub(r"\1", alternative).split()
    if not tokens:
        raise RangeSyntaxError(f"empty alternative in {alternative!r}")
    return tuple(
        comparator for token in tokens for comparator in _comparators(token)
    )


def _comparators(token: str) -> tuple[Comparator, ...]:
    comparator = _TOKEN.fullmatch(token)
    if comparator is None:
        raise RangeSyntaxError(f"unsupported comparator {token!r}")
    version = _PartialVersion.parse(comparator["version"])
    lowest, beyond = version.lowest, version.first_beyond
    match comparator["operator"]:
        case None:
            return (
                Comparator(Operator.AT_LEAST, lowest),
                Comparator(Operator.BELOW, beyond),
            )
        case "^" if version.is_complete:
            return (
                Comparator(Operator.AT_LEAST, lowest),
                Comparator(Operator.BELOW, version.caret_ceiling),
            )
        case "^":
            raise RangeSyntaxError(f"caret on a partial version {token!r}")
        case ">=":
            return (Comparator(Operator.AT_LEAST, lowest),)
        case ">":
            return (Comparator(Operator.AT_LEAST, beyond),)
        case "<=":
            return (Comparator(Operator.BELOW, beyond),)
        case _:
            return (Comparator(Operator.BELOW, lowest),)
