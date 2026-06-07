"""Per-workspace ``ipc://`` socket-path derivation for the dev arbiter."""

import hashlib
import tempfile
from pathlib import Path

# macOS ``sun_path`` is ~104 bytes and Linux ~108; budgeting against the
# smaller (macOS) limit keeps a path that passes here valid on both. The
# basenames ("e.sock"/"p.sock") are kept short and the per-workspace subdir is a
# fixed 12-hex digest, so the only variable is the temp base.
_SUN_PATH_MAX = 104


def ipc_socket_paths(workspace_root: Path | str) -> tuple[Path, Path]:
    """Return the (endpoint, pubsub) ``ipc://`` socket *paths* for a workspace.

    Deterministic for a given workspace root and distinct per root, so
    ``dev:stop``/``dev:status`` can recompute them when dev-state is missing or
    corrupt. The paths are filesystem-path sockets (no Linux-only abstract ``@``
    namespace) under ``<tempdir>/acc-dev-<12-hex-hash>/``, resolved via
    ``tempfile.gettempdir()`` so an unset ``$TMPDIR`` falls back to ``/tmp``.

    Hard-errors (never silently truncates — that would break per-workspace
    uniqueness) if either path would exceed the ``sun_path`` length budget.
    """
    canonical = str(Path(workspace_root).resolve())
    digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:12]
    base = Path(tempfile.gettempdir()) / f"acc-dev-{digest}"
    endpoint = base / "e.sock"
    pubsub = base / "p.sock"
    for label, path in (("endpoint", endpoint), ("pubsub", pubsub)):
        length = len(str(path))
        if length > _SUN_PATH_MAX:
            raise ValueError(
                f"dev {label} ipc socket path is {length} bytes, exceeding the "
                f"{_SUN_PATH_MAX}-byte sun_path limit: {path}. Set a shorter "
                f"$TMPDIR (the per-workspace hash and basenames are already "
                f"minimal)."
            )
    return endpoint, pubsub
