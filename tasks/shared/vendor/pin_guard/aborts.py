"""The guard run's one abort type, raised before any issue is written."""


class GuardAbortError(Exception):
    """A local input or pin inconsistency that stops the run outright."""
