import io
import json
import os
import re
import signal
import threading
import time
from collections.abc import Callable, Collection, Mapping
from pathlib import Path

import pytest
from invoke import Exit

from tasks.shared.build_measurement import timeline
from tasks.shared.hashing import TreeDigest, tree_digest
from tasks.shared.shared_fixtures.consumers import consumers_of
from tasks.shared.shared_fixtures.errors import SharedFixturesError
from tasks.shared.shared_fixtures.families import (
    FIXTURE_LOG_VARIABLE,
    HOLD_VARIABLE,
    SHARED_FAMILIES,
    WORKBENCHES_VARIABLE,
    Family,
)
from tasks.shared.shared_fixtures.log import FixtureLogs
from tasks.shared.shared_fixtures.runs import (
    FixtureRuns,
    HoldPoint,
    remove_leftover,
)
from tasks.shared.shared_fixtures.session import (
    SharedBuildError,
    SharedFixtureSession,
)

ABANDONED_RUN = "20261119T101500123456Z-0b6f9a0e-6c1d-4f0a-9d7e-2c1b3a4d5e6f"
TIMEOUT = 5.0


def _append(log: Path, event: Mapping[str, str]) -> None:
    with log.open("a") as file:
        file.write(json.dumps(event) + "\n")


def build_that_logs(trees: Mapping[Family, Path], log: Path) -> None:
    for family, tree in trees.items():
        tree.mkdir(parents=True)
        (tree / "fixtures.tsv").write_text("PG-r\tplain git\t/x\n")
        _append(
            log,
            {
                "event": "shared-build",
                "family": family.value,
                "tree": str(tree),
                "git": "git version 2.50.1",
                "jj": "jj 0.43.0",
            },
        )


def adopt_everywhere(handover: Mapping[str, str]) -> None:
    for family in SHARED_FAMILIES:
        for binary in consumers_of(family):
            _append(
                Path(handover[FIXTURE_LOG_VARIABLE]),
                {
                    "event": "adoption",
                    "family": family.value,
                    "test": f"{binary} a_test",
                    "hermetic-root": f"{handover[WORKBENCHES_VARIABLE]}/w",
                },
            )


class Harness:
    def __init__(self, tmp_path: Path) -> None:
        self.root = tmp_path / "shared"
        self.logs = tmp_path / "logs"
        self.out = io.StringIO()
        self.arrived = False
        self.build: Callable[[Mapping[Family, Path], Path], None] = (
            build_that_logs
        )

    def session(self, environ: Mapping[str, str] = {}, **collaborators):
        return SharedFixtureSession(
            self.root,
            self.logs,
            environ,
            interrupted=lambda: self.arrived,
            out=self.out,
            **collaborators,
        )

    def run(
        self,
        body: Callable[[Mapping[str, str]], None] = adopt_everywhere,
        *,
        families: Collection[Family] = SHARED_FAMILIES,
        filtered: bool = False,
        **collaborators,
    ) -> Mapping[str, str]:
        with self.session(**collaborators).open(
            families, self.build, filtered=filtered
        ) as handover:
            body(handover)
        return handover

    def run_directories(self) -> list[Path]:
        return [path for path in self.root.iterdir() if path.is_dir()]

    def log(self) -> Path:
        [log] = self.logs.glob("*.jsonl")
        return log

    def events(self, kind: str) -> list[dict]:
        return [
            event
            for line in self.log().read_text().splitlines()
            if (event := json.loads(line))["event"] == kind
        ]

    def printed(self) -> str:
        return self.out.getvalue()


@pytest.fixture
def harness(tmp_path: Path) -> Harness:
    return Harness(tmp_path)


def _write_into_the_tree(handover: Mapping[str, str]) -> None:
    adopt_everywhere(handover)
    tree = Path(handover[Family.MATRIX.handover_variable])
    tree.chmod(0o755)
    (tree / "poison.gitconfig").write_text("x")


def _failing_cli_tests(handover: Mapping[str, str]) -> None:
    raise Exit("nextest: cli tests failed", code=1)


class TestACleanRun:
    def test_hands_over_each_tree_the_workbenches_and_the_log(
        self, harness: Harness
    ):
        handover = harness.run()

        assert set(handover) == {
            Family.MATRIX.handover_variable,
            WORKBENCHES_VARIABLE,
            FIXTURE_LOG_VARIABLE,
        }
        assert handover[Family.MATRIX.handover_variable].endswith("/matrix")

    def test_digests_each_tree_after_the_build_and_before_removal(
        self, harness: Harness
    ):
        harness.run()

        digests = harness.events("tree-digest")
        assert [digest["phase"] for digest in digests] == ["built", "removed"]
        assert digests[0]["value"] == digests[1]["value"]
        assert harness.events("tree-changes") == []

    def test_removes_the_run_and_prints_the_summary(self, harness: Harness):
        harness.run()

        assert list(harness.root.iterdir()) == []
        assert "  matrix: 1 shared build, 4 adoptions" in harness.printed()

    def test_prunes_old_logs(self, harness: Harness):
        logs = FixtureLogs(harness.logs)
        for n in range(25):
            logs.create(
                f"20200101T0000{n:08d}Z-0b6f9a0e-6c1d-4f0a-9d7e-2c1b3a4d5e6f"
            )

        harness.run()

        assert len(list(harness.logs.iterdir())) == 20

    def test_sweeps_an_abandoned_run_before_starting(self, harness: Harness):
        (harness.root / ABANDONED_RUN / "matrix").mkdir(parents=True)
        harness.root.chmod(0o700)
        (harness.root / f"{ABANDONED_RUN}.lock").touch()
        swept_before_the_build: list[bool] = []

        def build(trees: Mapping[Family, Path], log: Path) -> None:
            swept_before_the_build.append(
                not (harness.root / ABANDONED_RUN).exists()
            )
            build_that_logs(trees, log)

        harness.build = build
        harness.run()

        assert swept_before_the_build == [True]

    def test_says_what_it_does_without_reading_as_a_timing_line(
        self, harness: Harness
    ):
        harness.run()

        lines = harness.printed().splitlines()
        assert "shared VCS fixtures: building matrix" in lines
        assert any(
            re.fullmatch(
                r"shared VCS fixtures: built in [\d.]+ s, digested in [\d.]+ s",
                line,
            )
            for line in lines
        )
        assert any(
            re.fullmatch(
                r"shared VCS fixtures: digested again in [\d.]+ s, "
                r"removed in [\d.]+ s",
                line,
            )
            for line in lines
        )
        for line in lines:
            assert not timeline._NEXTEST_SUMMARY.match(line)
            assert not timeline._FINISHED_IN.fullmatch(line)


class TestNothingToShare:
    def _run(self, harness: Harness) -> Mapping[str, str]:
        return harness.run(lambda _handover: None, families=(), filtered=True)

    def test_runs_no_build(self, harness: Harness):
        def build(_trees: Mapping[Family, Path], _log: Path) -> None:
            raise AssertionError("nothing should be built")

        harness.build = build

        self._run(harness)

    def test_hands_over_only_the_workbenches_and_the_log(
        self, harness: Harness
    ):
        assert set(self._run(harness)) == {
            WORKBENCHES_VARIABLE,
            FIXTURE_LOG_VARIABLE,
        }

    def test_says_it_builds_nothing(self, harness: Harness):
        self._run(harness)

        lines = harness.printed().splitlines()
        assert (
            "shared VCS fixtures: building nothing (no selected test "
            "consumes a shared family)"
        ) in lines
        assert not any(
            line.startswith("shared VCS fixtures: built") for line in lines
        )

    def test_cleans_up_and_prints_the_summary(self, harness: Harness):
        self._run(harness)

        assert list(harness.root.iterdir()) == []
        assert "  matrix: 0 shared builds" in harness.printed()


class TestAFailingBuild:
    @pytest.fixture(autouse=True)
    def failing_build(self, harness: Harness) -> None:
        def build(trees: Mapping[Family, Path], log: Path) -> None:
            raise SharedFixturesError("matrix: it broke")

        harness.build = build

    def test_names_the_family_and_its_cause(self, harness: Harness):
        with pytest.raises(SharedBuildError) as failure:
            harness.run()

        assert str(failure.value) == "shared VCS fixtures: matrix: it broke"

    def test_runs_no_tests_and_digests_no_tree(self, harness: Harness):
        ran: list[bool] = []

        with pytest.raises(SharedBuildError):
            harness.run(lambda handover: ran.append(True))

        assert ran == []
        assert harness.events("tree-digest") == []

    def test_still_cleans_up_and_prints_the_summary(self, harness: Harness):
        with pytest.raises(SharedBuildError):
            harness.run()

        assert harness.run_directories() == []
        assert "Shared VCS fixtures (log:" in harness.printed()


class TestARefusedRoot:
    def test_propagates_and_creates_nothing(
        self, harness: Harness, tmp_path: Path
    ):
        (tmp_path / "elsewhere").mkdir()
        harness.root.symlink_to(tmp_path / "elsewhere")

        with pytest.raises(SharedFixturesError, match="is a symlink"):
            harness.run()

        assert not harness.logs.exists()
        assert list((tmp_path / "elsewhere").iterdir()) == []


class TestTheHold:
    def test_waits_for_release_after_the_first_digest(
        self, harness: Harness, tmp_path: Path
    ):
        hold = tmp_path / "hold"

        def release_once_held() -> None:
            deadline = time.monotonic() + TIMEOUT
            while not (hold / "held").exists():
                if time.monotonic() > deadline:
                    return
                time.sleep(0.005)
            (hold / "release").touch()

        releaser = threading.Thread(target=release_once_held)
        releaser.start()
        harness.run(
            hold_point=lambda environ: HoldPoint(
                hold, poll=0.005, timeout=TIMEOUT
            )
        )
        releaser.join(timeout=TIMEOUT)

        assert (
            "shared VCS fixtures: held by ACCELERATOR_SHARED_FIXTURES_HOLD; "
            f"touch {hold}/release to continue (run directory: "
        ) in harness.printed()

    def test_a_hold_nobody_releases_fails_the_run_and_cleans_up(
        self, harness: Harness, tmp_path: Path
    ):
        with pytest.raises(SharedFixturesError, match="nothing released"):
            harness.run(
                hold_point=lambda environ: HoldPoint(
                    tmp_path / "hold", poll=0.001, timeout=0.01
                )
            )

        assert harness.run_directories() == []
        assert "Shared VCS fixtures (log:" in harness.printed()

    def test_a_hold_under_ci_is_refused_before_any_build(
        self, harness: Harness, tmp_path: Path
    ):
        built: list[bool] = []
        harness.build = lambda trees, log: built.append(True)

        session = harness.session(
            {HOLD_VARIABLE: str(tmp_path / "hold"), "CI": "true"}
        )

        with (
            pytest.raises(SharedFixturesError, match=HOLD_VARIABLE),
            session.open(SHARED_FAMILIES, harness.build, filtered=False),
        ):
            pass

        assert built == []
        assert harness.run_directories() == []

    def test_an_interrupt_during_the_hold_is_re_raised(
        self, harness: Harness, tmp_path: Path
    ):
        class Interrupted(HoldPoint):
            def wait(self, run: Path) -> None:
                raise KeyboardInterrupt

        with pytest.raises(KeyboardInterrupt):
            harness.run(hold_point=lambda environ: Interrupted(tmp_path))

        assert harness.run_directories() == []


class TestAnInterruptAsInvokeReportsIt:
    def test_during_the_build_ends_as_an_interrupt(self, harness: Harness):
        def interrupted_build(trees: Mapping[Family, Path], log: Path) -> None:
            harness.arrived = True
            raise SharedFixturesError("matrix: killed")

        harness.build = interrupted_build

        with pytest.raises(KeyboardInterrupt):
            harness.run()

        assert harness.run_directories() == []

    def test_during_the_tests_ends_as_an_interrupt_and_judges_nothing(
        self, harness: Harness
    ):
        def interrupted_tests(handover: Mapping[str, str]) -> None:
            harness.arrived = True
            raise Exit("nextest: cli tests failed", code=1)

        with pytest.raises(KeyboardInterrupt) as interruption:
            harness.run(interrupted_tests)

        assert not isinstance(interruption.value, SharedFixturesError)
        assert harness.run_directories() == []
        assert [d["phase"] for d in harness.events("tree-digest")] == ["built"]

    def test_the_tests_absorbed_still_ends_as_an_interrupt(
        self, harness: Harness
    ):
        def absorbing_the_interrupt(handover: Mapping[str, str]) -> None:
            adopt_everywhere(handover)
            harness.arrived = True

        with pytest.raises(KeyboardInterrupt):
            harness.run(absorbing_the_interrupt)

        assert harness.run_directories() == []
        assert [d["phase"] for d in harness.events("tree-digest")] == ["built"]


class TestAnExitThatIsNotAnException:
    def test_keeps_its_own_type_and_code(self, harness: Harness):
        def exiting(handover: Mapping[str, str]) -> None:
            raise SystemExit(3)

        with pytest.raises(SystemExit) as exiting_run:
            harness.run(exiting)

        assert exiting_run.value.code == 3
        assert harness.run_directories() == []


def _runs_that_cannot_remove(root: Path) -> FixtureRuns:
    def unremovable(path: Path) -> None:
        raise PermissionError(13, "denied", str(path))

    return FixtureRuns.open(root, rmtree=unremovable)


class _LogsThatCannotPrune(FixtureLogs):
    def prune(self, keep: int, live) -> None:
        raise OSError("the disk went away")


class TestAFailingCleanupStep:
    def test_a_removal_failing_during_an_interrupt_still_interrupts(
        self, harness: Harness
    ):
        def interrupted_tests(handover: Mapping[str, str]) -> None:
            harness.arrived = True
            raise Exit("nextest: cli tests failed", code=1)

        with pytest.raises(KeyboardInterrupt):
            harness.run(interrupted_tests, runs=_runs_that_cannot_remove)

        assert "could not remove" in harness.printed()

    def test_a_removal_failing_after_failed_tests_joins_their_failure(
        self, harness: Harness
    ):
        with pytest.raises(SharedFixturesError) as failure:
            harness.run(_failing_cli_tests, runs=_runs_that_cannot_remove)

        [body, problem] = str(failure.value).splitlines()
        assert body == "nextest: cli tests failed"
        assert problem.startswith("shared VCS fixtures: could not remove ")
        assert "Shared VCS fixtures (log:" in harness.printed()

    def test_a_pruning_failure_is_only_a_warning(self, harness: Harness):
        harness.run(logs_store=_LogsThatCannotPrune)

        assert "the disk went away" in harness.printed()
        assert "Shared VCS fixtures (log:" in harness.printed()


class TestASignalDuringCleanup:
    def test_is_held_until_cleanup_ends_then_raised_once(
        self, harness: Harness
    ):
        digests = 0

        def digest_then_signal(tree: Path) -> TreeDigest:
            nonlocal digests
            digests += 1
            if digests == 2:
                os.kill(os.getpid(), signal.SIGINT)
            return tree_digest(tree)

        with pytest.raises(KeyboardInterrupt):
            harness.run(_failing_cli_tests, digest=digest_then_signal)

        assert harness.run_directories() == []
        assert "Shared VCS fixtures (log:" in harness.printed()


class TestFailingTests:
    def test_carry_their_failure_and_skip_the_silent_consumer_check(
        self, harness: Harness
    ):
        with pytest.raises(Exit) as failure:
            harness.run(_failing_cli_tests)

        assert str(failure.value) == "nextest: cli tests failed"
        assert failure.value.code == 1
        assert [d["phase"] for d in harness.events("tree-digest")] == [
            "built",
            "removed",
        ]

    def test_fail_the_run_even_without_a_message(self, harness: Harness):
        def failing_silently(handover: Mapping[str, str]) -> None:
            adopt_everywhere(handover)
            raise Exit(code=1)

        with pytest.raises(Exit) as failure:
            harness.run(failing_silently)

        assert failure.value.code == 1

    def test_without_a_message_are_named_beside_a_problem(
        self, harness: Harness
    ):
        def deleting_silently(handover: Mapping[str, str]) -> None:
            remove_leftover(Path(handover[Family.MATRIX.handover_variable]))
            raise Exit(code=1)

        with pytest.raises(SharedFixturesError) as failure:
            harness.run(deleting_silently)

        [body, problem] = str(failure.value).splitlines()
        assert body == repr(Exit(code=1))
        assert "could not digest the matrix tree" in problem

    def test_that_deleted_a_tree_are_named_with_it(self, harness: Harness):
        def deleting(handover: Mapping[str, str]) -> None:
            remove_leftover(Path(handover[Family.MATRIX.handover_variable]))
            _failing_cli_tests(handover)

        with pytest.raises(SharedFixturesError) as failure:
            harness.run(deleting)

        [body, problem] = str(failure.value).splitlines()
        assert body == "nextest: cli tests failed"
        assert "could not digest the matrix tree" in problem
        assert harness.run_directories() == []
        assert "Shared VCS fixtures (log:" in harness.printed()


no_permission_stops_root = pytest.mark.skipif(
    hasattr(os, "geteuid") and os.geteuid() == 0,
    reason="no permission stops root",
)


class TestASealedTree:
    @no_permission_stops_root
    def test_a_test_writing_into_it_fails_at_the_write(self, harness: Harness):
        def writing(handover: Mapping[str, str]) -> None:
            tree = Path(handover[Family.MATRIX.handover_variable])
            (tree / "poison.gitconfig").write_text("x")

        with pytest.raises(PermissionError):
            harness.run(writing)

        assert harness.run_directories() == []


class TestAChangedTree:
    def test_fails_the_run_naming_the_tree_and_the_change(
        self, harness: Harness
    ):
        with pytest.raises(SharedFixturesError) as failure:
            harness.run(_write_into_the_tree)

        assert re.search(
            r"the matrix tree \S+/matrix changed during the run: "
            r"added poison\.gitconfig",
            str(failure.value),
        )
        assert [d["phase"] for d in harness.events("tree-digest")] == [
            "built",
            "removed",
        ]
        assert [c["added"] for c in harness.events("tree-changes")] == [
            ["poison.gitconfig"]
        ]


def _private_build(handover: Mapping[str, str]) -> None:
    adopt_everywhere(handover)
    _append(
        Path(handover[FIXTURE_LOG_VARIABLE]),
        {
            "event": "private-build",
            "fixtures": "matrix",
            "test": "vcs-adapters::classify sneaky",
            "hermetic-root": "/tmp/vcs-workbench-x",
        },
    )


def _unlisted_consumer(handover: Mapping[str, str]) -> None:
    adopt_everywhere(handover)
    _append(
        Path(handover[FIXTURE_LOG_VARIABLE]),
        {
            "event": "adoption",
            "family": "matrix",
            "test": "vcs-cli::status a_test",
            "hermetic-root": "/w",
        },
    )


class TestProblemsInTheLog:
    @pytest.mark.parametrize(
        ("body", "problem"),
        [
            (_private_build, "vcs-adapters::classify sneaky built the matrix"),
            (_unlisted_consumer, "vcs-cli::status uses the matrix"),
            (lambda handover: None, "is listed as a matrix consumer"),
        ],
        ids=["private build", "unlisted consumer", "silent consumer"],
    )
    def test_each_fails_the_run(self, harness: Harness, body, problem: str):
        with pytest.raises(SharedFixturesError, match=re.escape(problem)):
            harness.run(body)

    def test_a_build_the_log_does_not_record_fails_the_run(
        self, harness: Harness
    ):
        def build_without_logging(
            trees: Mapping[Family, Path], log: Path
        ) -> None:
            build_that_logs(trees, log.with_suffix(".elsewhere"))

        harness.build = build_without_logging

        with pytest.raises(
            SharedFixturesError, match="the log records no shared build"
        ):
            harness.run()
