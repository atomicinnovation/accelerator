"""Which artefacts in the instrumented build the current lane still uses."""

import dataclasses
import filecmp
import json
import re
import shlex
from collections.abc import Iterable, Iterator, Mapping
from pathlib import Path
from typing import Any

_UNIT_HASH = re.compile(r"-([0-9a-f]{16})(?=\.|$)")
_SWEPT = ("deps", "build", ".fingerprint")
_SIZE_UNITS = ("kB", "MB", "GB", "TB")


class UnmatchedUpliftError(Exception): ...


@dataclasses.dataclass(frozen=True)
class UnitListing:
    live_hashes: frozenset[str]
    uplifted: frozenset[Path]
    rebuilt: int

    @classmethod
    def parse(cls, messages: str) -> UnitListing:
        hashes: set[str] = set()
        uplifted: set[Path] = set()
        rebuilt = 0
        for message in _json_messages(messages):
            if message.get("reason") == "compiler-artifact":
                rebuilt += not message["fresh"]
                for path in _artefact_paths(message):
                    if path_hashes := _hashes([path]):
                        hashes.update(path_hashes)
                    else:
                        uplifted.add(Path(path))
            elif message.get("reason") == "build-script-executed":
                hashes.update(_hashes([message["out_dir"]]))
        return cls(frozenset(hashes), frozenset(uplifted), rebuilt)


@dataclasses.dataclass(frozen=True)
class Sweep:
    entries: int
    freed_bytes: int

    def summary_line(self) -> str:
        return (
            f"swept {self.entries} superseded artefacts "
            f"({_size(self.freed_bytes)})"
        )


def live_units(listing: UnitListing) -> frozenset[str]:
    """Add the hash of each uplifted artefact's twin in deps.

    cargo reports a binary only by the hash-free copy it uplifts out of deps,
    so the unit's hash is recovered from the deps file that copy matches.
    """
    twins = {path: _twin_hash(path) for path in listing.uplifted}
    unmatched = sorted(path.name for path, unit in twins.items() if not unit)
    if unmatched:
        raise UnmatchedUpliftError(
            f"no build in deps matches {', '.join(unmatched)}"
        )
    return listing.live_hashes | {unit for unit in twins.values() if unit}


def superseded_entries(build_dir: Path, live: frozenset[str]) -> list[Path]:
    if not live:
        raise ValueError("no live units, so nothing can be judged superseded")
    return [
        entry
        for swept in _SWEPT
        if (build_dir / swept).is_dir()
        for entry in sorted((build_dir / swept).iterdir())
        if (unit := _unit_hash(entry.name)) and unit not in live
    ]


def parse_build_environment(text: str) -> Mapping[str, str]:
    return {
        key: "".join(shlex.split(value))
        for key, _, value in (
            line.partition("=") for line in text.splitlines() if "=" in line
        )
    }


def _json_messages(messages: str) -> Iterator[dict[str, Any]]:
    for line in messages.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(message, dict):
            yield message


def _artefact_paths(message: Mapping[str, Any]) -> list[str]:
    executable = message.get("executable")
    return [*message["filenames"], *([executable] if executable else [])]


def _hashes(paths: Iterable[str]) -> set[str]:
    return {
        unit
        for path in paths
        for component in Path(path).parts
        for unit in _UNIT_HASH.findall(component)
    }


def _twin_hash(uplifted: Path) -> str | None:
    crate = uplifted.name.replace("-", "_")
    twin = re.compile(rf"{re.escape(crate)}-([0-9a-f]{{16}})")
    for candidate in sorted((uplifted.parent / "deps").glob(f"{crate}-*")):
        match = twin.fullmatch(candidate.name)
        if match and filecmp.cmp(uplifted, candidate, shallow=False):
            return match[1]
    return None


def _unit_hash(name: str) -> str | None:
    units = _UNIT_HASH.findall(name)
    return units[-1] if units else None


def _size(freed_bytes: int) -> str:
    size, unit = float(freed_bytes), "B"
    for larger in _SIZE_UNITS:
        if size < 1000:
            break
        size, unit = size / 1000, larger
    return f"{freed_bytes} B" if unit == "B" else f"{size:.2f} {unit}"
