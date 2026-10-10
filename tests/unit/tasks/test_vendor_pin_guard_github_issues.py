import json
import subprocess

import pytest

from tasks.shared.vendor.pin_guard import github_issues
from tasks.shared.vendor.pin_guard.findings import IssueDraft
from tasks.shared.vendor.pin_guard.github_issues import GhIssueTracker, run_gh
from tasks.shared.vendor.pin_guard.issues import (
    ExistingIssue,
    IssueTrackerError,
)
from tasks.shared.vendor.pin_guard.markers import IssueMarker, MarkerKind

DRAFT = IssueDraft(
    IssueMarker(MarkerKind.FINDING, ("keyring-age", "2025-08-24")),
    "Keyring is old",
    "Body.\n\n<!-- finding: keyring-age 2025-08-24 -->",
)


class RecordingGh:
    def __init__(self, **outputs):
        self.outputs = outputs
        self.calls = []

    def __call__(self, argv, *, stdin=None):
        self.calls.append((argv, stdin))
        return self.outputs[argv[0].replace("-", "_") + "_" + argv[1]]


def test_issues_are_listed_by_paginated_api_without_pull_requests():
    lines = "\n".join(
        json.dumps(entry)
        for entry in [
            {
                "number": 7,
                "state": "open",
                "author": "github-actions[bot]",
                "body": "Body.\n<!-- finding: x -->",
            },
            {"number": 8, "state": "closed", "author": "someone", "body": ""},
        ]
    )
    gh = RecordingGh(**{"api_--paginate": lines + "\n"})
    issues = GhIssueTracker(gh).issues()
    assert issues == [
        ExistingIssue(
            number=7,
            is_open=True,
            author="github-actions[bot]",
            body="Body.\n<!-- finding: x -->",
        ),
        ExistingIssue(number=8, is_open=False, author="someone", body=""),
    ]
    [(argv, stdin)] = gh.calls
    assert argv == [
        "api",
        "--paginate",
        "repos/{owner}/{repo}/issues"
        "?labels=runtime-pin-guard&state=all&per_page=100",
        "--jq",
        ".[] | select(.pull_request == null) | "
        '{number, state, author: .user.login, body: (.body // "")}',
    ]
    assert stdin is None


def test_an_unparseable_listing_line_is_a_tracker_error():
    gh = RecordingGh(**{"api_--paginate": "not json\n"})
    with pytest.raises(IssueTrackerError, match="not json"):
        GhIssueTracker(gh).issues()


def test_an_existing_label_is_not_created_again():
    gh = RecordingGh(label_list=json.dumps([{"name": "runtime-pin-guard"}]))
    GhIssueTracker(gh).ensure_label()
    assert [argv for argv, _ in gh.calls] == [
        ["label", "list", "--json", "name", "--limit", "1000"]
    ]


def test_a_missing_label_is_created():
    gh = RecordingGh(label_list=json.dumps([{"name": "bug"}]), label_create="")
    GhIssueTracker(gh).ensure_label()
    created = gh.calls[1][0]
    assert created[:3] == ["label", "create", "runtime-pin-guard"]
    assert "--description" in created
    assert "--color" in created


def test_an_issue_is_created_with_label_assignee_and_body_on_stdin():
    gh = RecordingGh(
        issue_create="https://github.com/atomicinnovation/accelerator/issues/42\n"
    )
    number = GhIssueTracker(gh).open_issue(DRAFT, "tobyclemson")
    assert number == 42
    [(argv, stdin)] = gh.calls
    assert argv == [
        "issue",
        "create",
        "--title",
        "Keyring is old",
        "--body-file",
        "-",
        "--label",
        "runtime-pin-guard",
        "--assignee",
        "tobyclemson",
    ]
    assert stdin == DRAFT.body


def test_create_output_without_an_issue_url_is_a_tracker_error():
    gh = RecordingGh(issue_create="something else\n")
    with pytest.raises(IssueTrackerError, match="something else"):
        GhIssueTracker(gh).open_issue(DRAFT, "tobyclemson")


def test_a_non_zero_exit_carries_the_subcommand_and_stderr(monkeypatch):
    def failing(*args, **kwargs):
        raise subprocess.CalledProcessError(
            1, ["gh", "issue", "create"], stderr="  could not assign  \n"
        )

    monkeypatch.setattr(github_issues.subprocess, "run", failing)
    with pytest.raises(
        IssueTrackerError, match=r"gh issue create: could not assign$"
    ):
        run_gh(["issue", "create"])


def test_a_timeout_carries_the_subcommand_and_stderr(monkeypatch):
    def slow(*args, **kwargs):
        raise subprocess.TimeoutExpired(
            ["gh", "label", "list"], 30, stderr="stalled"
        )

    monkeypatch.setattr(github_issues.subprocess, "run", slow)
    with pytest.raises(IssueTrackerError, match=r"gh label list: .*stalled"):
        run_gh(["label", "list"])


def test_a_missing_gh_is_a_tracker_error(monkeypatch):
    def absent(*args, **kwargs):
        raise FileNotFoundError("gh")

    monkeypatch.setattr(github_issues.subprocess, "run", absent)
    with pytest.raises(IssueTrackerError, match="gh api"):
        run_gh(["api", "x"])


def test_run_gh_passes_stdin_and_returns_stdout(monkeypatch):
    seen = {}

    def fake_run(argv, **kwargs):
        seen["argv"] = argv
        seen.update(kwargs)
        return subprocess.CompletedProcess(argv, 0, stdout="out", stderr="")

    monkeypatch.setattr(github_issues.subprocess, "run", fake_run)
    assert run_gh(["issue", "create"], stdin="body") == "out"
    assert seen["argv"] == ["gh", "issue", "create"]
    assert seen["input"] == "body"
    assert seen["check"] is True
    assert seen["timeout"] == 30


def test_an_unreadable_label_listing_is_a_tracker_error():
    gh = RecordingGh(label_list="<html>")
    with pytest.raises(IssueTrackerError, match="html"):
        GhIssueTracker(gh).ensure_label()
