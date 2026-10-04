"""npm audit advisories, and the dated ignores that suppress them."""

import dataclasses
import datetime as dt
import json
import re
import tomllib

BLOCKING_SEVERITIES = frozenset({"high", "critical"})

_REVIEW_BY = re.compile(r"review-by:\s*(\d{4}-\d{2}-\d{2})")


class IgnoreListError(Exception): ...


@dataclasses.dataclass(frozen=True)
class Advisory:
    ghsa_id: str
    title: str
    severity: str
    package: str


@dataclasses.dataclass(frozen=True)
class AdvisoryIgnore:
    ghsa_id: str
    reason: str
    review_by: dt.date

    def suppresses(self, advisory: Advisory, today: dt.date) -> bool:
        return self.ghsa_id == advisory.ghsa_id and not self.has_lapsed(today)

    def has_lapsed(self, today: dt.date) -> bool:
        return self.review_by < today


def parse_report(text: str) -> list[Advisory]:
    """Read the advisories out of `npm audit --json` output.

    A package listed only because it depends on a vulnerable one names that
    package rather than an advisory, so it contributes nothing here.
    """
    advisories: dict[str, Advisory] = {}
    for vulnerability in json.loads(text)["vulnerabilities"].values():
        for via in vulnerability["via"]:
            if isinstance(via, dict):
                advisory = Advisory(
                    ghsa_id=via["url"].rsplit("/", 1)[-1],
                    title=via["title"],
                    severity=via["severity"],
                    package=via["name"],
                )
                advisories.setdefault(advisory.ghsa_id, advisory)
    return list(advisories.values())


def blocking_advisories(
    advisories: list[Advisory],
    ignores: list[AdvisoryIgnore],
    today: dt.date,
) -> list[Advisory]:
    return [
        advisory
        for advisory in advisories
        if advisory.severity in BLOCKING_SEVERITIES
        and not any(ignore.suppresses(advisory, today) for ignore in ignores)
    ]


def lapsed_ignores(
    ignores: list[AdvisoryIgnore], today: dt.date
) -> list[AdvisoryIgnore]:
    return [ignore for ignore in ignores if ignore.has_lapsed(today)]


def parse_ignores(text: str) -> list[AdvisoryIgnore]:
    return [
        _parse_ignore(entry) for entry in tomllib.loads(text).get("ignore", [])
    ]


def _parse_ignore(entry: dict[str, str]) -> AdvisoryIgnore:
    ghsa_id = entry["id"]
    reason = entry.get("reason", "").strip()
    if not reason:
        raise IgnoreListError(f"advisory ignore {ghsa_id} has no reason")
    review_by = _REVIEW_BY.search(reason)
    if review_by is None:
        raise IgnoreListError(
            f"advisory ignore {ghsa_id} has no `review-by: YYYY-MM-DD` in its "
            "reason"
        )
    return AdvisoryIgnore(
        ghsa_id, reason, dt.date.fromisoformat(review_by.group(1))
    )
