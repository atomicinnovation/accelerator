"""Overall health of the supervised dev stack and its exit-code mapping."""

import enum


class Health(enum.Enum):
    """Overall health of the supervised stack derived from watcher states."""

    HEALTHY = "healthy"
    PARTIAL = "partial"
    DOWN = "down"


def evaluate_health(statuses: dict[str, str]) -> Health:
    """Map a circus ``statuses`` dict to overall stack health.

    Pure status->enum map: ``HEALTHY`` when both ``server`` and ``frontend`` are
    ``active``, ``PARTIAL`` when exactly one is, ``DOWN`` when neither is
    (including missing keys). It cannot distinguish "frontend not yet started"
    from "frontend died" — that startup-window judgement lives in the
    orchestrator, not here.
    """
    server_active = statuses.get("server") == "active"
    frontend_active = statuses.get("frontend") == "active"
    if server_active and frontend_active:
        return Health.HEALTHY
    if server_active or frontend_active:
        return Health.PARTIAL
    return Health.DOWN


def status_exit_code(health: Health) -> int:
    """Map stack health to the contractual ``dev:status`` exit code."""
    return {Health.HEALTHY: 0, Health.PARTIAL: 3, Health.DOWN: 4}[health]
