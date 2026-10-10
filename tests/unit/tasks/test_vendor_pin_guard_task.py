import datetime as dt

import pytest
from invoke import Context, Exit

from tasks.shared.vendor.gpg import KeyListingError, list_keys
from tasks.shared.vendor.pin_guard import wiring
from tasks.shared.vendor.pin_guard.github_issues import GhIssueTracker
from tasks.shared.vendor.pin_guard.guard import evaluate
from tasks.shared.vendor.pin_guard.issues import (
    MAXIMUM_NEW_ISSUES,
    DryRunIssueTracker,
    IssuePolicy,
)
from tasks.shared.vendor.pin_guard.local_inputs import (
    LocalInputPaths,
    read_local_inputs,
)
from tasks.shared.vendor.pin_guard.markers import MarkerKind
from tasks.vendor import commands
from tasks.vendor.commands import (
    GUARD_ISSUE_AUTHOR,
    guard_pins,
    guard_pins_with,
)
from tests.unit.tasks.shared.doubles import (
    BUMP_SUBJECTS,
    FakeIssueTracker,
    clear_repository,
    fake_ports,
    listing_lister,
)

TODAY = dt.date(2026, 10, 10)
POLICY = IssuePolicy(GUARD_ISSUE_AUTHOR)
STALE = {subject: TODAY - dt.timedelta(days=400) for subject in BUMP_SUBJECTS}


def test_the_default_fake_world_is_clear(tmp_path):
    inputs = read_local_inputs(clear_repository(tmp_path, TODAY), TODAY)
    assert evaluate(inputs, TODAY, fake_ports()).findings == ()


def test_a_local_input_error_exits_before_any_tracker_call(tmp_path):
    paths = clear_repository(
        tmp_path, TODAY, bumped=STALE, owner_line="Owner: Jane Doe"
    )
    ports = fake_ports()
    with pytest.raises(Exit, match="Owner: line") as raised:
        guard_pins_with(paths, TODAY, ports, POLICY)
    assert raised.value.code == 1
    assert ports.tracker.calls == []


def test_an_unlistable_keyring_exits_before_any_tracker_call(tmp_path):
    def unlistable(_keyring):
        raise KeyListingError("gpg is not installed")

    ports = fake_ports(key_lister=unlistable)
    paths = clear_repository(tmp_path, TODAY, bumped=STALE)
    with pytest.raises(Exit) as raised:
        guard_pins_with(paths, TODAY, ports, POLICY)
    assert raised.value.code == 1
    assert "keys/nodejs-release.asc" in str(raised.value.message)
    assert "gpg is not installed" in str(raised.value.message)
    assert ports.tracker.calls == []


def test_key_expiry_findings_open_beside_the_age_findings(tmp_path):
    ports = fake_ports(key_lister=listing_lister("nodejs-release.colons"))
    paths = clear_repository(tmp_path, TODAY, bumped=STALE)
    guard_pins_with(paths, TODAY, ports, POLICY)
    kinds = [m.parts[0] for m in ports.tracker.opened_markers]
    assert kinds.count("key-expiry") == 3
    assert kinds.count("pin-age") == 3


def test_a_clear_run_returns_and_opens_nothing(tmp_path):
    ports = fake_ports()
    guard_pins_with(clear_repository(tmp_path, TODAY), TODAY, ports, POLICY)
    assert ports.tracker.opened == []


def test_findings_open_their_issues_and_return(tmp_path):
    ports = fake_ports()
    paths = clear_repository(tmp_path, TODAY, bumped=STALE)
    guard_pins_with(paths, TODAY, ports, POLICY)
    assert [m.parts[0] for m in ports.tracker.opened_markers] == [
        "keyring-age",
        "pin-age",
        "pin-age",
        "pin-age",
    ]


def test_a_failed_draft_exits_naming_it_after_the_rest_open(tmp_path):
    paths = clear_repository(tmp_path, TODAY, bumped=STALE)
    inputs = read_local_inputs(paths, TODAY)
    failing = evaluate(inputs, TODAY, fake_ports()).findings[0]
    ports = fake_ports(tracker=FakeIssueTracker(failing={failing.marker}))
    with pytest.raises(Exit) as raised:
        guard_pins_with(paths, TODAY, ports, POLICY)
    assert raised.value.code == 1
    assert failing.draft(inputs.owner).title in str(raised.value.message)
    assert len(ports.tracker.opened) == 3


def test_a_tripped_cap_and_a_failed_draft_exit_once_naming_both(tmp_path):
    paths = clear_repository(tmp_path, TODAY, bumped=STALE)
    probe = fake_ports()
    with pytest.raises(Exit):
        guard_pins_with(paths, TODAY, probe, IssuePolicy(GUARD_ISSUE_AUTHOR, 3))
    [tripped] = probe.tracker.opened_markers
    assert tripped.kind is MarkerKind.GUARD_TRIPPED

    ports = fake_ports(tracker=FakeIssueTracker(failing={tripped}))
    with pytest.raises(Exit) as raised:
        guard_pins_with(paths, TODAY, ports, IssuePolicy(GUARD_ISSUE_AUTHOR, 3))
    message = str(raised.value.message)
    assert "4 new findings" in message
    assert "could not open" in message


class TestTaskBoundary:
    @pytest.fixture
    def nothing_read(self, monkeypatch):
        def refuse():
            raise AssertionError("an input was read")

        monkeypatch.setattr(LocalInputPaths, "repository", refuse)

    def test_today_with_open_issues_is_refused(self, nothing_read):
        with pytest.raises(Exit, match="--today"):
            guard_pins(Context(), today="2026-10-10", open_issues=True)

    def test_a_cap_below_one_is_refused(self, nothing_read):
        with pytest.raises(Exit, match="--maximum-new-issues"):
            guard_pins(Context(), maximum_new_issues=0)

    def test_the_issue_author_and_cap_reach_the_policy(self, monkeypatch):
        seen = {}

        def capture(paths, today, ports, policy):
            seen["policy"] = policy
            seen["today"] = today

        monkeypatch.setattr(commands, "guard_pins_with", capture)
        guard_pins(
            Context(),
            today="2026-10-01",
            issue_author="someone",
            maximum_new_issues=25,
        )
        assert seen["policy"] == IssuePolicy("someone", 25)
        assert seen["today"] == dt.date(2026, 10, 1)

    def test_the_default_policy_trusts_the_workflow_bot(self, monkeypatch):
        seen = {}
        monkeypatch.setattr(
            commands,
            "guard_pins_with",
            lambda *args: seen.setdefault("policy", args[3]),
        )
        guard_pins(Context())
        assert seen["policy"] == IssuePolicy(
            "github-actions[bot]", MAXIMUM_NEW_ISSUES
        )


def test_production_keys_are_listed_by_gpg():
    assert wiring.real_ports(open_issues=False).key_lister is list_keys


def test_a_dry_run_wraps_the_gh_tracker():
    assert isinstance(
        wiring.real_ports(open_issues=False).tracker, DryRunIssueTracker
    )


def test_open_issues_writes_through_the_gh_tracker():
    assert isinstance(
        wiring.real_ports(open_issues=True).tracker, GhIssueTracker
    )
