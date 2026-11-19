"""A stand-in for the shared-fixtures builder and nextest in a cli task run."""

import json
import shlex
from collections.abc import Callable, Mapping
from pathlib import Path
from unittest.mock import MagicMock

from tasks.shared.shared_fixtures.consumers import consumers_of
from tasks.shared.shared_fixtures.families import (
    FIXTURE_LOG_VARIABLE,
    WORKBENCHES_VARIABLE,
    Family,
)

BUILDER = "/workspace/cli/target/debug/shared-fixtures"
CAPTURE = Path(__file__).parent / "fixtures/shared-fixtures"
CAPTURED_BINARIES = frozenset(
    {
        "vcs-adapters::classify",
        "vcs-adapters::queries",
        "vcs-adapters::scrub",
        "vcs-adapters::detection",
        "vcs-test-support::matrix",
    }
)


def _everything(_binary: str, _name: str) -> bool:
    return True


def captured_listing(matching: Callable[[str, str], bool]) -> str:
    """The captured nextest listing as if filtered to what ``matching`` admits.

    ``matching`` takes a binary id and a test name.
    """
    listing = json.loads((CAPTURE / "nextest-list.json").read_text())
    for binary, suite in listing["rust-suites"].items():
        for name, case in suite["testcases"].items():
            if case["filter-match"]["status"] == "matches" and not matching(
                binary, name
            ):
                case["filter-match"] = {
                    "status": "mismatch",
                    "reason": "expression",
                }
    return json.dumps(listing)


def builder_artefact(executable: str = BUILDER) -> str:
    return json.dumps(
        {
            "reason": "compiler-artifact",
            "target": {"kind": ["bin"], "name": "shared-fixtures"},
            "profile": {"test": False},
            "filenames": [executable],
            "executable": executable,
            "fresh": True,
        }
    )


def passed(stdout: str = "", stderr: str = "") -> MagicMock:
    return MagicMock(exited=0, stdout=stdout, stderr=stderr)


def failed(stderr: str = "") -> MagicMock:
    return MagicMock(exited=1, stdout="", stderr=stderr)


def _append(log: str, event: Mapping[str, str]) -> None:
    with Path(log).open("a") as file:
        file.write(json.dumps(event) + "\n")


class FakeSharedFixtures:
    """Answers the builder and nextest as a passing run would.

    The builder creates each tree it is asked for and logs its shared build;
    nextest logs one adoption by every consumer of each handed tree, and
    lists the captured tests. Every other command goes to ``respond``.
    """

    def __init__(
        self,
        respond: Callable[..., MagicMock] | None = None,
        *,
        builder: MagicMock | None = None,
        nextest: MagicMock | Callable[[], MagicMock] | None = None,
        listing: MagicMock | None = None,
    ) -> None:
        self._respond = respond or self._listing_the_builder
        self._builder = builder
        self._nextest = nextest
        self._listing = listing or passed(captured_listing(_everything))

    def __call__(self, command: str, **kwargs) -> MagicMock:
        environment = kwargs.get("env") or {}
        if command.startswith("cargo nextest list"):
            return self._listing
        if " --matrix " in command:
            return self._build(command, environment)
        if "nextest" in command:
            return self._test(environment)
        return self._respond(command, **kwargs)

    def _build(self, command: str, environment: Mapping[str, str]) -> MagicMock:
        if self._builder is not None:
            return self._builder
        words = shlex.split(command)
        for family in Family:
            flag = f"--{family.value}"
            if flag in words:
                tree = Path(words[words.index(flag) + 1])
                tree.mkdir(parents=True)
                (tree / "fixtures.tsv").write_text("PG-r\tplain git\t/x\n")
                _append(
                    environment[FIXTURE_LOG_VARIABLE],
                    {
                        "event": "shared-build",
                        "family": family.value,
                        "tree": str(tree),
                        "git": "git version 2.50.1",
                        "jj": "jj 0.43.0",
                    },
                )
        return passed()

    def _test(self, environment: Mapping[str, str]) -> MagicMock:
        for family in Family:
            if family.handover_variable not in environment:
                continue
            for binary in sorted(consumers_of(family)):
                _append(
                    environment[FIXTURE_LOG_VARIABLE],
                    {
                        "event": "adoption",
                        "family": family.value,
                        "binary": binary,
                        "test": "a_test",
                        "hermetic-root": (
                            f"{environment[WORKBENCHES_VARIABLE]}/w"
                        ),
                    },
                )
        if callable(self._nextest) and not isinstance(self._nextest, MagicMock):
            return self._nextest()
        return self._nextest or passed()

    @staticmethod
    def _listing_the_builder(command: str, **_kwargs) -> MagicMock:
        if command.startswith("cargo test"):
            return passed(builder_artefact())
        return passed()
