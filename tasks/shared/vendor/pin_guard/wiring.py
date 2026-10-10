"""The one place the guard's production collaborators are wired."""

import sys

from tasks.shared.clock import Clock
from tasks.shared.vendor.gpg import list_keys
from tasks.shared.vendor.pin_guard.github_issues import GhIssueTracker
from tasks.shared.vendor.pin_guard.guard import GuardPorts
from tasks.shared.vendor.pin_guard.http_feeds import HttpFeedClient
from tasks.shared.vendor.pin_guard.issues import DryRunIssueTracker


def real_ports(*, open_issues: bool) -> GuardPorts:
    tracker = GhIssueTracker()
    return GuardPorts(
        tracker=tracker
        if open_issues
        else DryRunIssueTracker(tracker, sys.stdout),
        key_lister=list_keys,
        feeds=HttpFeedClient(Clock()),
        clock=Clock(),
    )
