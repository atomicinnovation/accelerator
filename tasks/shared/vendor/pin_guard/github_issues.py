"""The ``gh`` adapter for the guard's issue tracker."""

import json
import re
import subprocess
from typing import Protocol

from tasks.shared.vendor.pin_guard.findings import IssueDraft
from tasks.shared.vendor.pin_guard.issues import (
    LABEL,
    ExistingIssue,
    IssueTrackerError,
)

GH_TIMEOUT_SECONDS = 30
LABEL_DESCRIPTION = "Opened by the scheduled vendored-runtime pin guard"
LABEL_COLOUR = "B60205"
# `gh issue list` truncates at --limit; the paginated REST listing does not,
# but it also returns pull requests and a null body for an empty one.
_ISSUES_ENDPOINT = (
    f"repos/{{owner}}/{{repo}}/issues?labels={LABEL}&state=all&per_page=100"
)
_ISSUE_FIELDS = (
    ".[] | select(.pull_request == null) | "
    '{number, state, author: .user.login, body: (.body // "")}'
)
_ISSUE_URL = re.compile(r"/issues/(\d+)\s*$")


class GhRunner(Protocol):
    def __call__(self, argv: list[str], *, stdin: str | None = None) -> str: ...


def run_gh(argv: list[str], *, stdin: str | None = None) -> str:
    """Run ``gh <argv>``, raising ``IssueTrackerError`` on any failure."""
    subcommand = " ".join(["gh", *argv[:2]])
    try:
        completed = subprocess.run(
            ["gh", *argv],
            input=stdin,
            check=True,
            capture_output=True,
            text=True,
            timeout=GH_TIMEOUT_SECONDS,
        )
    except subprocess.CalledProcessError as error:
        raise IssueTrackerError(
            f"{subcommand}: {(error.stderr or '').strip()}"
        ) from error
    except subprocess.TimeoutExpired as error:
        stderr = error.stderr or ""
        if isinstance(stderr, bytes):
            stderr = stderr.decode(errors="replace")
        raise IssueTrackerError(
            f"{subcommand}: timed out after {GH_TIMEOUT_SECONDS}s "
            f"{stderr.strip()}"
        ) from error
    except OSError as error:
        raise IssueTrackerError(f"{subcommand}: {error}") from error
    return completed.stdout


class GhIssueTracker:
    def __init__(self, run: GhRunner = run_gh) -> None:
        self._run = run

    def issues(self) -> list[ExistingIssue]:
        listing = self._run(
            ["api", "--paginate", _ISSUES_ENDPOINT, "--jq", _ISSUE_FIELDS]
        )
        return [_existing_issue(line) for line in listing.splitlines() if line]

    def ensure_label(self) -> None:
        listing = self._run(
            ["label", "list", "--json", "name", "--limit", "1000"]
        )
        if LABEL in _label_names(listing):
            return
        self._run(
            [
                "label",
                "create",
                LABEL,
                "--description",
                LABEL_DESCRIPTION,
                "--color",
                LABEL_COLOUR,
            ]
        )

    def open_issue(self, draft: IssueDraft, assignee: str) -> int:
        output = self._run(
            [
                "issue",
                "create",
                "--title",
                draft.title,
                "--body-file",
                "-",
                "--label",
                LABEL,
                "--assignee",
                assignee,
            ],
            stdin=draft.body,
        )
        match = _ISSUE_URL.search(output)
        if match is None:
            raise IssueTrackerError(
                f"gh issue create printed no issue URL: {output.strip()}"
            )
        return int(match[1])


def _existing_issue(line: str) -> ExistingIssue:
    try:
        entry = json.loads(line)
        return ExistingIssue(
            number=int(entry["number"]),
            is_open=entry["state"] == "open",
            author=str(entry["author"]),
            body=str(entry["body"]),
        )
    except (ValueError, KeyError, TypeError) as error:
        raise IssueTrackerError(
            f"gh api returned an unreadable issue: {line}"
        ) from error


def _label_names(listing: str) -> set[str]:
    try:
        return {str(label["name"]) for label in json.loads(listing)}
    except (ValueError, KeyError, TypeError) as error:
        raise IssueTrackerError(
            f"gh label list returned an unreadable listing: {listing}"
        ) from error
