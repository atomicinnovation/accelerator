"""The criteria a scenario's turn declares, and how each judges the turn.

Every criterion reads the turn's main-conversation text, its snapshots and
its events, and matches fixed lines through ``formats``. A judgement
criterion is one whose outcome rests on the model's reading of the corpus,
which a scenario must therefore run three times.
"""

import fnmatch
import json
import re
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, ClassVar, Literal, get_args

from tasks.eval.ask_and_report import formats
from tasks.eval.ask_and_report.projects import SET, SLUG
from tasks.eval.ask_and_report.turns import (
    Snapshot,
    TurnRecord,
    changed,
    main_text,
)
from tasks.eval.documents.rule import normalised_url
from tasks.eval.documents.sources import parse_sources

type CoverageValue = Literal["answered", "partial", "unanswered"]
type Offered = Literal["accept", "pick", "proposal", "reject", "none"]

SLOT_LINE = "- {source-metadata}"
LISTED_REPORTS = "- **Reports**: `reports/`"
FIELDS_ADVANCED = ("last_updated", "last_updated_by")


@dataclass(frozen=True)
class Judging:
    """What a judge needs beyond the turn: the project, verb and validator."""

    root: Path
    verb: str
    validate: Callable[[Path], str | None]


def _in_set(path: str) -> str:
    return f"{SET.as_posix()}/{path}"


def _set_relative(paths: Sequence[str]) -> list[str]:
    prefix = f"{SET.as_posix()}/"
    return [p.removeprefix(prefix) for p in paths if p.startswith(prefix)]


def _text(snapshot: Snapshot, path: str) -> str | None:
    state = snapshot.get(_in_set(path))
    return state.text if state else None


def _frontmatter(document: str) -> dict[str, Any]:
    _, _, rest = document.partition("---\n")
    block, _, _ = rest.partition("\n---")
    fields: dict[str, Any] = {}
    for line in block.splitlines():
        key, separator, value = line.partition(": ")
        if separator:
            try:
                fields[key] = json.loads(value)
            except json.JSONDecodeError:
                fields[key] = value
    return fields


def _sections(document: str) -> dict[str, str]:
    found = re.split(r"^## (.+)$", document, flags=re.MULTILINE)
    return {
        heading.strip(): body
        for heading, body in zip(found[1::2], found[2::2], strict=True)
    }


def _block(lines: Sequence[str], header: str) -> list[str] | None:
    if header not in lines:
        return None
    items = []
    for line in lines[lines.index(header) + 1 :]:
        if not line.startswith("- "):
            break
        items.append(line)
    return items


def _proposal_items(text: str) -> list[str] | None:
    lines = formats.lines(text)
    for at, line in enumerate(lines):
        if formats.matches(formats.PROPOSED_ROUND, line):
            return [
                item
                for item in lines[at + 1 :]
                if formats.matches(formats.PROPOSED_ITEM, item)
            ]
    return None


def _one_of[T: str](value: object, allowed: tuple[T, ...], what: str) -> T:
    for option in allowed:
        if value == option:
            return option
    raise ValueError(f"{value!r} is no {what}")


def _expected(expected: object, got: object) -> list[str]:
    return [] if expected == got else [f"expected {expected!r}, got {got!r}"]


def _strings(value: object, what: str) -> tuple[str, ...]:
    if not isinstance(value, list) or not all(
        isinstance(item, str) for item in value
    ):
        raise TypeError(f"{what} must be a list of strings, not {value!r}")
    return tuple(value)


def _only(data: Mapping[str, object], keys: set[str], what: str) -> None:
    unknown = set(data) - keys
    if unknown:
        raise ValueError(f"a {what} cannot declare {sorted(unknown)}")


@dataclass(frozen=True)
class Refusal:
    is_judgement: ClassVar[bool] = False
    gate: formats.RefusalGate
    names: tuple[str, ...] = ()
    omits: tuple[str, ...] = ()

    @classmethod
    def parse(cls, value: Mapping[str, Any]) -> Refusal:
        _only(value, {"gate", "names", "omits"}, "refusal")
        return cls(
            gate=_one_of(value["gate"], tuple(formats.REFUSALS), "gate"),
            names=_strings(value.get("names", []), "names"),
            omits=_strings(value.get("omits", []), "omits"),
        )

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        lines = formats.lines(turn.text)
        said = "\n".join(lines)
        template = formats.REFUSALS[self.gate]
        known = {"verb": judging.verb, "slug": SLUG}
        failures = []
        if not any(formats.matches(template, ln, **known) for ln in lines):
            failures.append(f"expected the {self.gate} refusal, got {said!r}")
        failures += [
            f"expected {name!r} in the refusal"
            for name in self.names
            if formats.normalised(name) not in said
        ]
        failures += [
            f"expected no {name!r} in the refusal"
            for name in self.omits
            if formats.normalised(name) in said
        ]
        if written := changed(turn.before, turn.after):
            failures.append(f"expected no change, got {written}")
        return failures


@dataclass(frozen=True)
class Coverage:
    is_judgement: ClassVar[bool] = True
    value: CoverageValue

    @classmethod
    def parse(cls, value: object) -> Coverage:
        return cls(
            _one_of(value, get_args(CoverageValue.__value__), "coverage value")
        )

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        stated = next(
            (
                found["answered_partial_unanswered"]
                for line in formats.lines(turn.text)
                if (found := formats.matches(formats.COVERAGE, line))
            ),
            None,
        )
        return _expected(self.value, stated)


@dataclass(frozen=True)
class Covered:
    is_judgement: ClassVar[bool] = True
    keywords: tuple[str, ...]

    @classmethod
    def parse(cls, value: object) -> Covered:
        return cls(_strings(value, "covered"))

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        items = _block(formats.lines(turn.text), formats.COVERED)
        if items is None:
            return ["expected a Covered: block, got none"]
        return [
            f"expected {keyword!r} in a Covered: item, got {items}"
            for keyword in self.keywords
            if not any(keyword.casefold() in i.casefold() for i in items)
        ]


@dataclass(frozen=True)
class Gap:
    keyword: str
    tier: str | None = None


@dataclass(frozen=True)
class Gaps:
    is_judgement: ClassVar[bool] = True
    gaps: tuple[Gap, ...]

    @classmethod
    def parse(cls, value: list[Mapping[str, str]]) -> Gaps:
        for gap in value:
            _only(gap, {"keyword", "tier"}, "gap")
        return cls(tuple(Gap(g["keyword"], g.get("tier")) for g in value))

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        items = _block(formats.lines(turn.text), formats.GAPS)
        if items is None:
            return ["expected a Gaps: block, got none"]
        failures = [
            f"expected a filled Gaps: item, got {item!r}"
            for item in items
            if "…" in item or "or none" in item
        ]
        for gap in self.gaps:
            matching = [
                i for i in items if gap.keyword.casefold() in i.casefold()
            ]
            if not matching:
                failures.append(f"expected {gap.keyword!r} in a Gaps: item")
            elif gap.tier and not any(gap.tier in i for i in matching):
                failures.append(
                    f"expected {gap.tier!r} beside {gap.keyword!r}, "
                    f"got {matching}"
                )
        return failures


@dataclass(frozen=True)
class Equivalents:
    is_judgement: ClassVar[bool] = True
    paths: frozenset[str]

    @classmethod
    def parse(cls, value: object) -> Equivalents:
        return cls(frozenset(_strings(value, "equivalents")))

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        lines = formats.lines(turn.text)
        listed: set[str] = set()
        if formats.EQUIVALENT_REPORTS in lines:
            after = lines[lines.index(formats.EQUIVALENT_REPORTS) + 1 :]
            for line in after:
                found = formats.matches(formats.EQUIVALENT_REPORT, line)
                if not found:
                    break
                listed.add(f"reports/{found['file']}.md")
        return _expected(sorted(self.paths), sorted(listed))


def _offer_of(line: str) -> formats.Offer | None:
    return next(
        (
            offer
            for offer, prompt in formats.PROMPTS.items()
            if formats.matches(prompt, line, slug=SLUG)
        ),
        None,
    )


@dataclass(frozen=True)
class Offers:
    is_judgement: ClassVar[bool] = True
    offer: Offered

    @classmethod
    def parse(cls, value: object) -> Offers:
        return cls(_one_of(value, get_args(Offered.__value__), "offer"))

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        lines = formats.lines(turn.text)
        final = _offer_of(lines[-1]) if lines else None
        proposed = next(
            (
                at
                for at, line in enumerate(lines)
                if formats.matches(formats.PROPOSED_ROUND, line)
            ),
            None,
        )
        if self.offer == "none":
            failures = _expected(None, final)
            if proposed is not None:
                failures.append("expected no proposal, got one")
            return failures
        failures = _expected(self.offer, final)
        if proposed is not None and any(
            _offer_of(line) for line in lines[:proposed]
        ):
            failures.append("expected no prompt before the proposal")
        return failures


@dataclass(frozen=True)
class Writes:
    is_judgement: ClassVar[bool] = False
    paths: tuple[str, ...]

    @classmethod
    def parse(cls, value: object) -> Writes:
        return cls(_strings(value, "writes"))

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        written = changed(turn.before, turn.after)
        in_set = _set_relative(written)
        failures = _expected(sorted(self.paths), sorted(in_set))
        if len(in_set) != len(written):
            failures.append(f"expected changes only in the set, got {written}")
        return failures


@dataclass(frozen=True)
class Unchanged:
    is_judgement: ClassVar[bool] = False
    patterns: tuple[str, ...]

    @classmethod
    def parse(cls, value: object) -> Unchanged:
        return cls(_strings(value, "unchanged"))

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        differing = [
            path
            for path in _set_relative(changed(turn.initial, turn.after))
            if any(fnmatch.fnmatch(path, p) for p in self.patterns)
        ]
        return [f"expected {path} as it began" for path in differing]


@dataclass(frozen=True)
class Report:
    is_judgement: ClassVar[bool] = False
    path: str
    fields: Mapping[str, object] = field(default_factory=dict)
    absent: tuple[str, ...] = ()
    sections: tuple[str, ...] = ()
    no_sections: tuple[str, ...] = ()
    section_mentions: Mapping[str, tuple[str, ...]] = field(
        default_factory=dict
    )
    kept: tuple[str, ...] = ()

    @classmethod
    def parse(cls, value: Mapping[str, Any]) -> Report:
        _only(
            value,
            {
                "path",
                "fields",
                "absent",
                "sections",
                "no_sections",
                "section_mentions",
                "kept",
            },
            "report",
        )
        return cls(
            path=value["path"],
            fields=dict(value.get("fields", {})),
            absent=_strings(value.get("absent", []), "absent"),
            sections=_strings(value.get("sections", []), "sections"),
            no_sections=_strings(value.get("no_sections", []), "no_sections"),
            section_mentions={
                section: _strings(words, "section_mentions")
                for section, words in value.get("section_mentions", {}).items()
            },
            kept=_strings(value.get("kept", []), "kept"),
        )

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        document = _text(turn.after, self.path)
        if document is None:
            return [f"expected {self.path}, got no such file"]
        written = _frontmatter(document)
        sections = _sections(document)
        failures = [
            f"expected {key} {value!r}, got {written.get(key)!r}"
            for key, value in self.fields.items()
            if written.get(key) != value
        ]
        failures += [f"expected no {k}" for k in self.absent if k in written]
        failures += [
            f"expected a {s} section"
            for s in self.sections
            if s not in sections
        ]
        failures += [
            f"expected no {s} section"
            for s in self.no_sections
            if s in sections
        ]
        failures += [
            f"expected {word!r} in {section}"
            for section, words in self.section_mentions.items()
            for word in words
            if word.casefold() not in sections.get(section, "").casefold()
        ]
        began = _frontmatter(_text(turn.initial, self.path) or "")
        failures += [
            f"expected {key} kept as {began.get(key)!r}, "
            f"got {written.get(key)!r}"
            for key in self.kept
            if written.get(key) != began.get(key)
        ]
        if invalid := judging.validate(judging.root / SET / self.path):
            failures.append(f"expected {self.path} to validate, got {invalid}")
        return failures


@dataclass(frozen=True)
class SourceEntry:
    marker: str
    tier: str
    block: tuple[tuple[str, str], ...] | None = None


@dataclass(frozen=True)
class Sources:
    is_judgement: ClassVar[bool] = True
    path: str
    entries: tuple[SourceEntry, ...]

    @classmethod
    def parse(cls, value: list[Mapping[str, Any]], path: str) -> Sources:
        for entry in value:
            _only(entry, {"marker", "tier", "block"}, "source")
        return cls(
            path=path,
            entries=tuple(
                SourceEntry(
                    marker=entry["marker"],
                    tier=entry["tier"],
                    block=(
                        tuple((k, v) for k, v in entry["block"])
                        if "block" in entry
                        else None
                    ),
                )
                for entry in value
            ),
        )

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        document = _text(turn.after, self.path) or ""
        if SLOT_LINE in document:
            return ["expected no slot line, got one"]
        try:
            written = parse_sources(document)
        except ValueError as unreadable:
            return [f"expected readable Sources, got {unreadable}"]
        failures: list[str] = []
        for expected in self.entries:
            selected = [c for c in written if expected.marker in c.render()]
            if len(selected) != 1:
                failures.append(
                    f"expected one {expected.marker} entry, got {len(selected)}"
                )
                continue
            failures += _expected(expected.tier, str(selected[0].tier))
            if expected.block is not None:
                failures += _expected(expected.block, selected[0].block)
        cited = {
            normalised_url(citation.url)
            for path in _set_relative(list(turn.after))
            if path.startswith("findings/") and path.endswith(".md")
            for citation in parse_sources(_text(turn.after, path) or "")
        }
        failures += [
            f"expected only sources the findings cite, got {c.url}"
            for c in written
            if normalised_url(c.url) not in cited
        ]
        return failures


@dataclass(frozen=True)
class Manifest:
    is_judgement: ClassVar[bool] = False
    fields: Mapping[str, object]
    lists_reports: bool

    @classmethod
    def parse(cls, value: Mapping[str, Any]) -> Manifest:
        _only(value, {"fields", "lists_reports"}, "manifest")
        return cls(dict(value["fields"]), bool(value["lists_reports"]))

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        document = _text(turn.after, "manifest.md") or ""
        written = _frontmatter(document)
        failures = [
            f"expected {key} {value!r}, got {written.get(key)!r}"
            for key, value in self.fields.items()
            if written.get(key) != value
        ]
        listed = LISTED_REPORTS in _sections(document).get("Set", "")
        return failures + _expected(self.lists_reports, listed)


@dataclass(frozen=True)
class Proposal:
    is_judgement: ClassVar[bool] = True
    max_items: int | None = None
    profiles: frozenset[str] | None = None
    each_mentions: tuple[str, ...] = ()

    @classmethod
    def parse(cls, value: Mapping[str, Any]) -> Proposal:
        _only(value, {"max_items", "profiles", "each_mentions"}, "proposal")
        profiles = value.get("profiles")
        return cls(
            max_items=value.get("max_items"),
            profiles=(
                frozenset(_strings(profiles, "profiles")) if profiles else None
            ),
            each_mentions=_strings(value.get("each_mentions", []), "mentions"),
        )

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        items = _proposal_items(turn.text)
        if not items:
            return ["expected a proposal, got none"]
        failures = []
        if self.max_items is not None and len(items) > self.max_items:
            failures.append(f"expected {self.max_items} items, got {items}")
        for item in items:
            found = formats.matches(formats.PROPOSED_ITEM, item)
            named = set(found["profile"].split(", ")) if found else set()
            if self.profiles is not None and not named <= self.profiles:
                failures.append(
                    f"expected profiles from {self.profiles}: {item}"
                )
            if self.each_mentions and not any(
                word.casefold() in item.casefold()
                for word in self.each_mentions
            ):
                failures.append(f"expected one of {self.each_mentions}: {item}")
        return failures


def _masked(document: str) -> str:
    return "\n".join(
        line
        for line in document.splitlines()
        if not line.startswith(tuple(f"{key}:" for key in FIELDS_ADVANCED))
    )


@dataclass(frozen=True)
class AppendedRound:
    is_judgement: ClassVar[bool] = False
    number: int

    @classmethod
    def parse(cls, value: object) -> AppendedRound:
        if not isinstance(value, int) or isinstance(value, bool):
            raise TypeError(f"appended_round must be a number, not {value!r}")
        return cls(value)

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        before = _text(turn.before, "outline.md") or ""
        after = _text(turn.after, "outline.md") or ""
        if not _masked(after).startswith(_masked(before)):
            return ["expected the earlier rounds unchanged"]
        appended = _masked(after)[len(_masked(before)) :].strip("\n")
        headings = re.findall(r"^## .*$", appended, flags=re.MULTILINE)
        failures = _expected([f"## Round {self.number}"], headings)
        if not appended.startswith(f"## Round {self.number}"):
            failures.append("expected the round to open the appended text")
        failures += [
            f"expected {key} advanced"
            for key in FIELDS_ADVANCED
            if _frontmatter(after).get(key) == _frontmatter(before).get(key)
        ]
        proposed = (
            _proposal_items(main_text(turn.earlier[-1]))
            if turn.earlier
            else None
        )
        items = [
            formats.normalised(line)
            for line in appended.splitlines()
            if line.startswith("- [ ]")
        ]
        return failures + _expected(proposed, items)


@dataclass(frozen=True)
class NextSteps:
    is_judgement: ClassVar[bool] = False
    present: bool

    @classmethod
    def parse(cls, value: object) -> NextSteps:
        if not isinstance(value, bool):
            raise TypeError(f"next_steps must be true or false, not {value!r}")
        return cls(value)

    def judge(self, turn: TurnRecord, judging: Judging) -> list[str]:
        named = any(
            formats.matches(
                formats.APPENDED_ROUND, line, slug=SLUG, verb=judging.verb
            )
            for line in formats.lines(turn.text)
        )
        return _expected(self.present, named)


type Criterion = (
    Refusal
    | Coverage
    | Covered
    | Gaps
    | Equivalents
    | Offers
    | Writes
    | Unchanged
    | Report
    | Sources
    | Manifest
    | Proposal
    | AppendedRound
    | NextSteps
)

CRITERIA: dict[str, type[Criterion]] = {
    "refusal": Refusal,
    "coverage": Coverage,
    "covered": Covered,
    "gaps": Gaps,
    "equivalents": Equivalents,
    "offers": Offers,
    "writes": Writes,
    "unchanged": Unchanged,
    "report": Report,
    "sources": Sources,
    "manifest": Manifest,
    "proposal": Proposal,
    "appended_round": AppendedRound,
    "next_steps": NextSteps,
}


_PARSERS: dict[str, Callable[[Any], Criterion]] = {
    "refusal": Refusal.parse,
    "coverage": Coverage.parse,
    "covered": Covered.parse,
    "gaps": Gaps.parse,
    "equivalents": Equivalents.parse,
    "offers": Offers.parse,
    "writes": Writes.parse,
    "unchanged": Unchanged.parse,
    "report": Report.parse,
    "manifest": Manifest.parse,
    "proposal": Proposal.parse,
    "appended_round": AppendedRound.parse,
    "next_steps": NextSteps.parse,
}


def parse_criteria(expect: Mapping[str, Any]) -> tuple[Criterion, ...]:
    """Read a turn's ``expect`` block, each key into its criterion.

    ``sources`` judges the report ``report`` names, so it needs one beside it.
    """
    _only(expect, set(CRITERIA), "turn's expectations")
    criteria: list[Criterion] = []
    for key, value in expect.items():
        if key == "sources":
            if "report" not in expect:
                raise ValueError("sources needs a report beside it")
            criteria.append(Sources.parse(value, expect["report"]["path"]))
        else:
            criteria.append(_PARSERS[key](value))
    return tuple(criteria)
