"""npm audit reports, read as advisories and filtered by accepted exceptions."""

import json
import tomllib
from dataclasses import dataclass
from pathlib import Path

BLOCKING_SEVERITIES = frozenset({"high", "critical"})


class UnreadableAuditReportError(ValueError):
    pass


@dataclass(frozen=True)
class Advisory:
    id: str
    package: str
    severity: str
    title: str

    @property
    def blocks(self) -> bool:
        return self.severity in BLOCKING_SEVERITIES

    def describe(self) -> str:
        return f"{self.id} ({self.severity}) in {self.package}: {self.title}"


def advisories_in(report_json: str) -> list[Advisory]:
    """Each advisory npm attributes directly to a package.

    Dependants of a vulnerable package name it as a plain string in their
    `via` list; only the advisory objects carry an id, so those are the
    findings and the strings are their transitive echo.
    """
    try:
        vulnerabilities = json.loads(report_json)["vulnerabilities"]
    except (json.JSONDecodeError, KeyError, TypeError) as e:
        raise UnreadableAuditReportError(report_json.strip()) from e
    return [
        Advisory(
            id=via["url"].rsplit("/", 1)[-1],
            package=package,
            severity=via["severity"],
            title=via["title"],
        )
        for package, vulnerability in vulnerabilities.items()
        for via in vulnerability["via"]
        if isinstance(via, dict)
    ]


def accepted_advisory_ids(exceptions_file: Path) -> frozenset[str]:
    exceptions = tomllib.loads(exceptions_file.read_text())
    return frozenset(e["id"] for e in exceptions.get("exception", []))


def blocking_advisories(
    advisories: list[Advisory], accepted: frozenset[str]
) -> list[Advisory]:
    return [a for a in advisories if a.blocks and a.id not in accepted]
