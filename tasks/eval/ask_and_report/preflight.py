"""Prove, with the real CLI and no model, that each scenario's setup holds."""

from collections.abc import Mapping

from tasks.eval.ask_and_report.scenarios import Gate


def gate_of(verdict: Mapping[str, object]) -> Gate:
    """Name the scenario gate a ``topic consultable`` verdict reaches."""
    if verdict["consultable"]:
        return "open"
    match verdict["gate"]:
        case "status" | "freshness" as gate:
            return gate
        case "conducted" if verdict["unanswered"]:
            return "conducted"
        case "conducted" if verdict["held_back"]:
            return "held_back"
    raise ValueError(f"no scenario gate reads this verdict: {dict(verdict)}")
