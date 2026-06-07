"""The per-workspace dev discovery-cache (``dev.json``) and its atomic I/O."""

import dataclasses
import json
from pathlib import Path

from tasks.shared.files import atomic_write_text


@dataclasses.dataclass
class DevState:
    """Per-workspace discovery cache written under ``.accelerator/tmp/dev/``.

    The ``endpoint``/``pubsub_endpoint`` are full ``ipc://<path>`` strings
    (formatted once at write time so callers never hand-build them). The arbiter
    and watcher PIDs are ``None`` in the provisional pre-launch state and filled
    incrementally as each becomes live, so teardown can reach reparented orphans
    by identity even after the arbiter dies. ``npm_bin``/``node_bin`` are
    recorded for diagnosability.
    """

    endpoint: str
    pubsub_endpoint: str
    frontend_port: int
    frontend_url: str
    pidfile: str
    ini_path: str
    arbiter_pid: int | None = None
    arbiter_start_time: float | None = None
    server_pid: int | None = None
    server_start_time: float | None = None
    frontend_pid: int | None = None
    frontend_start_time: float | None = None
    npm_bin: str | None = None
    node_bin: str | None = None


_REQUIRED_STR_FIELDS = ("endpoint", "pubsub_endpoint", "frontend_url", "pidfile", "ini_path")
_OPTIONAL_INT_FIELDS = ("arbiter_pid", "server_pid", "frontend_pid")
_OPTIONAL_FLOAT_FIELDS = ("arbiter_start_time", "server_start_time", "frontend_start_time")
_OPTIONAL_STR_FIELDS = ("npm_bin", "node_bin")


def _is_int(value: object) -> bool:
    # bool is an int subclass; reject it so a JSON ``true`` never reads as a PID.
    return isinstance(value, int) and not isinstance(value, bool)


def _devstate_from_dict(raw: dict) -> DevState:
    kwargs: dict[str, object] = {}
    for key in _REQUIRED_STR_FIELDS:
        value = raw[key]  # KeyError -> schema mismatch
        if not isinstance(value, str):
            raise TypeError(key)
        kwargs[key] = value
    port = raw["frontend_port"]
    if not _is_int(port):
        raise TypeError("frontend_port")
    kwargs["frontend_port"] = port
    for key in _OPTIONAL_INT_FIELDS:
        value = raw.get(key)
        if value is not None and not _is_int(value):
            raise TypeError(key)
        kwargs[key] = value
    for key in _OPTIONAL_FLOAT_FIELDS:
        value = raw.get(key)
        if value is not None and not isinstance(value, int | float):
            raise TypeError(key)
        kwargs[key] = float(value) if value is not None else None
    for key in _OPTIONAL_STR_FIELDS:
        value = raw.get(key)
        if value is not None and not isinstance(value, str):
            raise TypeError(key)
        kwargs[key] = value
    return DevState(**kwargs)


def write_dev_state(path: Path | str, state: DevState) -> None:
    """Atomically write dev-state as JSON."""
    atomic_write_text(Path(path), json.dumps(dataclasses.asdict(state), indent=2))


def read_dev_state(path: Path | str) -> DevState | None:
    """Read dev-state, returning ``None`` on any unusable file.

    A single simple contract for every caller: ``None`` if the file is absent,
    unparseable, not an object, or structurally present but schema-mismatched
    (missing or wrong-typed fields).
    """
    try:
        raw = json.loads(Path(path).read_text())
    except (OSError, ValueError):
        return None
    if not isinstance(raw, dict):
        return None
    try:
        return _devstate_from_dict(raw)
    except (KeyError, TypeError, ValueError):
        return None
