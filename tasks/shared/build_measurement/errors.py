import shlex
from collections.abc import Sequence


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
