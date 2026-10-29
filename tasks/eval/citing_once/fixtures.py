"""Eval fixtures: compositions of notes or findings, and their criteria."""

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal

from tasks.eval.citing_once.sources import Citation, parse_sources

type Carrier = Literal["composer", "synthesise"]


@dataclass(frozen=True)
class Expectation:
    """What one entry a criterion selects must show; an empty one shows."""

    title: str | None = None
    url: str | None = None
    tier: str | None = None
    venue: str | None = None
    block: tuple[str, ...] | None = None

    def met_by(self, entry: Citation) -> bool:
        lines = tuple(f"{key}: {value}" for key, value in entry.block)
        return all(
            wanted is None or wanted == actual
            for wanted, actual in (
                (self.title, entry.title),
                (self.url, entry.url),
                (self.tier, str(entry.tier)),
                (self.venue, entry.venue),
                (self.block, lines),
            )
        )


@dataclass(frozen=True)
class Placed:
    document: str
    citation: Citation


@dataclass(frozen=True)
class Criterion:
    id: str
    statement: str
    markers: tuple[str, ...]
    citations: tuple[Placed, ...]
    expect: tuple[Expectation, ...]

    def selects(self, entry: Citation) -> bool:
        searched = " ".join(
            [entry.url, *(value for key, value in entry.block if key == "doi")]
        ).lower()
        return any(marker.lower() in searched for marker in self.markers)

    @classmethod
    def from_json(cls, data: dict[str, Any]) -> Criterion:
        identifier = data["id"]
        return cls(
            id=identifier,
            statement=data["statement"],
            markers=tuple(
                data.get(
                    "markers", [f"{identifier}.example", f"10.{identifier}/"]
                )
            ),
            citations=tuple(
                Placed(
                    document=placed["in"],
                    citation=_one_citation(placed["entry"]),
                )
                for placed in data["citations"]
            ),
            expect=tuple(_expectation(wanted) for wanted in data["expect"]),
        )


def _expectation(wanted: dict[str, Any]) -> Expectation:
    unknown = set(wanted) - {"title", "url", "tier", "venue", "block"}
    if unknown:
        raise ValueError(f"an expectation cannot check {sorted(unknown)}")
    return Expectation(
        title=wanted.get("title"),
        url=wanted.get("url"),
        tier=wanted.get("tier"),
        venue=wanted.get("venue"),
        block=tuple(wanted["block"]) if "block" in wanted else None,
    )


def _one_citation(lines: list[str]) -> Citation:
    (citation,) = parse_sources("## Sources\n" + "\n".join(lines) + "\n")
    return citation


def _document_key(document: str) -> tuple[int | str, ...]:
    return tuple(
        int(part) if part.isdigit() else part for part in document.split("-")
    )


@dataclass(frozen=True)
class Composition:
    """What one session reads: its documents, in each order under test."""

    name: str
    carrier: Carrier
    times: int
    orders: tuple[tuple[str, ...], ...]
    criteria: tuple[Criterion, ...]

    @property
    def documents(self) -> tuple[str, ...]:
        found = {
            placed.document
            for criterion in self.criteria
            for placed in criterion.citations
        }
        return tuple(sorted(found, key=_document_key))

    @property
    def session_count(self) -> int:
        return len(self.orders) * self.times

    def citations_in(self, document: str) -> list[Citation]:
        return [
            placed.citation
            for criterion in self.criteria
            for placed in criterion.citations
            if placed.document == document
        ]

    @classmethod
    def from_json(cls, data: dict[str, Any]) -> Composition:
        criteria = tuple(Criterion.from_json(c) for c in data["criteria"])
        composition = cls(
            name=data["name"],
            carrier=data["carrier"],
            times=data["times"],
            orders=tuple(tuple(order) for order in data.get("orders", [])),
            criteria=criteria,
        )
        if composition.orders:
            return composition
        return cls(
            name=composition.name,
            carrier=composition.carrier,
            times=composition.times,
            orders=(composition.documents,),
            criteria=criteria,
        )


@dataclass(frozen=True)
class Fixture:
    family: str
    title: str
    compositions: tuple[Composition, ...]


def load_fixtures(directory: Path) -> list[Fixture]:
    fixtures = []
    for path in sorted(directory.glob("*.json")):
        data = json.loads(path.read_text())
        fixtures.append(
            Fixture(
                family=data["family"],
                title=data["title"],
                compositions=tuple(
                    Composition.from_json(c) for c in data["compositions"]
                ),
            )
        )
    return fixtures
