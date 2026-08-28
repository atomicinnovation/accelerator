"""Tests for the shell-suite discovery helper in ``tasks/test``.

``run_shell_suites`` discovers executable ``test-*.sh`` suites; the one
remaining property to pin is that dropping an exec bit shrinks the discovered
set (an exec-bit-lossy filesystem otherwise removes a suite from CI silently).
The suite-count floors it once fed are all retired.
"""

from tasks.test import helpers


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
