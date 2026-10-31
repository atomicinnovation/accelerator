class MeasurementRefusedError(RuntimeError):
    """The harness cannot measure; no record is written."""


class TaskGraphError(MeasurementRefusedError):
    pass


class NoMeasurableCommitError(MeasurementRefusedError):
    pass


class HistoryError(MeasurementRefusedError):
    pass


class UnsupportedPlatformError(MeasurementRefusedError):
    pass
