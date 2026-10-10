"""The repository-held inputs the guard reads before it consults any feed.

Every input is validated here, so a missing or malformed one aborts the run
with its name rather than reading as "not stale" or "not affected".
"""

import datetime as dt
import re
import tomllib
from collections.abc import Callable
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Self

from tasks.shared.paths import (
    KEYS_DIR,
    PINS_TOML,
    PLAYWRIGHT_PACKAGE_JSON,
    RELEASING_MD,
)
from tasks.shared.vendor import assemble, pins
from tasks.shared.vendor.pin_guard.aborts import GuardAbortError

KEYRING_BUMP_SUBJECT = "keyring"
NODE_KEYRING_NAME = "keys/nodejs-release.asc"
_OWNER_LINE = re.compile(r"^Owner: (.+) \(@([A-Za-z0-9-]+)\)$", re.MULTILINE)
_NODE_VERSION = re.compile(r"^\d+\.\d+\.\d+$")
_CHROMIUM_REVISION = re.compile(r"^\d+$")


class LocalInputError(GuardAbortError):
    """A local input is missing or malformed."""


class PinName(StrEnum):
    PLAYWRIGHT_CORE = "playwright-core"
    NODE = "node"
    CHROMIUM = "chromium"


@dataclass(frozen=True, slots=True)
class Pin:
    name: PinName
    version: str
    bumped: dt.date


@dataclass(frozen=True, slots=True)
class Owner:
    name: str
    login: str

    @classmethod
    def from_releasing(cls, text: str) -> Self:
        """Parse ``RELEASING.md``'s single ``Owner: Name (@login)`` line."""
        matches = _OWNER_LINE.findall(text)
        if len(matches) != 1:
            raise LocalInputError(
                "RELEASING.md Owner: line must appear exactly once in the "
                f"form 'Owner: Name (@login)', found {len(matches)}"
            )
        name, login = matches[0]
        return cls(name, login)


@dataclass(frozen=True, slots=True)
class LocalInputPaths:
    pins: Path
    package_json: Path
    releasing: Path
    node_keyring: Path

    @classmethod
    def repository(cls) -> Self:
        return cls(
            pins=PINS_TOML,
            package_json=PLAYWRIGHT_PACKAGE_JSON,
            releasing=RELEASING_MD,
            node_keyring=KEYS_DIR / "nodejs-release.asc",
        )


@dataclass(frozen=True, slots=True)
class LocalInputs:
    playwright_core: Pin
    node: Pin
    chromium: Pin
    keyring_bumped: dt.date
    owner: Owner
    node_keyring: Path

    @property
    def pins(self) -> tuple[Pin, ...]:
        return (self.playwright_core, self.node, self.chromium)


def read_local_inputs(paths: LocalInputPaths, today: dt.date) -> LocalInputs:
    """Read and validate every local input, raising ``LocalInputError``."""
    pins_file = _PinsFile(paths.pins, today)
    return LocalInputs(
        playwright_core=Pin(
            PinName.PLAYWRIGHT_CORE,
            _playwright_version(paths.package_json),
            pins_file.bump_date(PinName.PLAYWRIGHT_CORE),
        ),
        node=Pin(
            PinName.NODE,
            pins_file.matching(
                "node.version", pins.node_version, _NODE_VERSION
            ),
            pins_file.bump_date(PinName.NODE),
        ),
        chromium=Pin(
            PinName.CHROMIUM,
            pins_file.matching(
                "chromium.revision", pins.chromium_revision, _CHROMIUM_REVISION
            ),
            pins_file.bump_date(PinName.CHROMIUM),
        ),
        keyring_bumped=pins_file.bump_date(KEYRING_BUMP_SUBJECT),
        owner=_owner(paths.releasing),
        node_keyring=_node_keyring(paths.node_keyring),
    )


@dataclass(frozen=True, slots=True)
class _PinsFile:
    path: Path
    today: dt.date

    def bump_date(self, subject: str) -> dt.date:
        field = f"{subject}.bumped"
        value = self._read(field, lambda: pins.bump_date(subject, self.path))
        if type(value) is not dt.date or value > self.today:
            raise LocalInputError(
                f"{self.path.name}: {field} must be a TOML local date on or "
                f"before {self.today.isoformat()}, got {value!r}"
            )
        return value

    def matching(
        self, field: str, read: Callable[[Path], str], form: re.Pattern[str]
    ) -> str:
        value = self._read(field, lambda: read(self.path))
        if not form.match(value):
            raise LocalInputError(
                f"{self.path.name}: {field} must match {form.pattern}, "
                f"got {value!r}"
            )
        return value

    def _read[T](self, field: str, read: Callable[[], T]) -> T:
        try:
            value = read()
        except tomllib.TOMLDecodeError as error:
            line = error.doc.splitlines()[error.lineno - 1]
            raise LocalInputError(
                f"{self.path.name} line {error.lineno}: {line} "
                f"is not valid TOML ({error.msg})"
            ) from error
        except OSError as error:
            raise LocalInputError(f"{self.path.name}: {error}") from error
        except (KeyError, TypeError) as error:
            raise LocalInputError(
                f"{self.path.name}: {field} is missing"
            ) from error
        if value is None:
            raise LocalInputError(f"{self.path.name}: {field} is missing")
        return value


def _playwright_version(package_json: Path) -> str:
    try:
        return assemble.pinned_playwright_version(package_json)
    except (OSError, ValueError, AttributeError) as error:
        raise LocalInputError(
            f"{package_json}: the playwright version is unreadable: {error}"
        ) from error


def _owner(releasing: Path) -> Owner:
    try:
        text = releasing.read_text()
    except OSError as error:
        raise LocalInputError(
            f"RELEASING.md Owner: line is unreadable: {error}"
        ) from error
    return Owner.from_releasing(text)


def _node_keyring(path: Path) -> Path:
    if not path.is_file() or path.stat().st_size == 0:
        raise LocalInputError(
            f"{NODE_KEYRING_NAME} is absent or empty at {path}"
        )
    return path
