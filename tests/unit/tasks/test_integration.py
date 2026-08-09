"""Tests for the suite-count guards in ``tasks/test/integration.py``.

Every guarded task asserts an at-least floor on the number of discovered shell
suites, and some additionally require named gates, so a dropped exec bit (which
makes a fail-closed gate silently vanish from CI) turns the build red instead of
shrinking the regression net. These tests cover all three halves: that
``run_shell_suites`` discovery actually shrinks when an exec bit is dropped,
that the guard fires below the baseline, and that a named gate cannot be lost
while the count floor still passes on filler.
"""

import pytest
from invoke import Context, Exit

from tasks.test import helpers, integration


@pytest.fixture(autouse=True)
def _stub_accelerator_env(mocker):
    """The guard tasks call ``accelerator_env`` (the ACCELERATOR_BIN overlay for
    the launcher built by the ``build:cli:dev`` mise dependency) and pass it to
    the suites; stub it to an empty overlay so the guard logic is tested without
    touching a real launcher path."""
    mocker.patch.object(integration, "accelerator_env", return_value={})
    # `hooks` shells out to pytest before its floor check, and invoke's @task
    # rejects a stand-in Context, so the real one has `run` stubbed instead —
    # otherwise these unit tests would run the whole hooks suite twice.
    mocker.patch.object(Context, "run")


class _FakeContext:
    """Records run() invocations without executing anything."""

    def __init__(self):
        self.ran = []

    def run(self, cmd, *args, **kwargs):
        self.ran.append(cmd)


class TestRunShellSuitesExecBit:
    def test_dropping_exec_bit_shrinks_discovery(self, tmp_path, monkeypatch):
        monkeypatch.setattr(helpers, "repo_root", lambda: tmp_path)
        sub = tmp_path / "scripts"
        sub.mkdir()
        for name in ("test-a.sh", "test-b.sh"):
            p = sub / name
            p.write_text("#!/usr/bin/env bash\n")
            p.chmod(0o755)

        ctx = _FakeContext()
        discovered = helpers.run_shell_suites(ctx, "scripts")
        assert len(discovered) == 2

        (sub / "test-b.sh").chmod(0o644)  # drop the exec bit
        reduced = helpers.run_shell_suites(ctx, "scripts")
        assert len(reduced) == 1, "exec-bit drop must shrink the discovered set"


# (task name, floor constant, required-by-name tuple). Every guarded task
# appears here, so a new floor without a paired case is a visible omission
# rather than an untested Exit branch.
_GUARDED = [
    ("config", "_EXPECTED_CONFIG_SUITES", "_REQUIRED_CONFIG_SUITES"),
    ("hooks", "_EXPECTED_HOOKS_SUITES", None),
    ("decisions", "_EXPECTED_DECISIONS_SUITES", None),
    ("github", "_EXPECTED_GITHUB_SUITES", None),
    ("work", "_EXPECTED_WORK_SUITES", None),
    ("integrations", "_EXPECTED_INTEGRATIONS_SUITES", None),
]


def _at_baseline(floor_name: str, required_name: str | None) -> list[str]:
    """A discovered set exactly at the floor, including any by-name gates."""
    required = (
        list(getattr(integration, required_name)) if required_name else []
    )
    floor = getattr(integration, floor_name)
    filler = [f"x/test-{i}.sh" for i in range(floor - len(required))]
    return required + filler


@pytest.mark.parametrize(
    ("name", "floor_name", "required_name"),
    [g for g in _GUARDED if getattr(integration, g[1]) > 0],
)
def test_guard_fires_below_baseline(mocker, name, floor_name, required_name):
    below = _at_baseline(floor_name, required_name)[:-1]
    mocker.patch.object(integration, "run_shell_suites", return_value=below)
    with pytest.raises(Exit):
        getattr(integration, name)(Context())


@pytest.mark.parametrize(("name", "floor_name", "required_name"), _GUARDED)
def test_guard_passes_at_baseline(mocker, name, floor_name, required_name):
    suites = _at_baseline(floor_name, required_name)
    mocker.patch.object(integration, "run_shell_suites", return_value=suites)
    getattr(integration, name)(Context())  # must not raise


@pytest.mark.parametrize(
    ("name", "floor_name", "required_name"),
    [g for g in _GUARDED if g[2] is not None],
)
def test_guard_fires_when_a_named_gate_is_missing(
    mocker, name, floor_name, required_name
):
    # At the count floor but with the by-name gates swapped for filler: the
    # count alone cannot detect a renamed or exec-bit-stripped gate.
    floor = getattr(integration, floor_name)
    suites = [f"x/test-{i}.sh" for i in range(floor)]
    mocker.patch.object(integration, "run_shell_suites", return_value=suites)
    with pytest.raises(Exit):
        getattr(integration, name)(Context())
