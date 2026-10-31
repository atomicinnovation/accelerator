"""The one record of a running build's process group, for a later session.

A session that dies without unwinding leaves the file behind, naming a
build that may still be running.
"""

from dataclasses import dataclass
from pathlib import Path

from tasks.shared.build_measurement.locations import BUILD_PGID
from tasks.shared.build_measurement.session import OrphanedBuild
from tasks.shared.files import atomic_write_text
from tasks.shared.processes import ProcessOps

_FIELDS = 2
_FIRST_ORDINARY_PID = 2


@dataclass(frozen=True, slots=True)
class BuildGroupFile:
    root: Path
    procs: ProcessOps

    @property
    def path(self) -> Path:
        return self.root / BUILD_PGID

    def remember(self, pgid: int, create_time: float) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        atomic_write_text(self.path, f"{pgid} {create_time!r}\n")

    def forget(self) -> None:
        self.path.unlink(missing_ok=True)

    def read(self) -> tuple[int, float] | None:
        try:
            fields = self.path.read_text().split()
        except FileNotFoundError:
            return None
        if len(fields) != _FIELDS or not fields[0].isdecimal():
            return None
        try:
            return int(fields[0]), float(fields[1])
        except ValueError:
            return None

    def orphaned(self) -> OrphanedBuild | None:
        if not self.path.exists():
            return None
        remembered = self.read()
        if remembered is None or not self._still_running(*remembered):
            self.forget()
            return None
        pgid, _ = remembered
        return OrphanedBuild(
            pgid=pgid,
            members=self.procs.group_members(pgid),
            path=self.path,
        )

    def _still_running(self, pgid: int, create_time: float) -> bool:
        if pgid < _FIRST_ORDINARY_PID or pgid == self.procs.own_group():
            return False
        if not self.procs.group_alive(pgid):
            return False
        if self.procs.create_time(pgid) is None:
            return True
        return self.procs.identity_matches(pgid, create_time)
