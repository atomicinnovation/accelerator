"""Fetch failures found in a run's stamped log.

``advisory-fetch-failure.log`` holds what cargo-deny 0.19.8 printed for
``mise run --output prefix --skip-deps deny:check`` with outbound network
denied (``sandbox-exec -p '(version 1)(allow default)(deny network-outbound
(remote ip))'``) and a scratch ``CARGO_HOME`` linking the real registry but
holding no advisory database; with a cached database, the failed fetch is
silent and the check passes. Only the ``[deny:check]`` lines were kept, each
prefixed with a hand-chosen stamp. Re-capture it the same way when
cargo-deny's wording changes.
"""

import pytest

from tasks.shared.build_measurement.fetch_failures import (
    ExternalInput,
    FetchFailure,
    fetch_failures,
)
from tasks.shared.build_measurement.timeline import StampedLine, read_stamped
from tasks.shared.sources import repo_root

_ADVISORY_FETCH_FAILURE = (
    repo_root()
    / "tests/unit/tasks/fixtures/build-measurement/advisory-fetch-failure.log"
)


def _stamped(*texts: str) -> tuple[StampedLine, ...]:
    return tuple(
        StampedLine(at=float(at), text=text) for at, text in enumerate(texts)
    )


def _inputs(lines: tuple[StampedLine, ...]) -> list[ExternalInput]:
    return [failure.external_input for failure in fetch_failures(lines, run=1)]


def test_a_curl_error_is_a_launcher_release_failure():
    lines = _stamped(
        "[test:integration:hooks] curl: (6) Could not resolve host: github.com"
    )

    assert _inputs(lines) == [ExternalInput.LAUNCHER_RELEASE]


def test_a_failed_advisory_fetch_is_an_advisory_database_failure():
    lines = read_stamped(_ADVISORY_FETCH_FAILURE.read_text(newline=""))

    assert _inputs(lines) == [ExternalInput.ADVISORY_DATABASE]


@pytest.mark.parametrize(
    "code", ["ETIMEDOUT", "ECONNRESET", "ENOTFOUND", "EAI_AGAIN"]
)
@pytest.mark.parametrize("marker", ["ERR!", "error"])
def test_each_npm_network_code_is_an_npm_registry_failure(code, marker):
    lines = _stamped(f"[deps:install:node] npm {marker} code {code}")

    assert _inputs(lines) == [ExternalInput.NPM_REGISTRY]


def test_a_network_code_outside_an_npm_error_is_not_a_failure():
    lines = _stamped(
        "[test:unit:tasks] tests/unit/tasks/shared/build_measurement/"
        "test_fetch_failures.py::test_each_npm_network_code"
        "[ETIMEDOUT] PASSED [  1%]"
    )

    assert fetch_failures(lines, run=1) == ()


def test_a_non_network_failure_leaves_the_list_empty():
    lines = _stamped(
        "[deny:check] error[vulnerability]: Denial of service in hickory-proto",
        "[deny:check]     ┌─ /repo/cli/Cargo.lock:120:1",
        "[deny:check] advisories FAILED, bans ok, licenses ok, sources ok",
        "[deps:install:node] npm ERR! code ERESOLVE",
        "[deps:install:node] npm ERR! ERESOLVE could not resolve",
    )

    assert fetch_failures(lines, run=1) == ()


def test_each_failure_names_its_task_and_line():
    lines = _stamped(
        "[deps:install:node] npm ERR! code ETIMEDOUT",
        "Finished in 2.00s",
        "[test:integration:hooks] curl: (6) Could not resolve host: github.com",
    )

    assert fetch_failures(lines, run=2) == (
        FetchFailure(
            run=2,
            task="deps:install:node",
            external_input=ExternalInput.NPM_REGISTRY,
            line="npm ERR! code ETIMEDOUT",
        ),
        FetchFailure(
            run=2,
            task="test:integration:hooks",
            external_input=ExternalInput.LAUNCHER_RELEASE,
            line="curl: (6) Could not resolve host: github.com",
        ),
    )
