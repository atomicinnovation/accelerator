"""The per-run fixture log: what the build step and each test recorded.

Tests append to it through ``vcs-test-support``'s ``FixtureLog``; the task
appends its own digests. ``cli/vcs-test-support/contract/fixture-log.jsonl``
holds one line of every kind, and both sides test against it.
"""

import dataclasses
import json
from collections.abc import Collection, Iterator, Mapping, Sequence
from enum import StrEnum
from pathlib import Path
from typing import Any

from tasks.shared.hashing import TreeChanges, TreeDigest
from tasks.shared.shared_fixtures.consumers import consumers_of
from tasks.shared.shared_fixtures.families import Family
from tasks.shared.shared_fixtures.runs import RUN_ID

_CONSUMERS_MODULE = "tasks/shared/shared_fixtures/consumers.py"
_CHANGES_SHOWN = 10


class DigestPhase(StrEnum):
    BUILT = "built"
    REMOVED = "removed"


class _UnreadableEventError(Exception): ...


@dataclasses.dataclass(frozen=True)
class SharedBuild:
    family: Family
    tree: Path
    git: str
    jj: str


@dataclasses.dataclass(frozen=True)
class Adoption:
    family: Family
    test: str
    hermetic_root: Path

    @property
    def binary(self) -> str:
        return _binary(self.test)


@dataclasses.dataclass(frozen=True)
class PrivateBuild:
    """A fixture a test built for itself; no family means the private sets."""

    family: Family | None
    test: str
    hermetic_root: Path

    @property
    def binary(self) -> str:
        return _binary(self.test)


@dataclasses.dataclass(frozen=True)
class TreeDigestRecorded:
    family: Family
    tree: Path
    phase: DigestPhase
    value: str


@dataclasses.dataclass(frozen=True)
class TreeChangesRecorded:
    family: Family
    tree: Path
    changes: TreeChanges


type Event = (
    SharedBuild
    | Adoption
    | PrivateBuild
    | TreeDigestRecorded
    | TreeChangesRecorded
)


@dataclasses.dataclass(frozen=True)
class LogContents:
    events: tuple[Event, ...]
    unparsed: tuple[int, ...]


@dataclasses.dataclass(frozen=True)
class TreeDigests:
    built: TreeDigest
    removed: TreeDigest


@dataclasses.dataclass(frozen=True)
class FinishedRun:
    """What the log is judged against once a run's tests have finished.

    ``trees`` holds each family whose build returned and was digested;
    ``digests`` only the trees that could be digested both times.
    """

    trees: Mapping[Family, Path]
    digests: Mapping[Family, TreeDigests]
    filtered: bool
    tests_passed: bool


@dataclasses.dataclass(frozen=True)
class FixtureSummary:
    log: Path
    root: Path
    events: Sequence[Event]

    def lines(self) -> list[str]:
        return [
            f"Shared VCS fixtures (log: {self.log}; shared root: {self.root})",
            *(self._family_line(family) for family in Family),
            "  private sets: "
            + _count(self._private_builds(None), "private build"),
        ]

    def _family_line(self, family: Family) -> str:
        shared_builds = sum(
            isinstance(event, SharedBuild) and event.family is family
            for event in self.events
        )
        adoptions = sum(
            isinstance(event, Adoption) and event.family is family
            for event in self.events
        )
        return (
            f"  {family.heading}: {_count(shared_builds, 'shared build')}, "
            f"{_count(adoptions, 'adoption')}, {_count(0, 'copy', 'copies')}, "
            f"{_count(self._private_builds(family), 'private build')}"
        )

    def _private_builds(self, family: Family | None) -> int:
        return sum(
            isinstance(event, PrivateBuild) and event.family is family
            for event in self.events
        )


class FixtureLog:
    def __init__(self, path: Path) -> None:
        self.path = path

    def read(self) -> LogContents:
        if not self.path.exists():
            return LogContents((), ())
        events: list[Event] = []
        unparsed: list[int] = []
        for number, line in enumerate(self.path.read_text().splitlines(), 1):
            try:
                events.append(_event(json.loads(line)))
            except ValueError, KeyError, TypeError, _UnreadableEventError:
                unparsed.append(number)
        return LogContents(tuple(events), tuple(unparsed))

    def summary(self, root: Path) -> FixtureSummary:
        return FixtureSummary(self.path, root, self.read().events)

    def record_digest(
        self, family: Family, tree: Path, phase: DigestPhase, digest: TreeDigest
    ) -> None:
        self._append(
            {
                "event": "tree-digest",
                "family": family.value,
                "tree": str(tree),
                "phase": phase.value,
                "value": digest.value,
            }
        )

    def record_changes(
        self, family: Family, tree: Path, changes: TreeChanges
    ) -> None:
        self._append(
            {
                "event": "tree-changes",
                "family": family.value,
                "tree": str(tree),
                "added": list(changes.added),
                "removed": list(changes.removed),
                "modified": list(changes.modified),
            }
        )

    def problems(self, run: FinishedRun) -> list[str]:
        contents = self.read()
        events = contents.events
        problems = [
            *_changed_trees(run),
            *_private_builds_of_shared_families(events, run),
            *(
                f"line {number} of the fixture log {self.path} does not parse"
                for number in contents.unparsed
            ),
            *_builds_missing_from_the_log(events, run),
        ]
        if not run.filtered:
            problems.extend(_unlisted_consumers(events))
            if run.tests_passed:
                problems.extend(_silent_consumers(events, run))
        return problems

    def _append(self, event: Mapping[str, Any]) -> None:
        with self.path.open("a") as log:
            log.write(json.dumps(event, separators=(",", ":")) + "\n")


class FixtureLogs:
    """The directory of run logs, kept after each run for inspection."""

    def __init__(self, directory: Path) -> None:
        self.directory = directory

    def create(self, run_id: str) -> FixtureLog:
        self.directory.mkdir(parents=True, exist_ok=True)
        path = self.directory / f"{run_id}.jsonl"
        path.touch(exist_ok=False)
        return FixtureLog(path)

    def prune(self, keep: int, live: Collection[str]) -> None:
        newest_first = sorted(self.listed(), key=lambda log: log.name)[::-1]
        for log in newest_first[keep:]:
            if log.stem not in live:
                log.unlink(missing_ok=True)

    def listed(self) -> list[Path]:
        if not self.directory.is_dir():
            return []
        return [
            log
            for log in self.directory.iterdir()
            if log.suffix == ".jsonl" and RUN_ID.fullmatch(log.stem)
        ]


def _event(line: Mapping[str, Any]) -> Event:
    kind = line["event"]
    if kind == "shared-build":
        return SharedBuild(
            Family(line["family"]), Path(line["tree"]), line["git"], line["jj"]
        )
    if kind == "adoption":
        return Adoption(
            Family(line["family"]), line["test"], Path(line["hermetic-root"])
        )
    if kind == "private-build":
        fixtures = line["fixtures"]
        return PrivateBuild(
            None if fixtures == "private-sets" else Family(fixtures),
            line["test"],
            Path(line["hermetic-root"]),
        )
    if kind == "tree-digest":
        return TreeDigestRecorded(
            Family(line["family"]),
            Path(line["tree"]),
            DigestPhase(line["phase"]),
            line["value"],
        )
    if kind == "tree-changes":
        return TreeChangesRecorded(
            Family(line["family"]),
            Path(line["tree"]),
            TreeChanges(
                added=tuple(line["added"]),
                removed=tuple(line["removed"]),
                modified=tuple(line["modified"]),
            ),
        )
    raise _UnreadableEventError(kind)


def _changed_trees(run: FinishedRun) -> Iterator[str]:
    for family, digests in run.digests.items():
        if digests.removed.value == digests.built.value:
            continue
        changes = digests.removed.changes_since(digests.built)
        reruns = ", ".join(
            f"mise run test:unit:cli --filter 'binary_id({consumer})'"
            for consumer in sorted(consumers_of(family))
        )
        yield (
            f"{family.noun} tree {run.trees.get(family)} changed during the "
            f"run: {_described(changes)}; the tree has been removed; to find "
            f"the test that wrote it, rerun each consumer alone with {reruns}"
        )


def _described(changes: TreeChanges) -> str:
    listed = [
        (kind, path)
        for kind, paths in (
            ("added", changes.added),
            ("removed", changes.removed),
            ("modified", changes.modified),
        )
        for path in paths
    ]
    shown: dict[str, list[str]] = {}
    for kind, path in listed[:_CHANGES_SHOWN]:
        shown.setdefault(kind, []).append(path)
    described = "; ".join(
        f"{kind} {', '.join(paths)}" for kind, paths in shown.items()
    )
    hidden = len(listed) - _CHANGES_SHOWN
    return f"{described}; and {hidden} more" if hidden > 0 else described


def _private_builds_of_shared_families(
    events: Sequence[Event], run: FinishedRun
) -> Iterator[str]:
    for event in events:
        if not isinstance(event, PrivateBuild) or event.family is None:
            continue
        family = event.family
        if family in run.trees:
            yield (
                f"{event.test} built {family.noun} privately although the "
                f"run handed it over through {family.handover_variable}; its "
                "Handover did not come from the environment; open it with "
                "Workbench::for_this_test(), or give a deliberately private "
                "Handover a test-local FixtureLog"
            )
        else:
            yield (
                f"{event.test} built {family.noun} privately; add "
                f"{event.binary} to the {family.heading} consumers in "
                f"{_CONSUMERS_MODULE}"
            )


def _builds_missing_from_the_log(
    events: Sequence[Event], run: FinishedRun
) -> Iterator[str]:
    recorded = {
        (event.family, event.tree)
        for event in events
        if isinstance(event, SharedBuild)
    }
    for family, tree in run.trees.items():
        if (family, tree) not in recorded:
            yield (
                f"{family.noun} was built but the log records no shared "
                f"build of {tree}; the hand-over did not reach the builder"
            )


def _unlisted_consumers(events: Sequence[Event]) -> Iterator[str]:
    users = sorted(
        {
            (event.binary, event.family)
            for event in events
            if isinstance(event, Adoption)
        }
    )
    for binary, family in users:
        if binary not in consumers_of(family):
            yield (
                f"{binary} uses {family.noun} but is not among its consumers "
                f"in {_CONSUMERS_MODULE}; add it there"
            )


def _silent_consumers(
    events: Sequence[Event], run: FinishedRun
) -> Iterator[str]:
    used = {
        (event.binary, event.family)
        for event in events
        if isinstance(event, Adoption | PrivateBuild)
    }
    for family in run.trees:
        for binary in sorted(consumers_of(family)):
            if (binary, family) not in used:
                yield (
                    f"{binary} is listed as a {family.heading} consumer but "
                    f"used no {family.heading}; remove it from "
                    f"{_CONSUMERS_MODULE}"
                )


def _binary(test: str) -> str:
    return test.partition(" ")[0]


def _count(amount: int, singular: str, plural: str | None = None) -> str:
    noun = singular if amount == 1 else (plural or f"{singular}s")
    return f"{amount} {noun}"
