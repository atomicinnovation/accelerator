import pytest

from tests.integration.support.tracker_blocks import (
    block_case_id,
    block_cases,
)

_SETTINGS = {
    "jira": "  site: acme\n  email: fixture@example.com\n",
    "linear": "",
}

_PROBES = {
    "jira": ("fields", "list"),
    "linear": ("show", "X"),
}

_TOKENS = {
    "ACCELERATOR_JIRA_TOKEN": "fixture-token",
    "ACCELERATOR_LINEAR_TOKEN": "fixture-token",
}


@pytest.fixture
def configured(repository, request):
    tracker, _, blocks = request.param
    repository.write(
        ".accelerator/config.md",
        blocks.frontmatter(tracker, _SETTINGS[tracker]),
    )
    repository.write("meta/work/.keep", "")
    repository.commit()
    return tracker, repository


_CASES = pytest.mark.parametrize(
    "configured", block_cases(), ids=block_case_id, indirect=True
)


@_CASES
def test_a_tracker_binary_parses_the_block(
    configured, binaries, run, matches_golden
):
    tracker, repository = configured

    matches_golden(
        run(
            binaries.subbinary(tracker),
            *_PROBES[tracker],
            cwd=repository.root,
            env=_TOKENS,
        )
    )


@_CASES
def test_work_sync_preview_parses_the_block(
    configured, binaries, run, matches_golden
):
    _, repository = configured

    matches_golden(
        run(
            binaries.subbinary("work"),
            "sync",
            "--preview",
            "--push-only",
            cwd=repository.root,
            env=_TOKENS,
        )
    )
