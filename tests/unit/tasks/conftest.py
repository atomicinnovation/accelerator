from pathlib import Path

import pytest

from tasks.test import cli as test_cli


@pytest.fixture
def shared_fixture_roots(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Keep a run of the cli task off the real shared root and logs.

    Also drops the fault-injection switches, so one left exported in a
    developer's shell never reaches a unit test.
    """
    monkeypatch.setattr(
        test_cli, "shared_fixtures_root", lambda: tmp_path / "shared-fixtures"
    )
    monkeypatch.setattr(
        test_cli, "_FIXTURE_LOGS", tmp_path / "vcs-fixture-logs"
    )
    monkeypatch.delenv("ACCELERATOR_SHARED_FIXTURES_HOLD", raising=False)
    monkeypatch.delenv("ACCELERATOR_SHARED_FIXTURES_FAIL", raising=False)
