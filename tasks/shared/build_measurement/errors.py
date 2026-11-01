import shlex
from collections.abc import Sequence
from pathlib import Path


class MeasurementRefusedError(RuntimeError):
    """The harness cannot measure; no record is written."""


class TaskGraphError(MeasurementRefusedError):
    pass


class NoMeasurableCommitError(MeasurementRefusedError):
    pass


class HistoryError(MeasurementRefusedError):
    def __init__(self, argv: Sequence[str], stderr: str) -> None:
        self.argv = tuple(argv)
        self.stderr = stderr
        super().__init__(
            f"the history could not be read: {shlex.join(self.argv)} "
            f"failed with {stderr}"
        )


class UnsupportedPlatformError(MeasurementRefusedError):
    pass


class UnreadableRecordError(MeasurementRefusedError):
    pass


class KeptRecordExistsError(MeasurementRefusedError):
    pass


class InvalidSettingsError(MeasurementRefusedError):
    pass


class SessionInProgressError(MeasurementRefusedError):
    pass


class BuildLeftRunningError(MeasurementRefusedError):
    def __init__(self, pgid: int) -> None:
        self.pgid = pgid
        super().__init__(
            f"the build's process group {pgid} outlived its reap; stop it "
            f"with `kill -KILL -{pgid}`"
        )


class OrphanedBuildError(MeasurementRefusedError):
    def __init__(
        self,
        pgid: int,
        members: Sequence[tuple[int, str]],
        remembered_at: Path,
    ) -> None:
        listed = "\n".join(f"  {pid} {command}" for pid, command in members)
        super().__init__(
            f"an earlier measure:build's build may still be running as "
            f"process group {pgid}:\n{listed}\n"
            f"if these are an earlier measure:build's mise, cargo or "
            f"nextest, stop them with `kill -TERM -{pgid}` (then "
            f"`kill -KILL -{pgid}` for any that survive) and retry; "
            f"if none is, delete {remembered_at}"
        )


class UnmeasurableDirectoryError(MeasurementRefusedError):
    pass
