import contextlib
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.build_measurement.errors import SessionInProgressError
from tasks.shared.build_measurement.locations import SESSION_LOCK
from tasks.shared.locking import workspace_lock


@dataclass(frozen=True, slots=True)
class WorkspaceSessionLock:
    root: Path

    @contextlib.contextmanager
    def held(self) -> Iterator[None]:
        path = self.root / SESSION_LOCK
        with workspace_lock(path) as acquired:
            if not acquired:
                raise SessionInProgressError(
                    "another measure:build session is running in this "
                    "workspace; wait for it to finish (its lock is released "
                    f"when it exits; do not delete {path})"
                )
            yield
