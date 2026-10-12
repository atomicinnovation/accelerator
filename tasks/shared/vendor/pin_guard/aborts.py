"""The guard run's one abort type, raised before any issue is written."""


class GuardAbortError(Exception):
    pass
