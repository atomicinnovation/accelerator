import json
import os
from pathlib import Path

import pytest

FIXTURES = Path(__file__).parent / "fixtures"
DEAD_PID = "2147483632"
SENTINEL = "owner.0123456789abcdef"
STATE = ".accelerator/state/integrations"

_JIRA_CONFIG = (
    "---\nwork:\n  integration: jira\njira:\n  site: acme\n"
    "  email: fixture@example.com\n  project_key: ENG\n---\n"
)
_LINEAR_CONFIG = (
    "---\nwork:\n  integration: linear\nlinear:\n  team_id: team-x-uuid\n---\n"
)

_JIRA_ROUTES = {
    "/rest/api/3/project": [{"key": "ENG", "id": "10000", "name": "Eng"}],
    "/rest/api/3/field": [
        {
            "id": "customfield_10016",
            "name": "Story Points",
            "custom": True,
            "schema": {"type": "number"},
        }
    ],
}


def _linear_operations() -> dict[str, str]:
    scenario = json.loads(
        (FIXTURES / "linear/team-states-200.json").read_text()
    )
    return {
        query["operation"]: query["response"]["body"]
        for query in scenario["graphql_queries"]
    }


def _respond(method: str, path: str, body: bytes) -> tuple[int, str]:
    if method == "GET" and path.split("?", maxsplit=1)[0] in _JIRA_ROUTES:
        return 200, json.dumps(_JIRA_ROUTES[path.split("?", maxsplit=1)[0]])
    if method == "POST" and path == "/graphql":
        text = body.decode()
        for operation, response in _linear_operations().items():
            if operation in text:
                return 200, response
    return 404, ""


@pytest.fixture
def tracker_api(loopback):
    loopback.serve(_respond)
    yield loopback
    loopback.refuse_everything()


_TRACKERS = {
    "jira": (
        _JIRA_CONFIG,
        ("init", "discover"),
        {"ACCELERATOR_JIRA_TOKEN": "fixture-token"},
    ),
    "linear": (
        _LINEAR_CONFIG,
        ("init", "discover", "--team-id", "team-x-uuid"),
        {"ACCELERATOR_LINEAR_TOKEN": "fixture-token"},
    ),
}


@pytest.fixture(params=sorted(_TRACKERS))
def tracker(request, repository, tracker_api):
    config, discover, env = _TRACKERS[request.param]
    repository.write(".accelerator/config.md", config)
    repository.commit()
    return request.param, discover, env


def _state(repository, name):
    return repository.root / STATE / name


def _observe_state(result, repository, name):
    state = _state(repository, name)
    if not state.exists():
        return result.with_observation(f"{STATE}/{name} is absent")
    for path in sorted(state.rglob("*")):
        relative = path.relative_to(state)
        if path.is_dir():
            result = result.with_observation(f"{relative}/")
        else:
            result = result.with_observation(
                f"{relative}:\n{path.read_text().rstrip()}"
            )
    return result


def _discover(run, binaries, repository, tracker):
    name, discover, env = tracker
    return run(
        binaries.subbinary(name), *discover, cwd=repository.root, env=env
    )


def test_discovery_writes_the_cache(
    repository, binaries, run, matches_golden, tracker
):
    result = _discover(run, binaries, repository, tracker)

    matches_golden(_observe_state(result, repository, tracker[0]))


def test_discovery_over_an_existing_cache(
    repository, binaries, run, matches_golden, tracker
):
    _discover(run, binaries, repository, tracker)

    result = _discover(run, binaries, repository, tracker)

    matches_golden(_observe_state(result, repository, tracker[0]))


def test_discovery_reclaims_a_lock_left_by_a_dead_process(
    repository, binaries, run, matches_golden, tracker
):
    lock = _state(repository, tracker[0]) / ".lock"
    lock.mkdir(parents=True)
    (lock / SENTINEL).write_text(DEAD_PID)

    result = _discover(run, binaries, repository, tracker)

    matches_golden(_observe_state(result, repository, tracker[0]))


@pytest.mark.skipif(os.geteuid() == 0, reason="root ignores permission bits")
def test_discovery_under_a_read_only_state_directory(
    repository, binaries, run, matches_golden, tracker
):
    state = _state(repository, tracker[0])
    state.mkdir(parents=True)
    state.chmod(0o555)
    try:
        result = _discover(run, binaries, repository, tracker)
    finally:
        state.chmod(0o755)

    matches_golden(_observe_state(result, repository, tracker[0]))


def test_jira_reads_the_fields_cache(
    repository, binaries, run, matches_golden, tracker_api
):
    repository.write(".accelerator/config.md", _JIRA_CONFIG)
    repository.write(
        f"{STATE}/jira/fields.json",
        json.dumps({"fields": _JIRA_ROUTES["/rest/api/3/field"]}) + "\n",
    )
    repository.commit()

    matches_golden(
        run(
            binaries.subbinary("jira"),
            "fields",
            "list",
            cwd=repository.root,
            env={"ACCELERATOR_JIRA_TOKEN": "fixture-token"},
        )
    )
