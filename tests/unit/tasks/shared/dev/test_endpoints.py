from pathlib import Path

import pytest

from tasks.shared.dev import endpoints
from tasks.shared.dev.endpoints import ipc_socket_paths


class TestIpcSocketPaths:
    def test_endpoint_and_pubsub_are_distinct(self, tmp_path: Path):
        endpoint, pubsub = ipc_socket_paths(tmp_path)
        assert endpoint != pubsub

    def test_deterministic_for_a_given_root(self, tmp_path: Path):
        assert ipc_socket_paths(tmp_path) == ipc_socket_paths(tmp_path)

    def test_distinct_per_workspace_root(self, tmp_path: Path):
        a = ipc_socket_paths(tmp_path / "wsA")
        b = ipc_socket_paths(tmp_path / "wsB")
        assert a != b

    def test_within_sun_path_limit(self, tmp_path: Path):
        for path in ipc_socket_paths(tmp_path):
            assert len(str(path)) <= endpoints._SUN_PATH_MAX

    def test_filesystem_path_not_abstract_namespace(self, tmp_path: Path):
        for path in ipc_socket_paths(tmp_path):
            assert not str(path).startswith("@")
            assert not str(path).startswith("\0")

    def test_resolves_when_tmpdir_unset(self, tmp_path: Path, monkeypatch):
        # tempfile.gettempdir() falls back to /tmp when $TMPDIR is unset.
        monkeypatch.delenv("TMPDIR", raising=False)
        import tempfile

        tempfile.tempdir = None  # clear the cached value so the env is re-read
        try:
            endpoint, pubsub = ipc_socket_paths(tmp_path)
            assert endpoint.is_absolute() and pubsub.is_absolute()
        finally:
            tempfile.tempdir = None

    def test_hard_errors_when_path_exceeds_limit(
        self, tmp_path: Path, monkeypatch
    ):
        deep = "/" + "x" * endpoints._SUN_PATH_MAX
        monkeypatch.setattr(endpoints.tempfile, "gettempdir", lambda: deep)
        with pytest.raises(ValueError, match="sun_path"):
            ipc_socket_paths(tmp_path)
