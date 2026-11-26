"""A run's lifecycle: build each family once, hand it over, judge and clean up.

Every failure after the root is opened takes one of two paths. An ordinary
failure cleans up fully and judges the log. An interrupted run only removes
what it made: tests killed mid-write leave trees and log lines that would
yield bogus problems.
"""

import contextlib
import dataclasses
import re
import time
from collections.abc import Callable, Collection, Iterator, Mapping, Sequence
from pathlib import Path
from typing import TextIO

from tasks.shared.hashing import TreeDigest, tree_digest
from tasks.shared.shared_fixtures.errors import SharedFixturesError
from tasks.shared.shared_fixtures.families import HOLD_VARIABLE, Family
from tasks.shared.shared_fixtures.log import (
    DigestPhase,
    FinishedRun,
    FixtureLog,
    FixtureLogs,
    SharedBuild,
    TreeDigests,
)
from tasks.shared.shared_fixtures.runs import (
    FixtureRun,
    FixtureRuns,
    HoldPoint,
    seal,
)
from tasks.shared.signals import UNWIND_SIGNALS, deferred_signals

_LOGS_KEPT = 20

type Build = Callable[[Mapping[Family, Path], Path], None]


class SharedBuildError(SharedFixturesError): ...


_SAYING = "shared VCS fixtures: "

FAMILY_BUILT = re.compile(
    rf"{re.escape(_SAYING)}(?P<heading>.+) built in (?P<seconds>[\d.]+) s"
)
"""A line saying how long one family took to build, as the session says it."""


def _listed(families: Collection[Family]) -> str:
    *others, last = families
    return f"{', '.join(others)} and {last}" if others else last


@dataclasses.dataclass
class _Progress:
    """What a run got as far as making, which is what cleanup undoes."""

    run: FixtureRun
    log: FixtureLog | None = None
    trees: dict[Family, Path] = dataclasses.field(default_factory=dict)
    built: dict[Family, TreeDigest] = dataclasses.field(default_factory=dict)


class SharedFixtureSession:
    def __init__(
        self,
        root: Path,
        logs: Path,
        environ: Mapping[str, str],
        *,
        interrupted: Callable[[], bool] = lambda: False,
        hold_point: Callable[
            [Mapping[str, str]], HoldPoint | None
        ] = HoldPoint.from_environment,
        runs: Callable[[Path], FixtureRuns] = FixtureRuns.open,
        logs_store: Callable[[Path], FixtureLogs] = FixtureLogs,
        digest: Callable[[Path], TreeDigest] = tree_digest,
        out: TextIO | None = None,
    ) -> None:
        self._root = root
        self._logs = logs
        self._environ = environ
        self._interrupted = interrupted
        self._hold_point = hold_point
        self._runs = runs
        self._logs_store = logs_store
        self._digest = digest
        self._out = out

    @contextlib.contextmanager
    def open(
        self,
        families: Collection[Family],
        build: Build,
        filtered: bool,
    ) -> Iterator[Mapping[str, str]]:
        runs = self._runs(self._root)
        runs.sweep_abandoned()
        progress = _Progress(runs.start())
        try:
            yield self._prepare(progress, families, build)
        except BaseException as failure:
            if not isinstance(failure, Exception):
                self._clean_up_after_interrupt(progress)
                raise
            if self._interrupted():
                self._clean_up_after_interrupt(progress)
                raise KeyboardInterrupt from failure
            self._finish(progress, runs, filtered, failure)
        else:
            if self._interrupted():
                self._clean_up_after_interrupt(progress)
                raise KeyboardInterrupt
            self._finish(progress, runs, filtered, None)

    def _prepare(
        self,
        progress: _Progress,
        families: Collection[Family],
        build: Build,
    ) -> Mapping[str, str]:
        hold = self._hold_point(self._environ)
        log = self._logs_store(self._logs).create(progress.run.run_id)
        progress.log = log
        if families:
            self._build(progress, log, families, build)
        else:
            self._say(
                "building nothing (no selected test consumes a shared family)"
            )
        if hold is not None:
            self._say(
                f"held by {HOLD_VARIABLE}; touch {hold.directory}/release to "
                f"continue (run directory: {progress.run.directory})"
            )
            hold.wait(progress.run.directory)
        return progress.run.handover(families, log.path)

    def _build(
        self,
        progress: _Progress,
        log: FixtureLog,
        families: Collection[Family],
        build: Build,
    ) -> None:
        self._say(f"building {_listed(families)}")
        started = time.monotonic()
        trees = {family: progress.run.tree(family) for family in families}
        try:
            build(trees, log.path)
        except SharedFixturesError as error:
            raise SharedBuildError(*error.problems) from error
        built = time.monotonic()
        for family, tree in trees.items():
            seal(tree)
            progress.built[family] = self._digest(tree)
            progress.trees[family] = tree
            log.record_digest(
                family, tree, DigestPhase.BUILT, progress.built[family]
            )
        self._say(
            f"built in {built - started:.1f} s, "
            f"digested in {time.monotonic() - built:.1f} s"
        )
        for event in log.read().events:
            if isinstance(event, SharedBuild):
                self._say(
                    f"{event.family.heading} built in {event.seconds:.1f} s"
                )

    def _finish(
        self,
        progress: _Progress,
        runs: FixtureRuns,
        filtered: bool,
        failure: Exception | None,
    ) -> None:
        with deferred_signals(UNWIND_SIGNALS):
            problems = self._clean_up(progress, runs, filtered, failure)
        if failure is not None and not problems:
            raise failure
        if not problems:
            return
        joined = None if failure is None else str(failure) or repr(failure)
        if isinstance(failure, SharedBuildError):
            raise SharedBuildError(*problems, failure=joined) from failure
        raise SharedFixturesError(*problems, failure=joined) from failure

    def _clean_up(
        self,
        progress: _Progress,
        runs: FixtureRuns,
        filtered: bool,
        failure: Exception | None,
    ) -> list[str]:
        problems: list[str] = []
        started = time.monotonic()
        digests = self._digest_again(progress, problems)
        digested = time.monotonic()
        problems.extend(self._remove(progress))
        removed = time.monotonic()
        self._prune_old_logs(runs)
        if progress.log is not None:
            problems.extend(
                self._judge(
                    progress.log,
                    FinishedRun(
                        trees=progress.trees,
                        digests=digests,
                        filtered=filtered,
                        tests_passed=failure is None,
                    ),
                )
            )
            problems.extend(self._summarise(progress.log))
        self._say(
            f"digested again in {digested - started:.1f} s, "
            f"removed in {removed - digested:.1f} s"
        )
        return problems

    def _remove(self, progress: _Progress) -> Sequence[str]:
        try:
            progress.run.close()
        except Exception as error:  # noqa: BLE001
            return _problems_of(error)
        return ()

    def _prune_old_logs(self, runs: FixtureRuns) -> None:
        try:
            self._logs_store(self._logs).prune(
                keep=_LOGS_KEPT, live=runs.live_run_ids()
            )
        except Exception as error:  # noqa: BLE001
            self._say(f"WARNING: pruning old fixture logs failed: {error}")

    def _judge(self, log: FixtureLog, run: FinishedRun) -> list[str]:
        try:
            return log.problems(run)
        except Exception as error:  # noqa: BLE001
            return [f"judging the fixture log failed: {error}"]

    def _digest_again(
        self, progress: _Progress, problems: list[str]
    ) -> dict[Family, TreeDigests]:
        digests: dict[Family, TreeDigests] = {}
        for family, built in progress.built.items():
            tree = progress.trees[family]
            try:
                removed = self._digest(tree)
                if progress.log is not None:
                    progress.log.record_digest(
                        family, tree, DigestPhase.REMOVED, removed
                    )
                    if removed.value != built.value:
                        progress.log.record_changes(
                            family, tree, removed.changes_since(built)
                        )
            except Exception as error:  # noqa: BLE001
                problems.append(
                    f"could not digest {family.noun} tree {tree} a second "
                    f"time: {error}"
                )
            else:
                digests[family] = TreeDigests(built, removed)
        return digests

    def _clean_up_after_interrupt(self, progress: _Progress) -> None:
        try:
            progress.run.close()
        except Exception as error:  # noqa: BLE001
            self._say(f"WARNING: {error}")
        if progress.log is not None:
            for warning in self._summarise(progress.log):
                self._say(f"WARNING: {warning}")

    def _summarise(self, log: FixtureLog) -> list[str]:
        try:
            for line in log.summary(self._root).lines():
                print(line, file=self._out, flush=True)
        except Exception as error:  # noqa: BLE001
            return [f"summarising the fixture log failed: {error}"]
        return []

    def _say(self, message: str) -> None:
        print(f"{_SAYING}{message}", file=self._out, flush=True)


def _problems_of(error: Exception) -> Sequence[str]:
    if isinstance(error, SharedFixturesError):
        return error.problems
    return (str(error),)
