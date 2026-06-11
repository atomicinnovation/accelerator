from pathlib import Path

from tasks.shared.dev.state import DevState, read_dev_state, write_dev_state


def _full_state(**overrides) -> DevState:
    base = {
        "endpoint": "ipc:///tmp/acc-dev-abc/e.sock",
        "pubsub_endpoint": "ipc:///tmp/acc-dev-abc/p.sock",
        "frontend_port": 54321,
        "frontend_url": "http://127.0.0.1:54321",
        "pidfile": "/dev/dir/circusd.pid",
        "ini_path": "/dev/dir/circus.ini",
        "arbiter_pid": 4242,
        "arbiter_start_time": 1000.5,
        "server_pid": 4243,
        "server_start_time": 1001.5,
        "frontend_pid": 4244,
        "frontend_start_time": 1002.5,
        "npm_bin": "/usr/local/bin/npm",
        "node_bin": "/usr/local/bin/node",
    }
    base.update(overrides)
    return DevState(**base)


class TestDevStateRoundTrip:
    def test_full_round_trip(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        state = _full_state()
        write_dev_state(path, state)
        assert read_dev_state(path) == state

    def test_provisional_state_null_pids_round_trip(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        state = DevState(
            endpoint="ipc:///tmp/e.sock",
            pubsub_endpoint="ipc:///tmp/p.sock",
            frontend_port=12345,
            frontend_url="http://127.0.0.1:12345",
            pidfile="/d/circusd.pid",
            ini_path="/d/circus.ini",
        )
        write_dev_state(path, state)
        loaded = read_dev_state(path)
        assert loaded == state
        assert loaded.arbiter_pid is None
        assert loaded.server_pid is None
        assert loaded.frontend_pid is None

    def test_incremental_pid_write(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        state = _full_state(
            arbiter_pid=None, server_pid=None, frontend_pid=None
        )
        write_dev_state(path, state)
        # server becomes active -> record only the server PID
        state.server_pid = 555
        state.server_start_time = 9.0
        write_dev_state(path, state)
        loaded = read_dev_state(path)
        assert loaded.server_pid == 555
        assert loaded.frontend_pid is None

    def test_missing_file_returns_none(self, tmp_path: Path):
        assert read_dev_state(tmp_path / "absent.json") is None

    def test_malformed_json_returns_none(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        path.write_text("{not json")
        assert read_dev_state(path) is None

    def test_non_object_json_returns_none(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        path.write_text("[1, 2, 3]")
        assert read_dev_state(path) is None

    def test_schema_mismatch_missing_field_returns_none(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        path.write_text('{"endpoint": "ipc:///x", "frontend_port": 1}')
        assert read_dev_state(path) is None

    def test_schema_mismatch_wrong_type_returns_none(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        import json

        bad = {
            "endpoint": "ipc:///e",
            "pubsub_endpoint": "ipc:///p",
            "frontend_port": "not-an-int",
            "frontend_url": "http://x",
            "pidfile": "/p",
            "ini_path": "/i",
        }
        path.write_text(json.dumps(bad))
        assert read_dev_state(path) is None

    def test_boolean_port_rejected(self, tmp_path: Path):
        path = tmp_path / "dev.json"
        import json

        bad = {
            "endpoint": "ipc:///e",
            "pubsub_endpoint": "ipc:///p",
            "frontend_port": True,  # bool is an int subclass; must be rejected
            "frontend_url": "http://x",
            "pidfile": "/p",
            "ini_path": "/i",
        }
        path.write_text(json.dumps(bad))
        assert read_dev_state(path) is None
