import dataclasses
import json
from typing import Self


@dataclasses.dataclass(frozen=True)
class NextestListing:
    """What ``cargo nextest list --message-format json`` says a filter selects.

    A binary is selected when any of its tests matches the filter.
    """

    binaries: frozenset[str]

    @classmethod
    def parse(cls, listing: str) -> Self:
        return cls(
            frozenset(
                binary
                for binary, suite in json.loads(listing)["rust-suites"].items()
                if any(
                    case["filter-match"]["status"] == "matches"
                    for case in suite["testcases"].values()
                )
            )
        )

    @property
    def selects_nothing(self) -> bool:
        return not self.binaries
