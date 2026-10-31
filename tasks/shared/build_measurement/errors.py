class MeasurementRefusedError(RuntimeError):
    """The harness cannot measure; no record is written."""


class TaskGraphError(MeasurementRefusedError):
    pass
