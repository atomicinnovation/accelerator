import json

import pytest

from tests.integration.support.characterisation import Repository

ORIGIN = "https://github.com/candidate-owner/candidate-repo.git"
REPOSITORY = "/repos/candidate-owner/candidate-repo"

_ROUTES = {
    REPOSITORY: {
        "id": 1,
        "name": "candidate-repo",
        "url": f"https://api.github.com{REPOSITORY}",
    },
    f"{REPOSITORY}/pulls/42": {
        "id": 1,
        "number": 42,
        "url": f"https://api.github.com{REPOSITORY}/pulls/42",
        "head": {"ref": "feature", "sha": "abc123"},
        "base": {"ref": "main", "sha": "def456"},
        "locked": False,
    },
}


def _respond(method, path, body):
    route = _ROUTES.get(path.partition("?")[0])
    return (200, json.dumps(route)) if method == "GET" and route else (404, "")


@pytest.fixture
def github(loopback):
    loopback.serve(_respond)
    yield loopback
    loopback.refuse_everything()


def _add_origin(repository, url):
    if repository.is_git:
        repository.git("remote", "add", "origin", url)
    else:
        repository.jj("git", "remote", "add", "origin", url)


@pytest.mark.parametrize(
    "origin",
    [
        ORIGIN,
        "git@github.com:candidate-owner/candidate-repo.git",
        None,
        "https://gitlab.example.com/owner/repo.git",
    ],
    ids=["https-origin", "ssh-origin", "no-origin", "foreign-origin"],
)
def test_base_repository_resolution(
    repository, binaries, run, matches_golden, github, origin
):
    repository.write("README.md", "fixture\n")
    repository.commit()
    if origin is not None:
        _add_origin(repository, origin)

    matches_golden(
        run(
            binaries.subbinary("collaboration"),
            "pr",
            "base-repo",
            "42",
            cwd=repository.root,
            env={"GH_TOKEN": "test-token"},
        )
    )


@pytest.mark.vcs_specific
def test_base_repository_in_a_colocated_jj_repository(
    hermetic, fixture_root, binaries, run, matches_golden, github
):
    repository = Repository.create(
        "colocated-jj", fixture_root / "colocated", hermetic
    )
    repository.write("README.md", "fixture\n")
    repository.commit()
    repository.jj("git", "remote", "add", "origin", ORIGIN)

    matches_golden(
        run(
            binaries.subbinary("collaboration"),
            "pr",
            "base-repo",
            "42",
            cwd=repository.root,
            env={"GH_TOKEN": "test-token"},
        )
    )
