"""The Sources section grammar: an entry line, its tier text and its block."""

import re
from dataclasses import dataclass
from typing import override

ENTRY = re.compile(
    r"^- \[(?P<title>.*)\]\((?P<url>[^)\s]*)\) — "
    r"(?P<tier>tier-[123](?: \([^)]*\))?)(?: — (?P<venue>.+))?$"
)
BLOCK_LINE = re.compile(r"^  - (?P<key>[a-z0-9_]+): (?P<value>.*)$")
ALSO = "also "
NONE_FOUND = "None found."


@dataclass(frozen=True)
class TierText:
    """A base tier, any marks, and the other tiers an ``also`` list names."""

    base: int
    marks: tuple[str, ...] = ()
    also: tuple[int, ...] = ()

    @classmethod
    def parse(cls, text: str) -> TierText:
        found = re.fullmatch(r"tier-([123])(?: \((.*)\))?", text)
        if found is None:
            raise ValueError(f"not tier text: {text!r}")
        parts: list[str] = found.group(2).split("; ") if found.group(2) else []
        marks = tuple(part for part in parts if not part.startswith(ALSO))
        also = tuple(
            int(tier.removeprefix("tier-"))
            for part in parts
            if part.startswith(ALSO)
            for tier in part.removeprefix(ALSO).split(", ")
        )
        return cls(base=int(found.group(1)), marks=marks, also=also)

    @property
    def tiers(self) -> frozenset[int]:
        return frozenset((self.base, *self.also))

    @override
    def __str__(self) -> str:
        parts = list(self.marks)
        if self.also:
            parts.append(ALSO + ", ".join(f"tier-{t}" for t in self.also))
        suffix = f" ({'; '.join(parts)})" if parts else ""
        return f"tier-{self.base}{suffix}"


@dataclass(frozen=True)
class Citation:
    """One Sources entry and the metadata block under it."""

    title: str
    url: str
    tier: TierText
    venue: str | None
    block: tuple[tuple[str, str], ...] = ()

    def render(self) -> str:
        venue = f" — {self.venue}" if self.venue is not None else ""
        lines = [f"- [{self.title}]({self.url}) — {self.tier}{venue}"]
        lines += [f"  - {key}: {value}" for key, value in self.block]
        return "\n".join(lines) + "\n"


def sources_section(document: str) -> str:
    found = re.search(
        r"^## Sources\n(.*?)(?=^## |\Z)", document, re.MULTILINE | re.DOTALL
    )
    return found.group(1) if found else ""


def parse_sources(document: str) -> list[Citation]:
    """Every citation in a document's Sources section, in written order."""
    citations: list[Citation] = []
    for line in sources_section(document).splitlines():
        if entry := ENTRY.match(line):
            citations.append(
                Citation(
                    title=entry["title"],
                    url=entry["url"],
                    tier=TierText.parse(entry["tier"]),
                    venue=entry["venue"],
                )
            )
        elif (block_line := BLOCK_LINE.match(line)) and citations:
            last = citations[-1]
            citations[-1] = Citation(
                title=last.title,
                url=last.url,
                tier=last.tier,
                venue=last.venue,
                block=(*last.block, (block_line["key"], block_line["value"])),
            )
        elif line.strip() not in {"", NONE_FOUND}:
            raise ValueError(f"not a Sources entry or block line: {line!r}")
    return citations
